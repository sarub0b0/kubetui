mod api_resources;
mod client;
pub mod config;
mod event;
mod log;
mod metric_type;
pub mod network;
mod pod;
mod v1_table;
mod worker;
pub mod yaml;

use self::{
    api_resources::{apis_list_from_api_database, ApiDatabase},
    config::ConfigMessage,
    network::NetworkDescriptionWorker,
    yaml::{
        fetch_resource_list::FetchResourceList,
        worker::{YamlWorker, YamlWorkerRequest},
        YamlMessage, YamlRequest, YamlResponse,
    },
};

use super::Event;
use client::KubeClient;
use config::get_config;
use futures::future::select_all;
use k8s_openapi::api::core::v1::Namespace;
use worker::Worker;

use std::{
    collections::{BTreeMap, HashMap},
    convert::TryFrom,
    panic,
    sync::atomic::AtomicBool,
    sync::Arc,
    time::Duration,
};

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
    error::{anyhow, Error, Result},
    event::kubernetes::network::NetworkPollWorker,
    panic_set_hook,
};

use self::{
    api_resources::ApiPollWorker, config::ConfigsPollWorker, event::EventPollWorker,
    log::LogWorkerBuilder, network::NetworkMessage, pod::PodPollWorker, worker::PollWorker,
};

pub use kube;

#[derive(Debug, Default)]
pub struct KubeListItem {
    pub namespace: String,
    pub name: String,
    pub metadata: Option<BTreeMap<String, String>>,
    pub item: String,
}

#[derive(Debug, Default)]
pub struct KubeList {
    pub list: Vec<KubeListItem>,
}

impl KubeList {
    pub fn new(list: Vec<KubeListItem>) -> Self {
        Self { list }
    }
}

#[derive(Debug, Default)]
pub struct KubeTableRow {
    pub namespace: String,
    pub name: String,
    pub metadata: Option<BTreeMap<String, String>>,
    pub row: Vec<String>,
}

#[derive(Debug, Default)]
pub struct KubeTable {
    pub header: Vec<String>,
    pub rows: Vec<KubeTableRow>,
}

impl KubeTable {
    pub fn header(&self) -> &Vec<String> {
        &self.header
    }

    pub fn rows(&self) -> &Vec<KubeTableRow> {
        &self.rows
    }

    pub fn push_row(&mut self, row: impl Into<KubeTableRow>) {
        let row = row.into();

        debug_assert!(
            self.header.len() == row.row.len(),
            "Mismatch header({}) != row({})",
            self.header.len(),
            row.row.len()
        );

        self.rows.push(row);
    }

    pub fn update_rows(&mut self, rows: Vec<KubeTableRow>) {
        if !rows.is_empty() {
            for row in rows.iter() {
                debug_assert!(
                    self.header.len() == row.row.len(),
                    "Mismatch header({}) != row({})",
                    self.header.len(),
                    row.row.len()
                );
            }
        }

        self.rows = rows;
    }
}

impl From<Kube> for Event {
    fn from(k: Kube) -> Self {
        Event::Kube(k)
    }
}

