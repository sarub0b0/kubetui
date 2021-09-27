mod api_resources;
mod config;
mod event;
mod log;
mod metric_type;
mod pod;
mod request;
mod v1_table;

use self::event::event_loop;
use super::Event;
use api_resources::{apis_list, apis_loop};
use config::{configs_loop, get_config};
use futures::future::select_all;
use k8s_openapi::api::core::v1::Namespace;
use pod::pod_loop;

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
    kubernetes::log::LogWorkerBuilder,
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

#[derive(Clone)]
pub struct KubeArgs {
    pub client: Client,
    pub server_url: String,
    pub current_context: String,
    pub current_namespace: String,
    pub contexts: Vec<NamedContext>,
    pub is_terminated: Arc<AtomicBool>,
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

type Namespaces = Arc<RwLock<Vec<String>>>;
type ApiResources = Arc<RwLock<Vec<String>>>;

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

    let is_terminated_clone = is_terminated.clone();

    let mut context: Option<String> = None;

    while !is_terminated.load(std::sync::atomic::Ordering::Relaxed) {
        let (client, server_url, current_namespace, current_context) =
            if let Some(context) = &context {
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

                (client, server_url, current_namespace, context.to_string())
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

                (client, server_url, current_namespace, current_context)
            };

        let namespaces = Arc::new(RwLock::new(vec![current_namespace.to_string()]));
        let api_resources: ApiResources = Default::default();

        tx.send(Event::Kube(Kube::GetCurrentContextResponse(
            current_context.to_string(),
            current_namespace.to_string(),
        )))?;

        let args = Arc::new(KubeArgs {
            client,
            server_url,
            current_context,
            current_namespace,
            contexts: kubeconfig.contexts.clone(),
            is_terminated: is_terminated_clone.clone(),
        });

        let main_loop = tokio::spawn(main_loop(
            rx.clone(),
            tx.clone(),
            Arc::clone(&namespaces),
            Arc::clone(&api_resources),
            args.clone(),
        ));

        let pod_loop = tokio::spawn(pod_loop(tx.clone(), Arc::clone(&namespaces), args.clone()));

        let config_loop = tokio::spawn(configs_loop(
            tx.clone(),
            Arc::clone(&namespaces),
            args.clone(),
        ));

        let event_loop = tokio::spawn(event_loop(
            tx.clone(),
            Arc::clone(&namespaces),
            args.clone(),
        ));

        let apis_loop = tokio::spawn(apis_loop(
            tx.clone(),
            Arc::clone(&namespaces),
            api_resources,
            args.clone(),
        ));

        let mut handlers = vec![main_loop, pod_loop, config_loop, event_loop, apis_loop];

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

async fn namespace_list(client: Client) -> Vec<String> {
    let namespaces: Api<Namespace> = Api::all(client);
    let lp = ListParams::default();
    let ns_list = namespaces.list(&lp).await.unwrap();

    ns_list.iter().map(|ns| ns.name()).collect()
}

async fn main_loop(
    rx: Receiver<Event>,
    tx: Sender<Event>,
    namespaces: Namespaces,
    api_resources: ApiResources,
    args: Arc<KubeArgs>,
) -> Result<WorkerResult> {
    let mut log_stream_handler: Option<Handlers> = None;
    let client = &args.client;
    let server_url = &args.server_url;

    while !args
        .is_terminated
        .load(std::sync::atomic::Ordering::Relaxed)
    {
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
                        let client = args.client.clone();
                        let res = namespace_list(client).await;
                        tx.send(Event::Kube(Kube::GetNamespacesResponse(res)))?;
                    }

                    Kube::LogStreamRequest(namespace, pod_name) => {
                        if let Some(handler) = log_stream_handler {
                            handler.abort();
                        }

                        let client = args.client.clone();

                        log_stream_handler = Some(
                            LogWorkerBuilder::new(tx, client, namespace, pod_name)
                                .build()
                                .spawn(),
                        );

                        task::yield_now().await;
                    }

                    Kube::ConfigRequest(ns, kind, name) => {
                        let client = args.client.clone();
                        let raw = get_config(client, &ns, &kind, &name).await;
                        tx.send(Event::Kube(Kube::ConfigResponse(raw)))?;
                    }

                    Kube::GetCurrentContextRequest => {
                        tx.send(Event::Kube(Kube::GetCurrentContextResponse(
                            args.current_context.to_string(),
                            args.current_namespace.to_string(),
                        )))?;
                    }

                    Kube::GetAPIsRequest => {
                        let apis = apis_list(client, server_url).await;

                        tx.send(Event::Kube(Kube::GetAPIsResponse(apis)))?;
                    }

                    Kube::SetAPIsRequest(apis) => {
                        let mut api_resources = api_resources.write().await;
                        *api_resources = apis;
                    }

                    Kube::GetContextsRequest => {
                        let contexts = args.contexts.iter().cloned().map(|ctx| ctx.name).collect();
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

#[async_trait]
trait JobWorker {
    type Output;

    async fn run(&self) -> Self::Output;

    fn spawn(&self) -> JoinHandle<Self::Output>
    where
        Self: Clone + Send + Sync + 'static,
        Self::Output: Send,
    {
        let worker = self.clone();
        tokio::spawn(async move { worker.run().await })
    }
}
