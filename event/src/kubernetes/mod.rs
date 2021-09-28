mod api_resources;
mod client;
mod config;
mod event;
mod log;
mod metric_type;
mod pod;
mod v1_table;
mod worker;

use super::Event;
use api_resources::apis_list;
use client::KubeClient;
use config::get_config;
use futures::future::select_all;
use k8s_openapi::api::core::v1::Namespace;
use worker::Worker;

use std::{convert::TryFrom, panic, sync::atomic::AtomicBool, sync::Arc, time::Duration};

use crossbeam::channel::{Receiver, Sender};
use tokio::{
    runtime::Runtime,
    sync::RwLock,
    task::{self, JoinHandle},
};

use async_trait::async_trait;

use kube::{
    api::ListParams,
    config::{Config, KubeConfigOptions, Kubeconfig, NamedContext},
    Api, Client, ResourceExt,
};

use crate::{
    error::{Error, Result},
    kubernetes::{
        api_resources::ApiPollWorker, config::ConfigsPollWorker, event::EventPollWorker,
        log::LogWorkerBuilder, pod::PodPollWorker, worker::PollWorker,
    },
    panic_set_hook,
};

pub use kube;

#[derive(Debug, Default)]
pub struct KubeTable {
    header: Vec<String>,
    rows: Vec<Vec<String>>,
}

impl KubeTable {
    pub fn header(&self) -> &Vec<String> {
        &self.header
    }

    pub fn rows(&self) -> &Vec<Vec<String>> {
        &self.rows
    }

    pub fn push_row(&mut self, row: impl Into<Vec<String>>) {
        let row = row.into();

        debug_assert!(
            self.header.len() == row.len(),
            "Mismatch header({}) != row({})",
            self.header.len(),
            row.len()
        );

        self.rows.push(row);
    }

    pub fn update_rows(&mut self, rows: Vec<Vec<String>>) {
        if !rows.is_empty() {
            for row in rows.iter() {
                debug_assert!(
                    self.header.len() == row.len(),
                    "Mismatch header({}) != row({})",
                    self.header.len(),
                    row.len()
                );
            }
        }

        self.rows = rows;
    }
}

pub enum Kube {
    // apis
    GetAPIsRequest,
    GetAPIsResponse(Result<Vec<String>>),
    SetAPIsRequest(Vec<String>),
    APIsResults(Result<Vec<String>>),
    // Context
    GetContextsRequest,
    GetContextsResponse(Result<Vec<String>>),
    SetContext(String),
    GetCurrentContextRequest,
    GetCurrentContextResponse(String, String), // current_context, namespace
    // Event
    Event(Result<Vec<String>>),
    // Namespace
    GetNamespacesRequest,
    GetNamespacesResponse(Vec<String>),
    SetNamespaces(Vec<String>),
    // Pod Status
    Pod(Result<KubeTable>),
    // Pod Logs
    LogStreamRequest(String, String),
    LogStreamResponse(Result<Vec<String>>),
    // ConfigMap & Secret
    Configs(Result<KubeTable>),
    ConfigRequest(String, String, String), // namespace, kind, resource_name
    ConfigResponse(Result<Vec<String>>),
}

#[derive(Default, Debug)]
pub struct Handlers(Vec<JoinHandle<Result<()>>>);

impl Handlers {
    fn abort(&self) {
        self.0.iter().for_each(|j| j.abort());
    }
}

fn cluster_server_url(kubeconfig: &Kubeconfig, named_context: &NamedContext) -> Result<String> {
    let cluster_name = named_context.context.cluster.clone();

    let named_cluster = kubeconfig.clusters.iter().find(|n| n.name == cluster_name);

    Ok(named_cluster
        .cloned()
        .ok_or_else(|| Error::Raw("Failed to get cluster server URL".into()))?
        .cluster
        .server)
}

pub type Namespaces = Arc<RwLock<Vec<String>>>;
pub type ApiResources = Arc<RwLock<Vec<String>>>;

#[derive(Clone)]
pub enum WorkerResult {
    ChangedContext(String),
    Terminated,
}

