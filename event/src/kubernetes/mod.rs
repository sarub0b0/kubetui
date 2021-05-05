mod api_resources;
mod config;
mod context;
mod event;
mod log;
mod pod;
mod request;
mod v1_table;

use super::Event;
use config::{configs_loop, get_config};
use context::namespace_list;
use event::event_loop;
use log::log_stream;
use pod::pod_loop;

use std::sync::Arc;

use crossbeam::channel::{Receiver, Sender};
use tokio::{
    runtime::Runtime,
    sync::RwLock,
    task::{self, JoinHandle},
};

use kube::{
    config::{Kubeconfig, NamedContext},
    Client,
};

pub enum Kube {
    // apis
    GetAPIsRequest,
    GetAPIsResponse(Vec<String>),
    SetAPIsRequest(Vec<String>),
    APIsResults(Vec<Vec<String>>),
    // Context
    GetContextsRequest,
    GetContextsResponse(Vec<String>),
    SetContext(String),
    GetCurrentContextRequest,
    GetCurrentContextResponse(String, String), // current_context, namespace
    // Event
    Event(Vec<String>),
    // Namespace
    GetNamespacesRequest,
    GetNamespacesResponse(Vec<String>),
    SetNamespace(String),
    // Pod Status
    Pod(Vec<Vec<String>>),
    // Pod Logs
    LogStreamRequest(String),
    LogStreamResponse(Vec<String>),
    // ConfigMap & Secret
    Configs(Vec<String>),
    ConfigRequest(String),
    ConfigResponse(Vec<String>),
}

pub struct Handlers(Vec<JoinHandle<()>>);

impl Handlers {
    fn abort(&self) {
        self.0.iter().for_each(|j| j.abort());
    }
}

fn cluster_server_url(kubeconfig: &Kubeconfig, named_context: &NamedContext) -> String {
    let cluster_name = named_context.context.cluster.clone();

    let named_cluster = kubeconfig.clusters.iter().find(|n| n.name == cluster_name);

    named_cluster.as_ref().unwrap().cluster.server.clone()
}

pub fn kube_process(tx: Sender<Event>, rx: Receiver<Event>) {
    let rt = Runtime::new().unwrap();

    rt.block_on(async move {
        let kubeconfig = Kubeconfig::read().unwrap();
        let current_context = kubeconfig.current_context.clone().unwrap();

        let named_context = kubeconfig
            .contexts
            .iter()
            .find(|n| n.name == current_context);

        let namespace = Arc::new(RwLock::new(match named_context {
            Some(nc) => nc
                .context
                .namespace
                .clone()
                .unwrap_or_else(|| "default".to_string()),
            None => "default".to_string(),
        }));

        let server_url = cluster_server_url(&kubeconfig, named_context.as_ref().unwrap());

        let client = Client::try_default().await.unwrap();

        let main_loop = tokio::spawn(main_loop(
            rx,
            tx.clone(),
            client.clone(),
            Arc::clone(&namespace),
            current_context,
        ));

        let pod_loop = tokio::spawn(pod_loop(
            tx.clone(),
            client.clone(),
            Arc::clone(&namespace),
            server_url.clone(),
        ));

        let config_loop = tokio::spawn(configs_loop(
            tx.clone(),
            client.clone(),
            Arc::clone(&namespace),
        ));

        let event_loop = tokio::spawn(event_loop(
            tx.clone(),
            client.clone(),
            Arc::clone(&namespace),
            server_url,
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
    let mut log_stream_handler: Option<Handlers> = None;
    loop {
        let rx = rx.clone();
        let tx = tx.clone();
        let client = client.clone();

        if let Ok(recv) = tokio::task::spawn_blocking(move || rx.recv()).await {
            match recv {
                Ok(Event::Kube(ev)) => match ev {
                    Kube::SetNamespace(ns) => {
                        {
                            let mut namespace = namespace.write().await;
                            *namespace = ns;
                        }

                        if let Some(handler) = log_stream_handler {
                            handler.abort();
                            log_stream_handler = None;
                        }
                    }

                    Kube::GetNamespacesRequest => {
                        let res = namespace_list(client).await;
                        tx.send(Event::Kube(Kube::GetNamespacesResponse(res)))
                            .unwrap();
                    }

                    Kube::LogStreamRequest(pod_name) => {
                        if let Some(handler) = log_stream_handler {
                            handler.abort();
                        }

                        let ns = namespace.read().await;

                        log_stream_handler = Some(log_stream(tx, client, &ns, &pod_name).await);
                        task::yield_now().await;
                    }

                    Kube::ConfigRequest(config) => {
                        let ns = namespace.read().await;
                        let raw = get_config(client, &ns, &config).await;
                        tx.send(Event::Kube(Kube::ConfigResponse(raw))).unwrap();
                    }

                    Kube::GetCurrentContextRequest => {
                        let ns = namespace.read().await;
                        tx.send(Event::Kube(Kube::GetCurrentContextResponse(
                            current_context.to_string(),
                            ns.to_string(),
                        )))
                        .unwrap();
                    }
                    Kube::GetAPIsRequest => {
                        let apis = vec!["v1".to_string(), "test".to_string()];

                        tx.send(Event::Kube(Kube::GetAPIsResponse(apis))).unwrap();
                    }
                    _ => unreachable!(),
                },
                Ok(_) => unreachable!(),
                Err(_) => {}
            }
        }
    }
}