#[derive(Debug)]
pub enum Kube {
    // apis
    GetAPIsRequest,
    GetAPIsResponse(Result<Vec<String>>),
    SetAPIsRequest(Vec<String>),
    APIsResults(Result<Vec<String>>),
    RestoreAPIs(Vec<String>),
    // Context
    // for header
    GetContextsRequest,
    GetContextsResponse(Result<Vec<String>>),
    SetContext(String),
    GetCurrentContextRequest,
    GetCurrentContextResponse {
        current_context: String,
        current_namespace: String,
    },
    // Context Restore
    RestoreContext {
        context: String,
        namespaces: Vec<String>,
    },
    // Event
    Event(Result<Vec<String>>),
    // Namespace
    // for multiple namespace
    GetNamespacesRequest,
    GetNamespacesResponse(Vec<String>),
    SetNamespacesRequest(Vec<String>),
    SetNamespacesResponse(Vec<String>),
    // Pod Status
    Pod(Result<KubeTable>),
    // Pod Logs
    LogStreamRequest {
        namespace: String,
        name: String,
    },
    LogStreamResponse(Result<Vec<String>>),
    // ConfigMap & Secret
    Config(ConfigMessage),
    // Network
    Network(NetworkMessage),
    // Yaml
    Yaml(YamlMessage),
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

async fn current_namespace(client: KubeClient, named_context: &NamedContext) -> Result<String> {
    if let Some(ns) = &named_context.context.namespace {
        Ok(ns.to_string())
    } else {
        let namespaces = namespace_list(client).await;

        if namespaces.iter().any(|ns| ns == "default") {
            Ok("default".to_string())
        } else if !namespaces.is_empty() {
            Ok(namespaces[0].to_string())
        } else {
            Err(anyhow!(Error::Raw(
                "Cannot get current namespace, namespaces".to_string()
            )))
        }
    }
}

async fn kube_worker_builder(
    kubeconfig: &Kubeconfig,
    context: &Option<String>,
) -> Result<(KubeClient, String, String)> {
    let ret = if let Some(context) = &context {
        let named_context = kubeconfig
            .contexts
            .iter()
            .find(|n| n.name == *context)
            .ok_or_else(|| Error::Raw("Cannot get contexts".into()))?;

        let options = KubeConfigOptions {
            context: Some(named_context.name.to_string()),
            cluster: Some(named_context.context.cluster.to_string()),
            user: Some(named_context.context.user.to_string()),
        };

        let config = Config::from_custom_kubeconfig(kubeconfig.clone(), &options).await?;

        let client = Client::try_from(config)?;

        let server_url = cluster_server_url(kubeconfig, named_context)?;

        let kube_client = KubeClient::new(client, server_url);

        let current_namespace = current_namespace(kube_client.clone(), named_context).await?;
        (kube_client, current_namespace, context.to_string())
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

        let server_url = cluster_server_url(kubeconfig, named_context)?;

        let client = Client::try_default().await?;

        let kube_client = KubeClient::new(client, server_url);

        let current_namespace = current_namespace(kube_client.clone(), named_context).await?;

        (kube_client, current_namespace, current_context)
    };

    Ok(ret)
}

pub(super) type Namespaces = Arc<RwLock<Vec<String>>>;
pub(super) type ApiResources = Arc<RwLock<Vec<String>>>;

#[derive(Clone)]
pub enum WorkerResult {
    ChangedContext(String),
    Terminated,
}

#[derive(Debug, Default)]
struct KubeState {
    default_namespace: String,
    selected_namespaces: Vec<String>, // selected
    api_resources: Vec<String>,
}

impl KubeState {
    fn new(
        default_namespace: impl Into<String>,
        namespaces: impl Into<Vec<String>>,
        api_resources: impl Into<Vec<String>>,
    ) -> Self {
        Self {
            default_namespace: default_namespace.into(),
            selected_namespaces: namespaces.into(),
            api_resources: api_resources.into(),
        }
    }
}

fn restore_state(
    tx: &Sender<Event>,
    state: &HashMap<String, KubeState>,
    context: &str,
    namespace: &str,
) -> Result<(String, Vec<String>, Vec<String>)> {
    let ret = if let Some(state) = state.get(context) {
        let KubeState {
            default_namespace,
            selected_namespaces: namespaces,
            api_resources,
        } = state;

        tx.send(Event::Kube(Kube::RestoreContext {
            context: context.to_string(),
            namespaces: namespaces.to_owned(),
        }))?;

        tx.send(Event::Kube(Kube::RestoreAPIs(api_resources.to_vec())))?;

        (
            default_namespace.to_string(),
            namespaces.to_owned(),
            api_resources.to_owned(),
        )
    } else {
        tx.send(Event::Kube(Kube::GetCurrentContextResponse {
            current_context: context.to_string(),
            current_namespace: namespace.to_string(),
        }))?;

        (
            namespace.to_string(),
            vec![namespace.to_string()],
            Default::default(),
        )
    };

    Ok(ret)
}

async fn inner_kube_process(
    tx: Sender<Event>,
    rx: Receiver<Event>,
    is_terminated: Arc<AtomicBool>,
) -> Result<()> {
    let kubeconfig = Kubeconfig::read()?;

    let mut context: Option<String> = None;

    let mut kube_state: HashMap<String, KubeState> = HashMap::new();

    while !is_terminated.load(std::sync::atomic::Ordering::Relaxed) {
        let (kube_client, current_namespace, current_context) =
            kube_worker_builder(&kubeconfig, &context).await?;

        // Restore
        let (current_namespace, namespaces, api_resources) =
            restore_state(&tx, &kube_state, &current_context, &current_namespace)?;

        let shared_namespaces = Arc::new(RwLock::new(namespaces.clone()));
        let shared_api_resources = Arc::new(RwLock::new(api_resources.clone()));
        let shared_api_database = Arc::new(RwLock::new(HashMap::new()));

        let poll_worker = PollWorker {
            namespaces: shared_namespaces.clone(),
            tx: tx.clone(),
            is_terminated: is_terminated.clone(),
            kube_client,
        };

        let main_handler = MainWorker {
            inner: poll_worker.clone(),
            rx: rx.clone(),
            contexts: kubeconfig.contexts.clone(),
            api_resources: shared_api_resources.clone(),
            api_database: shared_api_database.clone(),
        }
        .spawn();

        let pod_handler = PodPollWorker::new(poll_worker.clone()).spawn();
        let config_handler = ConfigsPollWorker::new(poll_worker.clone()).spawn();
        let network_handler = NetworkPollWorker::new(poll_worker.clone()).spawn();
        let event_handler = EventPollWorker::new(poll_worker.clone()).spawn();
        let apis_handler = ApiPollWorker::new(
            poll_worker.clone(),
            shared_api_resources.clone(),
            shared_api_database,
        )
        .spawn();

        let mut handlers = vec![
            main_handler,
            pod_handler,
            config_handler,
            network_handler,
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

                            let namespaces = shared_namespaces.read().await;
                            let api_resources = shared_api_resources.read().await;

                            kube_state.insert(
                                current_context.to_string(),
                                KubeState::new(
                                    current_namespace.to_string(),
                                    namespaces.to_vec(),
                                    api_resources.to_vec(),
                                ),
                            );
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

    if let Err(e) = rt.block_on(inner_kube_process(tx, rx, is_terminated)) {
        panic!("{}", e);
    }

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
    rx: Receiver<Event>,
    contexts: Vec<NamedContext>,
    api_resources: ApiResources,
    api_database: ApiDatabase,
}

#[async_trait]
impl Worker for MainWorker {
    type Output = Result<WorkerResult>;

    async fn run(&self) -> Self::Output {
        let mut log_stream_handler: Option<Handlers> = None;
        let mut network_handler: Option<JoinHandle<Result<()>>> = None;
        let mut yaml_handler: Option<JoinHandle<Result<()>>> = None;

        let MainWorker {
            inner: poll_worker,
            rx,
            contexts,
            api_resources,
            api_database,
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
                        Kube::SetNamespacesRequest(ns) => {
                            {
                                let mut namespace = namespaces.write().await;
                                *namespace = ns.clone();
                            }

                            if let Some(handler) = log_stream_handler {
                                handler.abort();
                                log_stream_handler = None;
                            }

                            if let Some(handler) = network_handler {
                                handler.abort();
                                network_handler = None;
                            }

                            tx.send(Event::Kube(Kube::SetNamespacesResponse(ns)))?;
                        }

                        Kube::GetNamespacesRequest => {
                            let res = namespace_list(kube_client.clone()).await;
                            tx.send(Event::Kube(Kube::GetNamespacesResponse(res)))?;
                        }

                        Kube::LogStreamRequest { namespace, name } => {
                            if let Some(handler) = log_stream_handler {
                                handler.abort();
                            }

                            log_stream_handler = Some(
                                LogWorkerBuilder::new(tx, kube_client.clone(), namespace, name)
                                    .build()
                                    .spawn(),
                            );

                            task::yield_now().await;
                        }

                        Kube::Config(ConfigMessage::DataRequest {
                            namespace,
                            kind,
                            name,
                        }) => {
                            let raw =
                                get_config(kube_client.clone(), &namespace, &kind, &name).await;
                            tx.send(ConfigMessage::DataResponse(raw).into())?;
                        }

                        Kube::GetAPIsRequest => {
                            let db = api_database.read().await;
                            let apis = apis_list_from_api_database(&db);
                            tx.send(Event::Kube(Kube::GetAPIsResponse(Ok(apis))))?;
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

                        Kube::Yaml(YamlMessage::Request(ev)) => {
                            use YamlRequest::*;
                            match ev {
                                APIs => {
                                    let db = api_database.read().await;
                                    let apis = apis_list_from_api_database(&db);

                                    tx.send(YamlResponse::APIs(Ok(apis)).into())?
                                }
                                Resource(req) => {
                                    let db = api_database.read().await;
                                    let ns = namespaces.read().await;

                                    let fetched_data =
                                        FetchResourceList::new(kube_client, req, &db, &ns)
                                            .fetch()
                                            .await;

                                    tx.send(YamlResponse::Resource(fetched_data).into())?
                                }
                                Yaml {
                                    kind,
                                    name,
                                    namespace,
                                } => {
                                    if let Some(handler) = yaml_handler {
                                        handler.abort();
                                    }

                                    let req = YamlWorkerRequest {
                                        kind,
                                        name,
                                        namespace,
                                    };

                                    yaml_handler = Some(
                                        YamlWorker::new(
                                            is_terminated.clone(),
                                            tx,
                                            kube_client.clone(),
                                            api_database.clone(),
                                            req,
                                        )
                                        .spawn(),
                                    );
                                    task::yield_now().await;
                                }
                            }
                        }

                        Kube::Network(NetworkMessage::Request(req)) => {
                            if let Some(handler) = network_handler {
                                handler.abort();
                            }

                            network_handler = Some(
                                NetworkDescriptionWorker::new(
                                    is_terminated.clone(),
                                    tx,
                                    kube_client.clone(),
                                    req,
                                )
                                .spawn(),
                            );

                            task::yield_now().await;
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
