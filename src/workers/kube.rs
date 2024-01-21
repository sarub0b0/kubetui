pub mod api_resources;
mod client;
mod color;
pub mod config;
mod event;
mod metric_type;
pub mod network;
pub mod pod;
mod v1_table;
mod worker;
pub mod yaml;

use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use anyhow::Result;
use async_trait::async_trait;
use crossbeam::channel::{Receiver, Sender};
use k8s_openapi::api::core::v1::Namespace;
use kube::{api::ListParams, Api, ResourceExt};
use tokio::{
    runtime::Runtime,
    sync::RwLock,
    task::{self, AbortHandle},
};

use crate::{logger, message::Message, panic_set_hook};

use self::{
    api_resources::{ApiMessage, ApiRequest, ApiResource, ApiResponse, SharedApiResources},
    client::KubeClient,
    config::{ConfigMessage, ConfigsDataWorker},
    context_message::{ContextMessage, ContextRequest, ContextResponse},
    inner::Inner,
    namespace_message::{NamespaceMessage, NamespaceRequest, NamespaceResponse},
    network::{NetworkDescriptionWorker, NetworkMessage},
    pod::{LogMessage, LogWorker},
    worker::{AbortWorker, PollWorker, Worker},
    yaml::{
        direct::DirectedYamlWorker,
        select::{resources::FetchResourceList, worker::SelectedYamlWorker},
        YamlMessage, YamlRequest, YamlResponse,
    },
};

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

impl From<Kube> for Message {
    fn from(k: Kube) -> Self {
        Message::Kube(k)
    }
}

#[derive(Debug)]
pub enum Kube {
    Context(ContextMessage),
    API(ApiMessage),
    RestoreAPIs(TargetApiResources),
    RestoreContext {
        context: String,
        namespaces: TargetNamespaces,
    },
    Event(Result<Vec<String>>),
    Namespace(NamespaceMessage),
    Pod(Result<KubeTable>),
    Log(LogMessage),
    Config(ConfigMessage),
    Network(NetworkMessage),
    Yaml(YamlMessage),
}

pub mod namespace_message {
    use crate::message::Message;
    use anyhow::Result;

    use super::{Kube, TargetNamespaces};

    #[derive(Debug)]
    pub enum NamespaceMessage {
        Request(NamespaceRequest),
        Response(NamespaceResponse),
    }

    #[derive(Debug)]
    pub enum NamespaceRequest {
        Get,
        Set(TargetNamespaces),
    }

    #[derive(Debug)]
    pub enum NamespaceResponse {
        Get(Result<TargetNamespaces>),
        Set(TargetNamespaces),
    }

    impl From<NamespaceRequest> for Message {
        fn from(n: NamespaceRequest) -> Self {
            Message::Kube(Kube::Namespace(NamespaceMessage::Request(n)))
        }
    }

    impl From<NamespaceResponse> for Message {
        fn from(n: NamespaceResponse) -> Self {
            Message::Kube(Kube::Namespace(NamespaceMessage::Response(n)))
        }
    }
}

pub mod context_message {
    use super::Kube;
    use crate::message::Message;

    #[derive(Debug)]
    pub enum ContextMessage {
        Request(ContextRequest),
        Response(ContextResponse),
    }

    #[derive(Debug)]
    pub enum ContextRequest {
        Get,
        Set(String),
    }

    #[derive(Debug)]
    pub enum ContextResponse {
        Get(Vec<String>),
    }

    impl From<ContextMessage> for Message {
        fn from(m: ContextMessage) -> Self {
            Message::Kube(Kube::Context(m))
        }
    }

    impl From<ContextRequest> for Message {
        fn from(m: ContextRequest) -> Self {
            Message::Kube(Kube::Context(ContextMessage::Request(m)))
        }
    }

    impl From<ContextResponse> for Message {
        fn from(m: ContextResponse) -> Self {
            Message::Kube(Kube::Context(ContextMessage::Response(m)))
        }
    }
}

