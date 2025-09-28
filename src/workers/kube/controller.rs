use std::{sync::Arc, time::Duration};

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use crossbeam::channel::{Receiver, Sender};
use futures::future::select_all;
use k8s_openapi::api::core::v1::Namespace;
use kube::{api::ListParams, config::Kubeconfig, Api, ResourceExt as _};
use ratatui::style::{Color, Style};
use tokio::{
    sync::RwLock,
    task::{self, AbortHandle, JoinHandle},
};

use crate::{
    features::{
        api_resources::{
            kube::{ApiConfig, ApiPoller, ApiResource, ApiResources, SharedApiResources},
            message::{ApiMessage, ApiRequest, ApiResponse},
        },
        config::{
            kube::{ConfigPoller, ConfigsDataWorker},
            message::ConfigMessage,
        },
        context::message::{ContextMessage, ContextRequest, ContextResponse},
        event::kube::{EventConfig, EventPoller},
        get::{kube::yaml::GetYamlWorker, message::GetMessage},
        namespace::message::{NamespaceMessage, NamespaceRequest, NamespaceResponse},
        network::{
            kube::{NetworkDescriptionWorker, NetworkPoller},
            message::NetworkMessage,
        },
        pod::{
            kube::{LogConfig, LogWorker, PodConfig, PodPoller},
            message::{LogMessage, PodMessage},
            PodColumns,
        },
        yaml::{
            kube::{FetchResourceList, YamlWorker},
            message::{YamlMessage, YamlRequest, YamlResponse},
        },
        StyledApiResource,
    },
    kube::KubeClient,
    logger,
    message::Message,
    workers::kube::message::Kube,
};

use super::{
    config::{read_kubeconfig, Context, KubeWorkerConfig},
    store::{KubeState, KubeStore},
    worker::Worker,
    AbortWorker as _,
};

pub type TargetNamespaces = Vec<String>;
pub type SharedTargetNamespaces = Arc<RwLock<TargetNamespaces>>;

pub type TargetApiResources = Vec<ApiResource>;
pub type SharedTargetApiResources = Arc<RwLock<TargetApiResources>>;

pub type StyledTargetApiResources = Vec<StyledApiResource>;

pub type SharedPodColumns = Arc<RwLock<PodColumns>>;

/// APIタブのダイアログで表示されるAPIリソースのスタイル設定
#[derive(Debug, Clone)]
pub struct ApisConfig {
    pub preferred_version_or_latest: Style,
    pub other_version: Style,
}

impl Default for ApisConfig {
    fn default() -> Self {
        Self {
            preferred_version_or_latest: Style::default(),
            other_version: Style::default().fg(Color::DarkGray),
        }
    }
}

/// Yamlタブのダイアログで表示されるAPIリソースのスタイル設定
#[derive(Debug, Clone)]
pub struct YamlConfig {
    pub preferred_version_or_latest: Style,
    pub other_version: Style,
}

impl Default for YamlConfig {
    fn default() -> Self {
        Self {
            preferred_version_or_latest: Style::default(),
            other_version: Style::default().fg(Color::DarkGray),
        }
    }
}

// target_api_resourcesとapis_configからStyledTargetApiResourcesを生成する
pub fn styled_target_api_resources(
    target_api_resources: &TargetApiResources,
    preferred_version_or_latest: Style,
    other_version: Style,
) -> StyledTargetApiResources {
    target_api_resources
        .iter()
        .map(|api| {
            let style = if api.is_api() || api.is_preferred_version() {
                preferred_version_or_latest
            } else {
                other_version
            };

            StyledApiResource::new(api.clone(), style)
        })
        .collect()
}

async fn fetch_all_namespaces(client: KubeClient) -> Result<Vec<String>> {
    let namespaces: Api<Namespace> = Api::all(client.as_client().clone());
    let lp = ListParams::default();
    let ns_list = namespaces.list(&lp).await?;

    Ok(ns_list.iter().map(|ns| ns.name_any()).collect())
}

#[derive(Clone)]
pub enum WorkerResult {
    ChangedContext(String),
}

pub struct KubeController {
    tx: Sender<Message>,
    rx: Receiver<Message>,
    kubeconfig: Kubeconfig,
    context: String,
    store: KubeStore,
    pod_config: PodConfig,
    event_config: EventConfig,
    api_config: ApiConfig,
    apis_config: ApisConfig,
    yaml_config: YamlConfig,
}

