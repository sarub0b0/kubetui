pub mod color;
mod controller;
mod store;
pub mod worker;

use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use anyhow::Result;
use crossbeam::channel::{Receiver, Sender};
use k8s_openapi::api::core::v1::Namespace;
use kube::{api::ListParams, Api, ResourceExt};
use tokio::{runtime::Runtime, sync::RwLock};

use crate::{
    features::{
        api_resources::{kube::ApiResource, message::ApiMessage},
        config::message::ConfigMessage,
        context::message::ContextMessage,
        get::message::GetMessage,
        namespace::message::NamespaceMessage,
        network::message::NetworkMessage,
        pod::message::LogMessage,
        yaml::message::YamlMessage,
    },
    kube::{table::KubeTable, KubeClient},
    logger,
    message::Message,
    panic_set_hook,
};

use self::inner::Inner;

impl From<Kube> for Message {
    fn from(k: Kube) -> Self {
        Message::Kube(k)
    }
}

#[derive(Debug)]
pub enum Kube {
    Context(ContextMessage),
    Api(ApiMessage),
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
    Get(GetMessage),
}

pub type TargetNamespaces = Vec<String>;
pub type SharedTargetNamespaces = Arc<RwLock<TargetNamespaces>>;

pub type TargetApiResources = Vec<ApiResource>;
pub type SharedTargetApiResources = Arc<RwLock<TargetApiResources>>;

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
        features::{
            api_resources::kube::{ApiPoller, SharedApiResources},
            config::kube::ConfigPoller,
            event::kube::EventPoller,
            network::kube::NetworkPoller,
            pod::kube::PodPoller,
        },
        message::Message,
        workers::kube::{
            worker::{PollerBase, Worker},
            WorkerResult,
        },
    };

    use super::{
        controller::EventController,
        fetch_all_namespaces,
        store::{KubeState, KubeStore},
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

                let poller_base = PollerBase {
                    shared_target_namespaces: shared_target_namespaces.clone(),
                    tx: tx.clone(),
                    is_terminated: is_terminated.clone(),
                    kube_client: client.clone(),
                };

                let event_controller_handle = EventController::new(
                    poller_base.clone(),
                    rx.clone(),
                    kubeconfig
                        .contexts
                        .iter()
                        .map(|ctx| ctx.name.to_string())
                        .collect(),
                    shared_target_api_resources.clone(),
                    shared_api_resources.clone(),
                )
                .spawn();

                let pod_handle = PodPoller::new(poller_base.clone()).spawn();
                let config_handle = ConfigPoller::new(poller_base.clone()).spawn();
                let network_handle = NetworkPoller::new(poller_base.clone()).spawn();
                let event_handle = EventPoller::new(poller_base.clone()).spawn();
                let api_handle = ApiPoller::new(
                    poller_base.clone(),
                    shared_target_api_resources.clone(),
                    shared_api_resources,
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

                fn abort<T>(handlers: &[JoinHandle<T>]) {
                    for h in handlers {
                        h.abort()
                    }
                }

                while !handles.is_empty() {
                    let (result, _, vec) = select_all(handles).await;

                    handles = vec;

                    match result {
                        Ok(ret) => match ret {
                            WorkerResult::ChangedContext(ctx) => {
                                abort(&handles);

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
                            abort(&handles);
                            tx.send(Message::Error(anyhow!("KubeProcess Error: {:?}", e)))?;
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
