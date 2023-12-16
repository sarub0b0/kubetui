mod log_collector;
mod log_streamer;
mod pod_watcher;

use std::collections::BTreeMap;

use anyhow::{anyhow, bail, Result};
use async_trait::async_trait;
use crossbeam::channel::Sender;
use futures::future::join_all;
use k8s_openapi::api::{
    apps::v1::{DaemonSet, Deployment, ReplicaSet, StatefulSet},
    batch::v1::Job,
    core::v1::Service,
};
use kube::Api;
use tokio::task::{JoinError, JoinHandle};

use crate::{
    context::Namespace,
    event::{
        kubernetes::{
            client::KubeClient,
            pod::filter::{Filter, LabelSelector, RetrievableResource},
            worker::{AbortWorker, Worker},
            Kube,
        },
        Event,
    },
    logger,
};

pub use self::log_streamer::LogPrefixType;

use self::{
    log_collector::{LogBuffer, LogCollector},
    log_streamer::LogStreamerOptions,
    pod_watcher::{PodWatcher, PodWatcherFilter, PodWatcherSelector},
};

#[macro_export]
macro_rules! send_response {
    ($tx:expr, $msg:expr) => {
        use $crate::event::kubernetes::pod::LogMessage;

        $tx.send(LogMessage::Response($msg).into())
            .expect("Failed to send LogMessage::Response");
    };
}

#[derive(Debug, Clone)]
pub struct LogConfig {
    namespaces: Namespace,
    query: String,
    prefix_type: LogPrefixType,
}

impl LogConfig {
    pub fn new(query: String, namespaces: Namespace, prefix_type: LogPrefixType) -> Self {
        Self {
            namespaces,
            query,
            prefix_type,
        }
    }
}

#[derive(Debug)]
pub enum LogMessage {
    Request(LogConfig),
    Response(Result<Vec<String>>),
}

impl From<LogMessage> for Event {
    fn from(m: LogMessage) -> Event {
        Event::Kube(Kube::Log(m))
    }
}

#[derive(Clone)]
pub struct LogWorker {
    tx: Sender<Event>,
    client: KubeClient,
    config: LogConfig,
}

impl LogWorker {
    pub fn new(tx: Sender<Event>, client: KubeClient, config: LogConfig) -> Self {
        Self { tx, client, config }
    }

    async fn spawn_tasks(&self, filter: Filter) -> Result<LogHandle> {
        logger!(info, "log filter config: {}", filter);

        // watch per namespace
        let mut pod_watchers = Vec::new();

        let namespaces = self.config.namespaces.to_vec();

        let log_buffer = LogBuffer::default();

        for namespace in namespaces {
            // retrieve label selector
            let label_selector = if let Some(value) = &filter.label_selector {
                let retrieve_label_selector =
                    RetrieveLabelSelector::new(&self.client, &namespace, value);

                Some(retrieve_label_selector.retrieve().await?)
            } else {
                None
            };

            let pod_watcher = PodWatcher::new(
                self.tx.clone(),
                self.client.clone(),
                log_buffer.clone(),
                namespace,
            )
            .filter(PodWatcherFilter {
                pod: filter.pod.clone(),
                exclude_pod: filter.exclude_pod.clone(),
                container: filter.container.clone(),
                exclude_container: filter.exclude_container.clone(),
            })
            .selector(PodWatcherSelector {
                label_selector,
                field_selector: filter.field_selector.clone(),
            })
            .log_streamer_options(LogStreamerOptions {
                prefix_type: self.config.prefix_type,
                include_log: filter.include_log.clone(),
                exclude_log: filter.exclude_log.clone(),
            });

            pod_watchers.push(pod_watcher);
        }

        let mut handles: Vec<_> = pod_watchers.iter().map(PodWatcher::spawn).collect();

        // collector
        let collector_handle = LogCollector::new(self.tx.clone(), log_buffer.clone()).spawn();

        handles.push(collector_handle);

        // drop handles
        Ok(LogHandle::new(handles))
    }
}

#[async_trait]
impl AbortWorker for LogWorker {
    async fn run(&self) {
        match Filter::parse(&self.config.query) {
            Ok(filter) => {
                match self.spawn_tasks(filter).await {
                    Ok(mut handles) => {
                        handles.join().await;
                    }
                    Err(err) => {
                        logger!(error, "{}", err);
                        send_response!(self.tx, Err(anyhow!(err)));
                    }
                };
            }
            Err(err) => {
                logger!(error, "{}", err);

                let msg = indoc::formatdoc! {r#"
                       {err}
                       Invalid query.
                       You can display the help popup by entering "?" or "help" in the log query form.
                   "#,
                   err = err
                };

                send_response!(self.tx, Err(anyhow!(msg)));
            }
        }
    }
}

struct LogHandle {
    inner: Vec<JoinHandle<()>>,
}

impl LogHandle {
    fn new(handles: Vec<JoinHandle<()>>) -> Self {
        Self { inner: handles }
    }

    async fn join(&mut self) -> Vec<Result<(), JoinError>> {
        join_all(&mut self.inner).await
    }

    fn abort(&self) {
        self.inner.iter().for_each(JoinHandle::abort);
    }
}

impl Drop for LogHandle {
    fn drop(&mut self) {
        logger!(info, "abort log tasks.");
        self.abort();
    }
}

