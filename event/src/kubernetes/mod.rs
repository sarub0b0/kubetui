mod config;
mod context;
mod event;
mod log;
mod pod;

use self::event::event_loop;

use super::Event;
use crate::kubernetes::config::{configs_loop, get_config};
use crate::kubernetes::log::log_stream;
use crate::kubernetes::pod::pod_loop;

use std::sync::{Arc, RwLock};

use crossbeam::channel::{Receiver, Sender};
use tokio::{
    runtime::Runtime,
    task::{self, JoinHandle},
};

use k8s_openapi::api::core::v1::Namespace;

use kube::{
    api::{ListParams, Meta},
    config::Kubeconfig,
    Api, Client,
};

pub enum Kube {
    // Context
    GetCurrentContextRequest,
    GetCurrentContextResponse(String, String), // current_context, namespace
    // Event
    Event(Vec<String>),
    // Namespace
    GetNamespacesRequest,
    GetNamespacesResponse(Vec<String>),
    SetNamespace(String),
    // Pod Logs
    Pod(Vec<String>),
    LogStreamRequest(String),
    LogStreamResponse(Vec<String>),
    // ConfigMap & Secret
    Configs(Vec<String>),
    ConfigRequest(String),
    ConfigResponse(Vec<String>),
}

pub struct Handlers(JoinHandle<()>, JoinHandle<()>);

impl Handlers {
    fn abort(&self) {
        self.0.abort();
        self.1.abort();
    }
}

pub fn kube_process(tx: Sender<Event>, rx: Receiver<Event>) {
    let rt = Runtime::new().unwrap();

    rt.block_on(async move {
        let kubeconfig = Kubeconfig::read().unwrap();
        let current_context = kubeconfig.current_context.unwrap();

        let named_context = kubeconfig
            .contexts
            .iter()
            .find(|n| n.name == current_context);

        let namespace = Arc::new(RwLock::new(
            named_context.unwrap().clone().context.namespace.unwrap(),
        ));

        let client = Client::try_default().await.unwrap();

        let main_loop = tokio::spawn(main_loop(
            rx,
            tx.clone(),
            client.clone(),
            Arc::clone(&namespace),
            current_context,
        ));

        let pod_loop = tokio::spawn(pod_loop(tx.clone(), client.clone(), Arc::clone(&namespace)));

        let config_loop = tokio::spawn(configs_loop(
            tx.clone(),
            client.clone(),
            Arc::clone(&namespace),
        ));

        let event_loop = tokio::spawn(event_loop(
            tx.clone(),
            client.clone(),
            Arc::clone(&namespace),
        ));

        main_loop.await.unwrap();
        pod_loop.await.unwrap();
        config_loop.await.unwrap();
        event_loop.await.unwrap();
    });
}

async fn main_loop(
    rx: Receiver<Event>,
    tx: Sender<Event>,
    client: Client,
    namespace: Arc<RwLock<String>>,
    current_context: String,
) {
    let tx_ns = tx.clone();
    let tx_config = tx.clone();
    let tx_ctx = tx.clone();

    let mut log_stream_handler: Option<Handlers> = None;
    loop {
        match rx.recv() {
            Ok(Event::Kube(ev)) => match ev {
                Kube::SetNamespace(ns) => {
                    let selectd_ns = ns.clone();
                    let mut ns = namespace.write().unwrap();
                    *ns = selectd_ns;
                }

                Kube::GetNamespacesRequest => {
                    let res = namespace_list(client.clone()).await;
                    tx_ns
                        .send(Event::Kube(Kube::GetNamespacesResponse(res)))
                        .unwrap();
                }

                Kube::LogStreamRequest(pod_name) => {
                    if let Some(handler) = log_stream_handler {
                        handler.abort();
                    }

                    let ns = namespace.read().unwrap().clone();
                    log_stream_handler =
                        Some(log_stream(tx.clone(), client.clone(), ns, pod_name).await);
                    task::yield_now().await;
                }

                Kube::ConfigRequest(config) => {
                    let ns = namespace.read().unwrap().clone();
                    let raw = get_config(client.clone(), &ns, &config).await;
                    tx_config
                        .send(Event::Kube(Kube::ConfigResponse(raw)))
                        .unwrap();
                }

                Kube::GetCurrentContextRequest => {
                    let ns = namespace.read().unwrap().clone();
                    tx_ctx
                        .send(Event::Kube(Kube::GetCurrentContextResponse(
                            current_context.to_string(),
                            ns,
                        )))
                        .unwrap();
                }
                _ => unreachable!(),
            },
            Ok(_) => unreachable!(),
            Err(_) => {}
        }
    }
}

// async fn _log_stream_spawn(
//     tx: Sender<Event>,
//     client: Client,
//     ns: String,
//     pod_name: String,
// ) -> Option<Handlers> {
//     Some({
//         let pods: Api<Pod> = Api::namespaced(client.clone(), &ns);
//         let pod = pods.get(&pod_name).await.unwrap();
//         let phase = get_status(pod);

//         if phase == "Running" || phase == "Completed" {
//             log_stream(tx, client, ns, pod_name).await
//         } else {
//             event_watch(tx, client, ns, pod_name, "Pod").await
//         }
//     })
// }

async fn namespace_list(client: Client) -> Vec<String> {
    let namespaces: Api<Namespace> = Api::all(client);
    let lp = ListParams::default();
    let ns_list = namespaces.list(&lp).await.unwrap();

    ns_list.iter().map(|ns| ns.name()).collect()
}