impl KubeController {
    pub async fn new(
        tx: Sender<Message>,
        rx: Receiver<Message>,
        config: KubeWorkerConfig,
    ) -> Result<Self> {
        let KubeWorkerConfig {
            kubeconfig,
            target_namespaces,
            context,
            all_namespaces,
            pod_config,
            event_config,
            api_config,
            apis_config,
            yaml_config,
        } = config;

        let kubeconfig = read_kubeconfig(kubeconfig)?;

        let context = Context::try_from(&kubeconfig, context)?;

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
            kubeconfig,
            context: context.to_string(),
            store,
            pod_config,
            event_config,
            api_config,
            apis_config,
            yaml_config,
        })
    }

    pub async fn run(self) -> Result<()> {
        let Self {
            tx,
            rx,
            kubeconfig,
            mut context,
            mut store,
            pod_config,
            event_config,
            api_config,
            apis_config,
            yaml_config,
        } = self;

        loop {
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
                styled_target_api_resources(
                    &target_api_resources,
                    apis_config.preferred_version_or_latest,
                    apis_config.other_version,
                ),
            )))?;

            let shared_target_namespaces = Arc::new(RwLock::new(target_namespaces.to_vec()));
            let shared_target_api_resources = Arc::new(RwLock::new(target_api_resources.to_vec()));
            let shared_api_resources = ApiResources::shared();
            let shared_pod_columns = Arc::new(RwLock::new(
                pod_config.default_columns.clone().unwrap_or_default(),
            ));

            let contexts = kubeconfig
                .contexts
                .iter()
                .map(|ctx| ctx.name.to_string())
                .collect();

            let event_controller_args = EventControllerArgs {
                shared_target_namespaces: shared_target_namespaces.clone(),
                kube_client: client.clone(),
                tx: tx.clone(),
                rx: rx.clone(),
                contexts,
                shared_target_api_resources: shared_target_api_resources.clone(),
                shared_api_resources: shared_api_resources.clone(),
                shared_pod_columns: shared_pod_columns.clone(),
                apis_config: apis_config.clone(),
                yaml_config: yaml_config.clone(),
            };

            let event_controller_handle = EventController::new(event_controller_args).spawn();

            let pod_handle = PodPoller::new(
                tx.clone(),
                shared_target_namespaces.clone(),
                shared_pod_columns.clone(),
                client.clone(),
                pod_config.clone(),
            )
            .spawn();

            let config_handle =
                ConfigPoller::new(tx.clone(), shared_target_namespaces.clone(), client.clone())
                    .spawn();

            let network_handle = NetworkPoller::new(
                tx.clone(),
                shared_target_namespaces.clone(),
                client.clone(),
                shared_api_resources.clone(),
            )
            .spawn();

            let event_handle = EventPoller::new(
                tx.clone(),
                shared_target_namespaces.clone(),
                client.clone(),
                event_config.clone(),
            )
            .spawn();

            let api_handle = ApiPoller::new(
                tx.clone(),
                shared_target_namespaces.clone(),
                client.clone(),
                shared_target_api_resources.clone(),
                shared_api_resources,
                api_config.clone(),
            )
            .spawn();

            let mut handles = vec![
                event_controller_handle,
                pod_handle,
                config_handle,
                network_handle,
                event_handle,
                api_handle,
            ];

            while !handles.is_empty() {
                let (result, _, vec) = select_all(handles).await;

                handles = vec;

                match result {
                    Ok(ret) => match ret {
                        WorkerResult::ChangedContext(ctx) => {
                            Self::abort(&handles);

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
                    },
                    Err(e) => {
                        Self::abort(&handles);
                        tx.send(Message::Error(anyhow!("KubeProcess Error: {:?}", e)))?;
                    }
                }
            }
        }
    }

    fn abort<T>(handlers: &[JoinHandle<T>]) {
        for h in handlers {
            h.abort()
        }
    }
}

struct EventControllerArgs {
    shared_target_namespaces: SharedTargetNamespaces,
    kube_client: KubeClient,
    tx: Sender<Message>,
    rx: Receiver<Message>,
    contexts: Vec<String>,
    shared_target_api_resources: SharedTargetApiResources,
    shared_api_resources: SharedApiResources,
    shared_pod_columns: SharedPodColumns,
    apis_config: ApisConfig,
    yaml_config: YamlConfig,
}

#[derive(Clone)]
struct EventController {
    shared_target_namespaces: SharedTargetNamespaces,
    kube_client: KubeClient,
    tx: Sender<Message>,
    rx: Receiver<Message>,
    contexts: Vec<String>,
    shared_target_api_resources: SharedTargetApiResources,
    shared_api_resources: SharedApiResources,
    shared_pod_columns: SharedPodColumns,
    apis_config: ApisConfig,
    yaml_config: YamlConfig,
}

impl EventController {
    fn new(args: EventControllerArgs) -> Self {
        Self {
            shared_target_namespaces: args.shared_target_namespaces,
            kube_client: args.kube_client,
            tx: args.tx,
            rx: args.rx,
            contexts: args.contexts,
            shared_target_api_resources: args.shared_target_api_resources,
            shared_api_resources: args.shared_api_resources,
            shared_pod_columns: args.shared_pod_columns,
            apis_config: args.apis_config,
            yaml_config: args.yaml_config,
        }
    }
}

