pub mod api_resources;
mod client;
mod color;
pub mod config;
mod event;
pub mod log;
mod metric_type;
pub mod network;
mod pod;
mod v1_table;
mod worker;
pub mod yaml;

use super::Event;

use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use anyhow::{bail, Result};
use async_trait::async_trait;
use crossbeam::channel::{Receiver, Sender};
use k8s_openapi::api::core::v1::Namespace;
use kube::{api::ListParams, Api, ResourceExt};
use tokio::{
    runtime::Runtime,
    sync::RwLock,
    task::{self, JoinHandle},
};

use crate::{logger, panic_set_hook};

use self::{
    api_resources::{
        apis_list_from_api_database, ApiDatabase, ApiMessage, ApiRequest, ApiResponse,
    },
    client::KubeClient,
    config::{ConfigMessage, ConfigsDataWorker},
    context_message::{ContextMessage, ContextRequest, ContextResponse},
    inner::Inner,
    log::{LogStreamMessage, LogWorkerBuilder},
    namespace_message::{NamespaceMessage, NamespaceRequest, NamespaceResponse},
    network::{NetworkDescriptionWorker, NetworkMessage},
    worker::{PollWorker, Worker},
    yaml::{
        fetch_resource_list::FetchResourceList,
        worker::{YamlWorker, YamlWorkerRequest},
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

impl From<Kube> for Event {
    fn from(k: Kube) -> Self {
        Event::Kube(k)
    }
}

#[derive(Debug)]
pub enum Kube {
    Context(ContextMessage),
    API(ApiMessage),
    RestoreAPIs(Vec<String>),
    RestoreContext {
        context: String,
        namespaces: Vec<String>,
    },
    Event(Result<Vec<String>>),
    Namespace(NamespaceMessage),
    Pod(Result<KubeTable>),
    LogStream(LogStreamMessage),
    Config(ConfigMessage),
    Network(NetworkMessage),
    Yaml(YamlMessage),
}

pub mod namespace_message {
    use crate::event::Event;

    use super::Kube;

    #[derive(Debug)]
    pub enum NamespaceMessage {
        Request(NamespaceRequest),
        Response(NamespaceResponse),
    }

    #[derive(Debug)]
    pub enum NamespaceRequest {
        Get,
        Set(Vec<String>),
    }

    #[derive(Debug)]
    pub enum NamespaceResponse {
        Get(Vec<String>),
        Set(Vec<String>),
    }

    impl From<NamespaceRequest> for Event {
        fn from(n: NamespaceRequest) -> Self {
            Event::Kube(Kube::Namespace(NamespaceMessage::Request(n)))
        }
    }

    impl From<NamespaceResponse> for Event {
        fn from(n: NamespaceResponse) -> Self {
            Event::Kube(Kube::Namespace(NamespaceMessage::Response(n)))
        }
    }
}

pub mod context_message {
    use super::Kube;
    use crate::event::Event;

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

    impl From<ContextMessage> for Event {
        fn from(m: ContextMessage) -> Self {
            Event::Kube(Kube::Context(m))
        }
    }

    impl From<ContextRequest> for Event {
        fn from(m: ContextRequest) -> Self {
            Event::Kube(Kube::Context(ContextMessage::Request(m)))
        }
    }

    impl From<ContextResponse> for Event {
        fn from(m: ContextResponse) -> Self {
            Event::Kube(Kube::Context(ContextMessage::Response(m)))
        }
    }
}

#[derive(Default, Debug)]
pub struct Handlers(Vec<JoinHandle<Result<()>>>);

impl Handlers {
    fn abort(&self) {
        self.0.iter().for_each(|j| j.abort());
    }
}

pub(super) type Namespaces = Arc<RwLock<Vec<String>>>;
pub(super) type ApiResources = Arc<RwLock<Vec<String>>>;

#[derive(Clone)]
pub enum WorkerResult {
    ChangedContext(String),
    Terminated,
}

async fn namespace_list(client: KubeClient) -> Vec<String> {
    let namespaces: Api<Namespace> = Api::all(client.as_client().clone());
    let lp = ListParams::default();
    let ns_list = namespaces.list(&lp).await.unwrap();

    ns_list.iter().map(|ns| ns.name_any()).collect()
}

#[derive(Debug, Default, Clone)]
pub struct KubeWorkerConfig {
    pub kubeconfig: Option<PathBuf>,
    pub namespaces: Option<Vec<String>>,
    pub context: Option<String>,
    pub all_namespaces: bool,
}

#[derive(Debug, Clone)]
pub struct KubeWorker {
    pub(super) tx: Sender<Event>,
    pub(super) rx: Receiver<Event>,
    pub(super) is_terminated: Arc<AtomicBool>,
    pub(super) config: KubeWorkerConfig,
}

impl KubeWorker {
    pub fn new(
        tx: Sender<Event>,
        rx: Receiver<Event>,
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

    async fn inner(worker: KubeWorker) -> Result<()> {
        let inner = Inner::try_from(worker).await?;
        inner.run().await
    }

    pub fn run(&self) -> Result<()> {
        logger!(info, "Start tick event");

        let is_terminated_panic = self.is_terminated.clone();
        panic_set_hook!({
            is_terminated_panic.store(true, std::sync::atomic::Ordering::Relaxed);
        });

        let ret: Result<()> = match Runtime::new() {
            Ok(rt) => match rt.block_on(Self::inner(self.clone())) {
                Ok(_) => Ok(()),
                Err(e) => {
                    self.is_terminated.store(true, Ordering::Relaxed);

                    bail!("{}", e)
                }
            },
            Err(e) => {
                self.is_terminated
                    .store(true, std::sync::atomic::Ordering::Relaxed);
                bail!("failed to create runtime: {}", e)
            }
        };

        logger!(info, "Terminated tick event");

        if let Err(e) = ret {
            self.is_terminated
                .store(true, std::sync::atomic::Ordering::Relaxed);
            bail!("{:?}", e)
        } else {
            Ok(())
        }
    }
}

#[derive(Clone)]
struct MainWorker {
    inner: PollWorker,
    rx: Receiver<Event>,
    contexts: Vec<String>,
    api_resources: ApiResources,
    api_database: ApiDatabase,
}

#[async_trait]
impl Worker for MainWorker {
    type Output = Result<WorkerResult>;

    async fn run(&self) -> Self::Output {
        let mut log_stream_handler: Option<Handlers> = None;
        let mut config_handler: Option<JoinHandle<Result<()>>> = None;
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

            let Ok(recv) = task.await else { continue };

            match recv {
                Ok(Event::Kube(ev)) => match ev {
                    Kube::Namespace(NamespaceMessage::Request(req)) => match req {
                        NamespaceRequest::Get => {
                            let ns = namespace_list(kube_client.clone()).await;
                            tx.send(NamespaceResponse::Get(ns).into())?;
                        }
                        NamespaceRequest::Set(req) => {
                            {
                                let mut namespace = namespaces.write().await;
                                *namespace = req.clone();
                            }

                            if let Some(handler) = log_stream_handler {
                                handler.abort();
                                log_stream_handler = None;
                            }

                            if let Some(h) = config_handler {
                                h.abort();
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

                            tx.send(NamespaceResponse::Set(req).into())?;
                        }
                    },

                    Kube::LogStream(LogStreamMessage::Request { namespace, name }) => {
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
                                let db = api_database.read().await;
                                let apis = apis_list_from_api_database(&db);
                                tx.send(ApiResponse::Get(Ok(apis)).into())?;
                            }
                            Set(req) => {
                                let mut api_resources = api_resources.write().await;
                                *api_resources = req.clone();
                                // tx.send(ApiResponse::Get(Ok(req.clone())).into())?;
                            }
                        }
                    }

                    Kube::Context(ContextMessage::Request(req)) => match req {
                        ContextRequest::Get => {
                            tx.send(ContextResponse::Get(contexts.to_vec()).into())?
                        }
                        ContextRequest::Set(req) => {
                            if let Some(h) = log_stream_handler {
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

                            return Ok(WorkerResult::ChangedContext(req));
                        }
                    },

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

        Ok(WorkerResult::Terminated)
    }
}

mod inner {
    use std::{
        collections::HashMap,
        path::PathBuf,
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        },
    };

    use anyhow::{anyhow, Result};
    use crossbeam::channel::{Receiver, Sender};
    use futures::future::select_all;
    use k8s_openapi::api::core::v1::Namespace;
    use kube::{
        api::ListParams,
        config::{Kubeconfig, KubeconfigError},
        Api, ResourceExt,
    };
    use tokio::{sync::RwLock, task::JoinHandle};

    use crate::{
        error::Error,
        event::{
            kubernetes::{
                api_resources::ApiPollWorker,
                config::ConfigsPollWorker,
                event::EventPollWorker,
                network::NetworkPollWorker,
                pod::PodPollWorker,
                worker::{PollWorker, Worker},
                MainWorker, WorkerResult,
            },
            Event,
        },
    };

    use super::{
        kube_store::{KubeState, KubeStore},
        Kube, {KubeWorker, KubeWorkerConfig},
    };

    pub struct Inner {
        tx: Sender<Event>,
        rx: Receiver<Event>,
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
                namespaces,
                context,
                all_namespaces,
            } = config;

            let kubeconfig = read_kubeconfig(kubeconfig)?;

            let Context { inner: context } = Context::try_from(&kubeconfig, context)?;

            let mut store = KubeStore::try_from_kubeconfig(kubeconfig.clone()).await?;

            let KubeState {
                client: state_client,
                namespaces: state_namespaces,
                ..
            } = store.get_mut(&context)?;

            if let Some(namespaces) = namespaces {
                *state_namespaces = namespaces;
            }

            if all_namespaces {
                let api: Api<Namespace> = Api::all(state_client.as_client().clone());

                let lp = ListParams::default();
                let list = api.list(&lp).await?;

                let vec: Vec<String> = list.iter().map(|ns| ns.name_any()).collect();

                *state_namespaces = vec;
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
                    namespaces,
                    api_resources,
                } = store.get(&context)?.clone();

                tx.send(Event::Kube(Kube::RestoreContext {
                    context: context.to_string(),
                    namespaces: namespaces.to_vec(),
                }))?;

                tx.send(Event::Kube(Kube::RestoreAPIs(api_resources.to_vec())))?;

                let shared_namespaces = Arc::new(RwLock::new(namespaces.to_vec()));
                let shared_api_resources = Arc::new(RwLock::new(api_resources.to_vec()));
                let shared_api_database = Arc::new(RwLock::new(HashMap::new()));

                let poll_worker = PollWorker {
                    namespaces: shared_namespaces.clone(),
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

                                    let namespaces = shared_namespaces.read().await;
                                    let api_resources = shared_api_resources.read().await;

                                    store.insert(
                                        context.to_string(),
                                        KubeState::new(
                                            client.clone(),
                                            namespaces.to_vec(),
                                            api_resources.to_vec(),
                                        ),
                                    );

                                    context = ctx;
                                }
                                WorkerResult::Terminated => {}
                            },
                            Err(_) => {
                                tx.send(Event::Error(Error::Raw("KubeProcess Error".to_string())))?
                            }
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
    use std::{collections::HashMap, fmt::Debug};

    use anyhow::{anyhow, Result};
    use futures::future::try_join_all;
    use kube::{
        config::{KubeConfigOptions, Kubeconfig},
        Client, Config,
    };

    use super::client::KubeClient;

    pub type Context = String;

    #[derive(Clone)]
    pub struct KubeState {
        pub client: KubeClient,
        pub namespaces: Vec<String>,
        pub api_resources: Vec<String>,
    }

    impl KubeState {
        pub fn new(
            client: KubeClient,
            namespaces: Vec<String>,
            api_resources: Vec<String>,
        ) -> Self {
            Self {
                client,
                namespaces,
                api_resources,
            }
        }
    }

    #[derive(Debug)]
    #[cfg_attr(test, derive(PartialEq))]
    pub struct KubeStore {
        inner: HashMap<Context, KubeState>,
    }

    impl From<HashMap<Context, KubeState>> for KubeStore {
        fn from(inner: HashMap<Context, KubeState>) -> Self {
            KubeStore { inner }
        }
    }

    impl std::fmt::Debug for KubeState {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "KubeStore {{ client: _, namespaces: {:?}, api_resources: {:?} }}",
                self.namespaces, self.api_resources
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
                        let Some(kube::config::Context{ref user, ..}) = context.context else {return None};

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
                    let namespace = config.default_namespace.to_string();

                    let client = Client::try_from(config)?;

                    let kube_client = KubeClient::new(client, cluster_url);

                    anyhow::Ok((
                        context.name.to_string(),
                        KubeState {
                            client: kube_client,
                            namespaces: vec![namespace],
                            api_resources: vec![],
                        },
                    ))
                }))
                .await?;

            let inner: HashMap<Context, KubeState> = jobs.into_iter().collect();

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
                self.namespaces == rhs.namespaces
                    && self.api_resources == rhs.api_resources
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

            let expected = HashMap::from([
                (
                    "cluster-1".to_string(),
                    KubeState {
                        client: KubeClient::new(client.clone(), "https://192.168.0.1/"),
                        namespaces: vec!["ns-1".to_string()],
                        api_resources: Default::default(),
                    },
                ),
                (
                    "cluster-2".to_string(),
                    KubeState {
                        client: KubeClient::new(client.clone(), "https://192.168.0.2/"),
                        namespaces: vec!["ns-2".to_string()],
                        api_resources: Default::default(),
                    },
                ),
                (
                    "cluster-3".to_string(),
                    KubeState {
                        client: KubeClient::new(client, "https://192.168.0.3/"),
                        namespaces: vec!["default".to_string()],
                        api_resources: Default::default(),
                    },
                ),
            ])
            .into();

            assert_eq!(actual, expected);
        }
    }
}