async fn inner_kube_process(
    tx: Sender<Event>,
    rx: Receiver<Event>,
    is_terminated: Arc<AtomicBool>,
) -> Result<()> {
    let kubeconfig = Kubeconfig::read()?;

    let mut context: Option<String> = None;

    while !is_terminated.load(std::sync::atomic::Ordering::Relaxed) {
        let (kube_client, current_namespace, current_context) = if let Some(context) = &context {
            let named_context = kubeconfig
                .contexts
                .iter()
                .find(|n| n.name == *context)
                .ok_or_else(|| Error::Raw("Cannot get contexts".into()))?;

            let current_namespace = named_context
                .context
                .namespace
                .clone()
                .ok_or_else(|| Error::Raw("Cannot get current namespace".into()))?;

            let options = KubeConfigOptions {
                context: Some(named_context.name.to_string()),
                cluster: Some(named_context.context.cluster.to_string()),
                user: Some(named_context.context.user.to_string()),
            };

            let config = Config::from_custom_kubeconfig(kubeconfig.clone(), &options).await?;

            let client = Client::try_from(config)?;

            let server_url = cluster_server_url(&kubeconfig, named_context)?;

            (
                KubeClient::new(client, server_url),
                current_namespace,
                context.to_string(),
            )
        } else {
            let current_context = kubeconfig
                .current_context
                .clone()
                .ok_or_else(|| Error::Raw("Cannot get current context".into()))?;

            let named_context = kubeconfig
                .contexts
                .iter()
                .find(|n| n.name == current_context)
                .ok_or_else(|| Error::Raw("Cannot get contexts".into()))?;

            let current_namespace = named_context
                .context
                .namespace
                .clone()
                .ok_or_else(|| Error::Raw("Cannot get current namespace".into()))?;

            let server_url = cluster_server_url(&kubeconfig, named_context)?;

            let client = Client::try_default().await?;

            (
                KubeClient::new(client, server_url),
                current_namespace,
                current_context,
            )
        };

        let namespaces = Arc::new(RwLock::new(vec![current_namespace.to_string()]));
        let api_resources: ApiResources = Default::default();

        tx.send(Event::Kube(Kube::GetCurrentContextResponse(
            current_context.to_string(),
            current_namespace.to_string(),
        )))?;

        let poll_worker = PollWorker {
            namespaces: namespaces.clone(),
            tx: tx.clone(),
            is_terminated: is_terminated.clone(),
            kube_client,
        };

        let main_handler = MainWorker {
            inner: poll_worker.clone(),
            api_resources: api_resources.clone(),
            rx: rx.clone(),
            contexts: kubeconfig.contexts.clone(),
        }
        .spawn();

        let pod_handler = PodPollWorker::new(poll_worker.clone()).spawn();
        let config_handler = ConfigsPollWorker::new(poll_worker.clone()).spawn();
        let event_handler = EventPollWorker::new(poll_worker.clone()).spawn();
        let apis_handler = ApiPollWorker::new(poll_worker.clone(), api_resources).spawn();

        let mut handlers = vec![
            main_handler,
            pod_handler,
            config_handler,
            event_handler,
            apis_handler,
        ];

        fn abort<T>(handlers: &[JoinHandle<T>]) {
            for h in handlers {
                h.abort()
            }
        }

        while !handlers.is_empty() {
            let (ret, _, vec) = select_all(handlers).await;

            handlers = vec;

            match ret {
                Ok(h) => match h {
                    Ok(result) => match result {
                        WorkerResult::ChangedContext(ctx) => {
                            abort(&handlers);

                            context = Some(ctx);
                        }
                        WorkerResult::Terminated => {}
                    },
                    Err(_) => tx.send(Event::Error(Error::Raw("KubeProcess Error".to_string())))?,
                },
                Err(_) => {
                    abort(&handlers);
                    tx.send(Event::Error(Error::Raw("KubeProcess Error".to_string())))?;
                }
            }
        }
    }

    Ok(())
}