struct LogHandle {
    handler: AbortHandle,
    config: LogConfig,
}

impl LogHandle {
    fn abort(&self) {
        self.handler.abort();
    }

    fn toggle_json_pretty_print(&mut self, tx: Sender<Message>, client: KubeClient) {
        self.abort();

        self.config.json_pretty_print = !self.config.json_pretty_print;

        self.handler = LogWorker::new(tx, client, self.config.clone()).spawn();
    }
}

#[async_trait]
impl Worker for EventController {
    type Output = WorkerResult;

    async fn run(&self) -> Self::Output {
        let mut log_handler: Option<LogHandle> = None;
        let mut config_handler: Option<AbortHandle> = None;
        let mut network_handler: Option<AbortHandle> = None;
        let mut yaml_handler: Option<AbortHandle> = None;
        let mut get_handler: Option<AbortHandle> = None;

        let EventController {
            shared_target_namespaces,
            kube_client,
            tx,
            rx,
            contexts,
            shared_target_api_resources,
            shared_api_resources,
            shared_pod_columns,
            apis_config,
            yaml_config,
        } = self;

        loop {
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

                            if let Some(handler) = get_handler {
                                handler.abort();
                                get_handler = None;
                            }

                            tx.send(NamespaceResponse::Set(req).into())
                                .expect("Failed to send NamespaceResponse:Set");
                        }
                    },

                    Kube::Pod(PodMessage::Request(req)) => {
                        let mut pod_columns = shared_pod_columns.write().await;
                        *pod_columns = req;

                        logger!(info, "Pod columns updated: {:#?}", pod_columns);
                    }

                    Kube::Log(LogMessage::Request(req)) => {
                        if let Some(handler) = log_handler {
                            handler.abort();
                        }

                        let abort_handle =
                            LogWorker::new(tx, kube_client.clone(), req.clone()).spawn();

                        log_handler = Some(LogHandle {
                            handler: abort_handle,
                            config: req,
                        });

                        task::yield_now().await;
                    }

                    Kube::Log(LogMessage::ToggleJsonPrettyPrint) => {
                        if let Some(ref mut handler) = log_handler {
                            handler.toggle_json_pretty_print(tx.clone(), kube_client.clone());
                            task::yield_now().await;
                        }
                    }

                    Kube::Config(ConfigMessage::Request(req)) => {
                        if let Some(handler) = config_handler {
                            handler.abort();
                        }

                        config_handler =
                            Some(ConfigsDataWorker::new(tx, kube_client.clone(), req).spawn());

                        task::yield_now().await;
                    }

                    Kube::Api(ApiMessage::Request(req)) => {
                        use ApiRequest::*;
                        match req {
                            Get => {
                                let api_resources = shared_api_resources.read().await;
                                let styled_api_resources = styled_target_api_resources(
                                    &api_resources,
                                    apis_config.preferred_version_or_latest,
                                    apis_config.other_version,
                                );
                                tx.send(ApiResponse::Get(Ok(styled_api_resources)).into())
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

                            if let Some(h) = get_handler {
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

                                let ret = styled_target_api_resources(
                                    &api_resources,
                                    yaml_config.preferred_version_or_latest,
                                    yaml_config.other_version,
                                );

                                logger!(info, "APIs: {:#?}", ret);

                                tx.send(YamlResponse::APIs(Ok(ret)).into())
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
                            Yaml(req) => {
                                if let Some(handler) = yaml_handler {
                                    handler.abort();
                                }

                                yaml_handler = Some(
                                    YamlWorker::new(
                                        tx,
                                        kube_client.clone(),
                                        shared_api_resources.clone(),
                                        req,
                                    )
                                    .spawn(),
                                );

                                task::yield_now().await;
                            }
                        }
                    }

                    Kube::Get(GetMessage::Request(req)) => {
                        if let Some(handler) = get_handler {
                            handler.abort();
                        }

                        get_handler =
                            Some(GetYamlWorker::new(tx, kube_client.clone(), req).spawn());

                        task::yield_now().await;
                    }

                    Kube::Network(NetworkMessage::Request(req)) => {
                        if let Some(handler) = network_handler {
                            handler.abort();
                        }

                        network_handler = Some(
                            NetworkDescriptionWorker::new(
                                tx,
                                kube_client.clone(),
                                req,
                                shared_api_resources.clone(),
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

                assert_eq!(context.as_str(), "cluster-1");
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

                assert_eq!(context.as_str(), "cluster-2");
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

                assert_eq!(context.as_str(), "cluster-1");
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