struct RetrieveLabelSelector<'a> {
    client: &'a KubeClient,
    namespace: &'a str,
    label_selector: &'a LabelSelector,
}

impl<'a> RetrieveLabelSelector<'a> {
    fn new(client: &'a KubeClient, namespace: &'a str, label_selector: &'a LabelSelector) -> Self {
        Self {
            client,
            namespace,
            label_selector,
        }
    }

    async fn retrieve(&self) -> Result<String> {
        match self.label_selector {
            LabelSelector::Resource(resource) => self.retrieve_from_resource(resource).await,
            LabelSelector::String(value) => Ok(value.to_string()),
        }
    }

    async fn retrieve_from_resource(&self, resource: &RetrievableResource) -> Result<String> {
        use RetrievableResource::*;

        match resource {
            DaemonSet(name) => self.retrieve_from_daemonset(name).await,
            Deployment(name) => self.retrieve_from_deployment(name).await,
            Job(name) => self.retrieve_from_job(name).await,
            ReplicaSet(name) => self.retrieve_from_replicaset(name).await,
            Service(name) => self.retrieve_from_service(name).await,
            StatefulSet(name) => self.retrieve_from_statefulset(name).await,
        }
    }

    async fn retrieve_from_daemonset(&self, name: &str) -> Result<String> {
        let api: Api<DaemonSet> = Api::namespaced(self.client.to_client(), self.namespace);

        let daemonset = api.get(name).await?;

        let Some(spec) = daemonset.spec else {
            bail!("daemonset.spec is none. ({})", name);
        };

        let Some(metadata) = spec.template.metadata else {
            bail!("daemonset.spec.template.metadata is none. ({})", name);
        };

        let Some(labels) = metadata.labels else {
            bail!(
                "daemonset.spec.template.metadata.labels is none. ({})",
                name
            );
        };

        Ok(Self::btreemap_to_comma_string(&labels))
    }

    async fn retrieve_from_deployment(&self, name: &str) -> Result<String> {
        let api: Api<Deployment> = Api::namespaced(self.client.to_client(), self.namespace);

        let deployment = api.get(name).await?;

        let Some(spec) = deployment.spec else {
            bail!("deployment.spec is none. ({})", name);
        };

        let Some(metadata) = spec.template.metadata else {
            bail!("deployment.spec.template.metadata is none. ({})", name);
        };

        let Some(labels) = metadata.labels else {
            bail!(
                "deployment.spec.template.metadata.labels is none. ({})",
                name
            );
        };

        Ok(Self::btreemap_to_comma_string(&labels))
    }

    async fn retrieve_from_job(&self, name: &str) -> Result<String> {
        let api: Api<Job> = Api::namespaced(self.client.to_client(), self.namespace);

        let job = api.get(name).await?;

        let Some(spec) = job.spec else {
            bail!("job.spec is none. ({})", name);
        };

        let Some(metadata) = spec.template.metadata else {
            bail!("job.spec.template.metadata is none. ({})", name);
        };

        let Some(labels) = metadata.labels else {
            bail!("job.spec.template.metadata.labels is none. ({})", name);
        };

        Ok(Self::btreemap_to_comma_string(&labels))
    }

    async fn retrieve_from_replicaset(&self, name: &str) -> Result<String> {
        let api: Api<ReplicaSet> = Api::namespaced(self.client.to_client(), self.namespace);

        let replicaset = api.get(name).await?;

        let Some(spec) = replicaset.spec else {
            bail!("replicaset.spec is none. ({})", name);
        };

        let Some(template) = spec.template else {
            bail!("replicaset.spec.template is none. ({})", name);
        };

        let Some(metadata) = template.metadata else {
            bail!("replicaset.spec.template.metadata is none. ({})", name);
        };

        let Some(labels) = metadata.labels else {
            bail!(
                "replicaset.spec.template.metadata.labels is none. ({})",
                name
            );
        };

        Ok(Self::btreemap_to_comma_string(&labels))
    }

    async fn retrieve_from_service(&self, name: &str) -> Result<String> {
        let api: Api<Service> = Api::namespaced(self.client.to_client(), self.namespace);

        let service = api.get(name).await?;

        let Some(spec) = service.spec else {
            bail!("service.spec is none. ({})", name);
        };

        let Some(selector) = spec.selector else {
            bail!("service.spec.selector is none. ({})", name);
        };

        Ok(Self::btreemap_to_comma_string(&selector))
    }

    async fn retrieve_from_statefulset(&self, name: &str) -> Result<String> {
        let api: Api<StatefulSet> = Api::namespaced(self.client.to_client(), self.namespace);

        let statefulset = api.get(name).await?;

        let Some(spec) = statefulset.spec else {
            bail!("statefulset.spec is none. ({})", name);
        };

        let Some(metadata) = spec.template.metadata else {
            bail!("statefulset.spec.template.metadata is none. ({})", name);
        };

        let Some(labels) = metadata.labels else {
            bail!(
                "statefulset.spec.template.metadata.labels is none. ({})",
                name
            );
        };

        Ok(Self::btreemap_to_comma_string(&labels))
    }

    fn btreemap_to_comma_string(map: &BTreeMap<String, String>) -> String {
        map.iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<String>>()
            .join(",")
    }
}