pub(super) type TargetNamespaces = Vec<String>;
pub(super) type SharedTargetNamespaces = Arc<RwLock<TargetNamespaces>>;

pub(super) type TargetApiResources = Vec<ApiResource>;
pub(super) type SharedTargetApiResources = Arc<RwLock<TargetApiResources>>;

#[derive(Clone)]
pub enum WorkerResult {
    ChangedContext(String),
    Terminated,
}

async fn fetch_all_namespaces(client: KubeClient) -> Result<Vec<String>> {
    let namespaces: Api<Namespace> = Api::all(client.as_client().clone());
    let lp = ListParams::default();
    let ns_list = namespaces.list(&lp).await?;

    Ok(ns_list.iter().map(|ns| ns.name_any()).collect())
}

#[derive(Debug, Default, Clone)]
pub struct KubeWorkerConfig {
    pub kubeconfig: Option<PathBuf>,
    pub target_namespaces: Option<TargetNamespaces>,
    pub context: Option<String>,
    pub all_namespaces: bool,
}

#[derive(Debug, Clone)]
pub struct KubeWorker {
    pub(super) tx: Sender<Message>,
    pub(super) rx: Receiver<Message>,
    pub(super) is_terminated: Arc<AtomicBool>,
    pub(super) config: KubeWorkerConfig,
}

impl KubeWorker {
    pub fn new(
        tx: Sender<Message>,
        rx: Receiver<Message>,
        is_terminated: Arc<AtomicBool>,
        config: KubeWorkerConfig,
    ) -> Self {
        KubeWorker {
            tx,
            rx,
            is_terminated,
            config,
        }
    }

    pub fn start(&self) -> Result<()> {
        logger!(info, "KubeWorker start");

        let rt = Runtime::new()?;

        let ret = rt.block_on(Self::inner(self.clone()));

        logger!(info, "KubeWorker end");

        if let Err(e) = ret {
            self.is_terminated.store(true, Ordering::Relaxed);

            Err(e)
        } else {
            Ok(())
        }
    }

    async fn inner(worker: KubeWorker) -> Result<()> {
        let inner = Inner::try_from(worker).await?;
        inner.run().await
    }

    pub fn set_panic_hook(&self) {
        let is_terminated = self.is_terminated.clone();

        panic_set_hook!({
            is_terminated.store(true, Ordering::Relaxed);
        });
    }
}

#[derive(Clone)]
struct MainWorker {
    inner: PollWorker,
    rx: Receiver<Message>,
    contexts: Vec<String>,
    shared_target_api_resources: SharedTargetApiResources,
    shared_api_resources: SharedApiResources,
}

#[async_trait]
impl Worker for MainWorker {
    type Output = WorkerResult;