pub fn kube_process(
    tx: Sender<Event>,
    rx: Receiver<Event>,
    is_terminated: Arc<AtomicBool>,
) -> Result<()> {
    let is_terminated_clone = is_terminated.clone();
    panic_set_hook!({
        is_terminated_clone.store(true, std::sync::atomic::Ordering::Relaxed);
    });

    let rt = Runtime::new()?;

    rt.block_on(inner_kube_process(tx, rx, is_terminated))?;

    #[cfg(feature = "logging")]
    ::log::debug!("Terminated kube event");

    Ok(())
}

async fn namespace_list(client: KubeClient) -> Vec<String> {
    let namespaces: Api<Namespace> = Api::all(client.client_clone());
    let lp = ListParams::default();
    let ns_list = namespaces.list(&lp).await.unwrap();

    ns_list.iter().map(|ns| ns.name()).collect()
}

#[derive(Clone)]
struct MainWorker {
    inner: PollWorker,
    api_resources: ApiResources,
    rx: Receiver<Event>,
    contexts: Vec<NamedContext>,
}

#[derive(Clone)]
struct MainWorkerArgs {
    api_resources: ApiResources,
    rx: Receiver<Event>,
    contexts: Vec<NamedContext>,
}

#[async_trait]
impl Worker for MainWorker {
    type Output = Result<WorkerResult>;

    async fn run(&self) -> Self::Output {
        let mut log_stream_handler: Option<Handlers> = None;

        let MainWorker {
            inner: poll_worker,
            api_resources,
            rx,
            contexts,
        } = self;

        let PollWorker {
            namespaces,
            tx,
            is_terminated,
            kube_client,
        } = poll_worker;

        while !is_terminated.load(std::sync::atomic::Ordering::Relaxed) {
            let rx = rx.clone();
            let tx = tx.clone();

            let task = tokio::task::spawn_blocking(move || rx.recv_timeout(Duration::from_secs(1)));

            if let Ok(recv) = task.await {
                match recv {
                    Ok(Event::Kube(ev)) => match ev {
                        Kube::SetNamespaces(ns) => {
                            {
                                let mut namespace = namespaces.write().await;
                                *namespace = ns;
                            }

                            if let Some(handler) = log_stream_handler {
                                handler.abort();
                                log_stream_handler = None;
                            }
                        }

                        Kube::GetNamespacesRequest => {
                            let res = namespace_list(kube_client.clone()).await;
                            tx.send(Event::Kube(Kube::GetNamespacesResponse(res)))?;
                        }

                        Kube::LogStreamRequest(namespace, pod_name) => {
                            if let Some(handler) = log_stream_handler {
                                handler.abort();
                            }

                            log_stream_handler = Some(
                                LogWorkerBuilder::new(tx, kube_client.clone(), namespace, pod_name)
                                    .build()
                                    .spawn(),
                            );

                            task::yield_now().await;
                        }

                        Kube::ConfigRequest(ns, kind, name) => {
                            let raw = get_config(kube_client.clone(), &ns, &kind, &name).await;
                            tx.send(Event::Kube(Kube::ConfigResponse(raw)))?;
                        }

                        Kube::GetAPIsRequest => {
                            let apis = apis_list(kube_client.clone()).await;

                            tx.send(Event::Kube(Kube::GetAPIsResponse(apis)))?;
                        }

                        Kube::SetAPIsRequest(apis) => {
                            let mut api_resources = api_resources.write().await;
                            *api_resources = apis;
                        }

                        Kube::GetContextsRequest => {
                            let contexts = contexts.iter().cloned().map(|ctx| ctx.name).collect();
                            let contexts = Ok(contexts);

                            tx.send(Event::Kube(Kube::GetContextsResponse(contexts)))?
                        }

                        Kube::SetContext(ctx) => {
                            return Ok(WorkerResult::ChangedContext(ctx));
                        }
                        _ => unreachable!(),
                    },
                    Ok(_) => unreachable!(),
                    Err(_) => {}
                }
            }
        }

        Ok(WorkerResult::Terminated)
    }
}