    async fn run(&self) -> Self::Output {
        let mut log_handler: Option<AbortHandle> = None;
        let mut config_handler: Option<AbortHandle> = None;
        let mut network_handler: Option<AbortHandle> = None;
        let mut yaml_handler: Option<AbortHandle> = None;

        let MainWorker {
            inner: poll_worker,
            rx,
            contexts,
            shared_target_api_resources,
            shared_api_resources,
        } = self;

        let PollWorker {
            shared_target_namespaces,
            tx,
            is_terminated,
            kube_client,
        } = poll_worker;

        while !is_terminated.load(std::sync::atomic::Ordering::Relaxed) {
            let rx = rx.clone();
            let tx = tx.clone();

            let task = tokio::task::spawn_blocking(move || rx.recv_timeout(Duration::from_secs(1)));

            let Ok(recv) = task.await else { continue };

            match recv {
                Ok(Message::Kube(ev)) => match ev {
                    Kube::Namespace(NamespaceMessage::Request(req)) => match req {
                        NamespaceRequest::Get => {
                            let ns = fetch_all_namespaces(kube_client.clone()).await;
                            tx.send(NamespaceResponse::Get(ns).into())
                                .expect("Failed to send NamespaceResponse::Get");
                        }
                        NamespaceRequest::Set(req) => {
                            {
                                let mut target_namespaces = shared_target_namespaces.write().await;
                                *target_namespaces = req.clone();
                            }

                            if let Some(handler) = log_handler {
                                handler.abort();
                                log_handler = None;
                            }

                            if let Some(handler) = config_handler {
                                handler.abort();
                                config_handler = None;
                            }

                            if let Some(handler) = network_handler {
                                handler.abort();
                                network_handler = None;
                            }

                            if let Some(handler) = yaml_handler {
                                handler.abort();
                                yaml_handler = None;
                            }

                            tx.send(NamespaceResponse::Set(req).into())
                                .expect("Failed to send NamespaceResponse:Set");
                        }
                    },

                    Kube::Log(LogMessage::Request(req)) => {
                        if let Some(handler) = log_handler {
                            handler.abort();
                        }

                        log_handler = Some(LogWorker::new(tx, kube_client.clone(), req).spawn());

                        task::yield_now().await;
                    }

                    Kube::Config(ConfigMessage::Request(req)) => {
                        if let Some(handler) = config_handler {
                            handler.abort();
                        }

                        config_handler = Some(
                            ConfigsDataWorker::new(
                                is_terminated.clone(),
                                tx,
                                kube_client.clone(),
                                req,
                            )
                            .spawn(),
                        );

                        task::yield_now().await;
                    }

                    Kube::API(ApiMessage::Request(req)) => {
                        use ApiRequest::*;
                        match req {
                            Get => {
                                let api_resources = shared_api_resources.read().await;
                                tx.send(ApiResponse::Get(Ok(api_resources.to_vec())).into())
                                    .expect("Failed to send ApiResponse::Get");
                            }
                            Set(req) => {
                                let mut target_api_resources =
                                    shared_target_api_resources.write().await;
                                *target_api_resources = req.clone();
                            }
                        }
                    }

                    Kube::Context(ContextMessage::Request(req)) => match req {
                        ContextRequest::Get => tx
                            .send(ContextResponse::Get(contexts.to_vec()).into())
                            .expect("Failed to send ContextResponse::Get"),
                        ContextRequest::Set(req) => {
                            if let Some(h) = log_handler {
                                h.abort();
                            }

                            if let Some(h) = config_handler {
                                h.abort();
                            }

                            if let Some(h) = network_handler {
                                h.abort();
                            }

                            if let Some(h) = yaml_handler {
                                h.abort();
                            }

                            return WorkerResult::ChangedContext(req);
                        }
                    },

                    Kube::Yaml(YamlMessage::Request(ev)) => {
                        use YamlRequest::*;
                        match ev {
                            APIs => {
                                let api_resources = shared_api_resources.read().await;

                                tx.send(YamlResponse::APIs(Ok(api_resources.to_vec())).into())
                                    .expect("Failed to send YamlResponse::Apis");
                            }
                            Resource(req) => {
                                let api_resources = shared_api_resources.read().await;
                                let target_namespaces = shared_target_namespaces.read().await;

                                let fetched_data = FetchResourceList::new(
                                    kube_client,
                                    req,
                                    &api_resources,
                                    &target_namespaces,
                                )
                                .fetch()
                                .await;

                                tx.send(YamlResponse::Resource(fetched_data).into())
                                    .expect("Failed to send YamlResponse::Resource");
                            }
                            SelectedYaml(req) => {
                                if let Some(handler) = yaml_handler {
                                    handler.abort();
                                }

                                yaml_handler = Some(
                                    SelectedYamlWorker::new(
                                        is_terminated.clone(),
                                        tx,
                                        kube_client.clone(),
                                        shared_api_resources.clone(),
                                        req,
                                    )
                                    .spawn(),
                                );
                                task::yield_now().await;
                            }

                            DirectedYaml(req) => {
                                if let Some(handler) = yaml_handler {
                                    handler.abort();
                                }

                                yaml_handler = Some(
                                    DirectedYamlWorker::new(
                                        is_terminated.clone(),
                                        tx,
                                        kube_client.clone(),
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

        WorkerResult::Terminated
    }
}

mod inner {
    use std::{
        path::PathBuf,
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        },
    };

    use anyhow::{anyhow, Result};
    use crossbeam::channel::{Receiver, Sender};
    use futures::future::select_all;
    use kube::config::{Kubeconfig, KubeconfigError};
    use tokio::{sync::RwLock, task::JoinHandle};

    use crate::{
        error::Error,
        message::Message,
        workers::kube::{
            api_resources::{ApiPollWorker, SharedApiResources},
            config::ConfigsPollWorker,
            event::EventPollWorker,
            network::NetworkPollWorker,
            pod::PodPollWorker,
            worker::{PollWorker, Worker},
            MainWorker, WorkerResult,
        },
    };

    use super::{
        fetch_all_namespaces,
        kube_store::{KubeState, KubeStore},
        Kube, {KubeWorker, KubeWorkerConfig},
    };

    pub struct Inner {
        tx: Sender<Message>,
        rx: Receiver<Message>,
        is_terminated: Arc<AtomicBool>,
        kubeconfig: Kubeconfig,
        context: String,
        store: KubeStore,
    }

    struct Context {
        pub inner: String,
    }

    impl Context {
        fn try_from(kubeconfig: &Kubeconfig, context: Option<String>) -> Result<Self> {
            let context = if let Some(context) = context {
                kubeconfig
                    .contexts
                    .iter()
                    .find_map(|ctx| {
                        if ctx.name == context {
                            Some(ctx.name.to_string())
                        } else {
                            None
                        }
                    })
                    .ok_or_else(|| anyhow!(format!("Cannot find context {}", context)))?
            } else if let Some(current_context) = &kubeconfig.current_context {
                current_context.to_string()
            } else {
                kubeconfig
                    .contexts
                    .first()
                    .ok_or_else(|| anyhow!("Empty contexts"))?
                    .name
                    .to_string()
            };

            Ok(Self { inner: context })
        }
    }

    fn read_kubeconfig(kubeconfig: Option<PathBuf>) -> Result<Kubeconfig, KubeconfigError> {
        if let Some(path) = kubeconfig {
            Kubeconfig::read_from(path)
        } else {
            Kubeconfig::read()
        }
    }

    impl Inner {
        pub async fn try_from(worker: KubeWorker) -> Result<Self> {
            let KubeWorker {
                tx,
                rx,
                is_terminated,
                config,
            } = worker;

            let KubeWorkerConfig {
                kubeconfig,
                target_namespaces,
                context,
                all_namespaces,
            } = config;

            let kubeconfig = read_kubeconfig(kubeconfig)?;

            let Context { inner: context } = Context::try_from(&kubeconfig, context)?;

            let mut store = KubeStore::try_from_kubeconfig(kubeconfig.clone()).await?;

            let KubeState {
                client: state_client,
                target_namespaces: state_of_target_namespaces,
                ..
            } = store.get_mut(&context)?;

            if let Some(namespaces) = target_namespaces {
                *state_of_target_namespaces = namespaces;
            }

            if all_namespaces {
                let target_namespaces = fetch_all_namespaces(state_client.clone()).await?;

                *state_of_target_namespaces = target_namespaces;
            }

            Ok(Self {
                tx,
                rx,
                is_terminated,
                kubeconfig,
                context,
                store,
            })
        }

        pub async fn run(self) -> Result<()> {
            let Self {
                tx,
                rx,
                is_terminated,
                kubeconfig,
                mut context,
                mut store,
            } = self;

            while !is_terminated.load(Ordering::Relaxed) {
                let KubeState {
                    client,
                    target_namespaces,
                    target_api_resources,
                } = store.get(&context)?.clone();

                tx.send(Message::Kube(Kube::RestoreContext {
                    context: context.to_string(),
                    namespaces: target_namespaces.to_vec(),
                }))?;

                tx.send(Message::Kube(Kube::RestoreAPIs(
                    target_api_resources.to_vec(),
                )))?;

                let shared_target_namespaces = Arc::new(RwLock::new(target_namespaces.to_vec()));
                let shared_target_api_resources =
                    Arc::new(RwLock::new(target_api_resources.to_vec()));
                let shared_api_resources = SharedApiResources::default();

                let poll_worker = PollWorker {
                    shared_target_namespaces: shared_target_namespaces.clone(),
                    tx: tx.clone(),
                    is_terminated: is_terminated.clone(),
                    kube_client: client.clone(),
                };

                let main_handler = MainWorker {
                    inner: poll_worker.clone(),
                    rx: rx.clone(),
                    contexts: kubeconfig
                        .contexts
                        .iter()
                        .map(|ctx| ctx.name.to_string())
                        .collect(),
                    shared_target_api_resources: shared_target_api_resources.clone(),
                    shared_api_resources: shared_api_resources.clone(),
                }
                .spawn();

                let pod_handler = PodPollWorker::new(poll_worker.clone()).spawn();
                let config_handler = ConfigsPollWorker::new(poll_worker.clone()).spawn();
                let network_handler = NetworkPollWorker::new(poll_worker.clone()).spawn();
                let event_handler = EventPollWorker::new(poll_worker.clone()).spawn();
                let apis_handler = ApiPollWorker::new(
                    poll_worker.clone(),
                    shared_target_api_resources.clone(),
                    shared_api_resources,
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
                        Ok(handler) => match handler {
                            WorkerResult::ChangedContext(ctx) => {
                                abort(&handlers);

                                let target_namespaces = shared_target_namespaces.read().await;
                                let target_api_resources = shared_target_api_resources.read().await;

                                store.insert(
                                    context.to_string(),
                                    KubeState::new(
                                        client.clone(),
                                        target_namespaces.to_vec(),
                                        target_api_resources.to_vec(),
                                    ),
                                );

                                context = ctx;
                            }
                            WorkerResult::Terminated => {}
                        },
                        Err(e) => {
                            abort(&handlers);
                            tx.send(Message::Error(Error::Raw(format!(
                                "KubeProcess Error: {:?}",
                                e
                            ))))?;
                        }
                    }
                }
            }

            Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        use indoc::indoc;

        use super::*;

        const CONFIG: &str = indoc! {
            r#"
            apiVersion: v1
            clusters:
              - cluster:
                  certificate-authority-data: ""
                  server: https://192.168.0.1
                name: cluster-1
              - cluster:
                  certificate-authority-data: ""
                  server: https://192.168.0.2
                name: cluster-2
              - cluster:
                  certificate-authority-data: ""
                  server: https://192.168.0.3
                name: cluster-3
            contexts:
              - context:
                  cluster: cluster-1
                  namespace: ns-1
                  user: user-1
                name: cluster-1
              - context:
                  cluster: cluster-2
                  namespace: ns-2
                  user: user-2
                name: cluster-2
              - context:
                  cluster: cluster-3
                  user: user-3
                name: cluster-3
            current-context: cluster-2
            kind: Config
            preferences: {}
            users: []
            "#
        };

        mod context {
            use super::*;

            mod context指定あり {
                use super::*;
                use pretty_assertions::assert_eq;

                #[test]
                fn configに存在するときokを返す() {
                    let kubeconfig = Kubeconfig::from_yaml(CONFIG).unwrap();

                    let context =
                        Context::try_from(&kubeconfig, Some("cluster-1".to_string())).unwrap();

                    assert_eq!(context.inner, "cluster-1");
                }

                #[test]
                fn configに存在しないときerrを返す() {
                    let kubeconfig = Kubeconfig::from_yaml(CONFIG).unwrap();

                    let context = Context::try_from(&kubeconfig, Some("nothing".to_string()));

                    assert_eq!(context.is_err(), true);
                }
            }

            mod context指定なし {
                use super::*;
                use pretty_assertions::assert_eq;

                #[test]
                fn current_contextがあるときcurrent_contextを返す() {
                    let kubeconfig = Kubeconfig::from_yaml(CONFIG).unwrap();

                    let context = Context::try_from(&kubeconfig, None).unwrap();

                    assert_eq!(context.inner, "cluster-2");
                }

                #[test]
                fn current_contextがないとき1つ目のcontextを返す() {
                    let config = indoc! {
                        r#"
                        apiVersion: v1
                        clusters: []
                        contexts:
                          - context:
                              cluster: cluster-1
                              namespace: ns-1
                              user: user-1
                            name: cluster-1
                          - context:
                              cluster: cluster-2
                              namespace: ns-2
                              user: user-2
                            name: cluster-2
                          - context:
                              cluster: cluster-3
                              user: user-3
                            name: cluster-3
                        kind: Config
                        preferences: {}
                        users: []
                        "#
                    };

                    let kubeconfig = Kubeconfig::from_yaml(config).unwrap();

                    let context = Context::try_from(&kubeconfig, None).unwrap();

                    assert_eq!(context.inner, "cluster-1");
                }

                #[test]
                fn current_contextとcontextsがないときerrを返す() {
                    let config = indoc! {
                        r#"
                        apiVersion: v1
                        clusters: []
                        contexts: []
                        kind: Config
                        preferences: {}
                        users: []
                        "#
                    };

                    let kubeconfig = Kubeconfig::from_yaml(config).unwrap();

                    let context = Context::try_from(&kubeconfig, None);

                    assert_eq!(context.is_err(), true);
                }
            }
        }
    }
}

mod kube_store {
    use std::{collections::BTreeMap, fmt::Debug};

    use anyhow::{anyhow, Result};
    use futures::future::try_join_all;
    use kube::{
        config::{KubeConfigOptions, Kubeconfig},
        Client, Config,
    };

    use super::{client::KubeClient, TargetApiResources, TargetNamespaces};

    pub type Context = String;

    #[derive(Clone)]
    pub struct KubeState {
        pub client: KubeClient,
        pub target_namespaces: TargetNamespaces,
        pub target_api_resources: TargetApiResources,
    }

    impl KubeState {
        pub fn new(
            client: KubeClient,
            target_namespaces: TargetNamespaces,
            target_api_resources: TargetApiResources,
        ) -> Self {
            Self {
                client,
                target_namespaces,
                target_api_resources,
            }
        }
    }

    #[derive(Debug)]
    #[cfg_attr(test, derive(PartialEq))]
    pub struct KubeStore {
        inner: BTreeMap<Context, KubeState>,
    }

    impl From<BTreeMap<Context, KubeState>> for KubeStore {
        fn from(inner: BTreeMap<Context, KubeState>) -> Self {
            KubeStore { inner }
        }
    }

    impl std::fmt::Debug for KubeState {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "KubeStore {{ client: _, target_namespaces: {:?}, target_api_resources: {:?} }}",
                self.target_namespaces, self.target_api_resources
            )
        }
    }

    impl KubeStore {
        pub async fn try_from_kubeconfig(config: Kubeconfig) -> Result<Self> {
            let Kubeconfig {
                clusters,
                contexts,
                auth_infos,
                ..
            } = &config;

            let jobs: Vec<(Context, KubeState)> =
                try_join_all(contexts.iter().map(|context| async {
                    let cluster = clusters.iter().find_map(|cluster| {
                        if cluster.name == context.name {
                            Some(cluster.name.to_string())
                        } else {
                            None
                        }
                    });

                    let user = auth_infos.iter().find_map(|auth_info| {
                        let Some(kube::config::Context { ref user, .. }) = context.context else {
                            return None;
                        };

                        if &auth_info.name == user {
                            Some(auth_info.name.to_string())
                        } else {
                            None
                        }
                    });

                    let options = KubeConfigOptions {
                        context: Some(context.name.to_string()),
                        cluster,
                        user,
                    };

                    let config = Config::from_custom_kubeconfig(config.clone(), &options).await?;

                    let cluster_url: String = config.cluster_url.to_string();
                    let target_namespace = config.default_namespace.to_string();

                    let client = Client::try_from(config)?;

                    let kube_client = KubeClient::new(client, cluster_url);

                    anyhow::Ok((
                        context.name.to_string(),
                        KubeState {
                            client: kube_client,
                            target_namespaces: vec![target_namespace],
                            target_api_resources: vec![],
                        },
                    ))
                }))
                .await?;

            let inner: BTreeMap<Context, KubeState> = jobs.into_iter().collect();

            Ok(inner.into())
        }

        pub fn get(&self, context: &str) -> Result<&KubeState> {
            self.inner
                .get(context)
                .ok_or_else(|| anyhow!(format!("Cannot get context {}", context)))
        }

        pub fn get_mut(&mut self, context: &str) -> Result<&mut KubeState> {
            self.inner
                .get_mut(context)
                .ok_or_else(|| anyhow!(format!("Cannot get context {}", context)))
        }

        pub fn insert(&mut self, context: Context, state: KubeState) {
            self.inner.insert(context, state);
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use indoc::indoc;
        use pretty_assertions::assert_eq;

        impl PartialEq for KubeState {
            fn eq(&self, rhs: &Self) -> bool {
                self.target_namespaces == rhs.target_namespaces
                    && self.target_api_resources == rhs.target_api_resources
                    && self.client.as_server_url() == rhs.client.as_server_url()
            }
        }

        const CONFIG: &str = indoc! {
            r#"
            apiVersion: v1
            clusters:
              - cluster:
                  certificate-authority-data: ""
                  server: https://192.168.0.1
                name: cluster-1
              - cluster:
                  certificate-authority-data: ""
                  server: https://192.168.0.2
                name: cluster-2
              - cluster:
                  certificate-authority-data: ""
                  server: https://192.168.0.3
                name: cluster-3
            contexts:
              - context:
                  cluster: cluster-1
                  namespace: ns-1
                  user: user-1
                name: cluster-1
              - context:
                  cluster: cluster-2
                  namespace: ns-2
                  user: user-2
                name: cluster-2
              - context:
                  cluster: cluster-3
                  user: user-3
                name: cluster-3
            current-context: cluster-2
            kind: Config
            preferences: {}
            users:
              - name: user-1
                user:
                  token: user-1
              - name: user-2
                user:
                  token: user-2
              - name: user-3
                user:
                  token: user-3
            "#
        };

        #[tokio::test]
        async fn kubeconfigからstateを生成() {
            let kubeconfig = Kubeconfig::from_yaml(CONFIG).unwrap();

            let actual = KubeStore::try_from_kubeconfig(kubeconfig).await.unwrap();

            let config = Config::new(Default::default());

            let client = Client::try_from(config).unwrap();

            let expected = BTreeMap::from([
                (
                    "cluster-1".to_string(),
                    KubeState {
                        client: KubeClient::new(client.clone(), "https://192.168.0.1/"),
                        target_namespaces: vec!["ns-1".to_string()],
                        target_api_resources: Default::default(),
                    },
                ),
                (
                    "cluster-2".to_string(),
                    KubeState {
                        client: KubeClient::new(client.clone(), "https://192.168.0.2/"),
                        target_namespaces: vec!["ns-2".to_string()],
                        target_api_resources: Default::default(),
                    },
                ),
                (
                    "cluster-3".to_string(),
                    KubeState {
                        client: KubeClient::new(client, "https://192.168.0.3/"),
                        target_namespaces: vec!["default".to_string()],
                        target_api_resources: Default::default(),
                    },
                ),
            ])
            .into();

            assert_eq!(actual, expected);
        }
    }
}
