use std::time::Duration;

use async_trait::async_trait;
use crossbeam::channel::Receiver;
use tokio::task::{self, AbortHandle};

use crate::{
    features::{
        api_resources::{
            kube::SharedApiResources,
            message::{ApiMessage, ApiRequest, ApiResponse},
        },
        config::{kube::ConfigsDataWorker, message::ConfigMessage},
        context::message::{ContextMessage, ContextRequest, ContextResponse},
        get::{kube::yaml::GetYamlWorker, message::GetMessage},
        namespace::message::{NamespaceMessage, NamespaceRequest, NamespaceResponse},
        network::{kube::NetworkDescriptionWorker, message::NetworkMessage},
        pod::{kube::LogWorker, message::LogMessage},
        yaml::{
            kube::{FetchResourceList, YamlWorker},
            message::{YamlMessage, YamlRequest, YamlResponse},
        },
    },
    message::Message,
};

use super::{
    fetch_all_namespaces,
    worker::{AbortWorker as _, PollerBase, Worker},
    Kube, SharedTargetApiResources, WorkerResult,
};

#[derive(Clone)]
pub struct EventController {
    base: PollerBase,
    rx: Receiver<Message>,
    contexts: Vec<String>,
    shared_target_api_resources: SharedTargetApiResources,
    shared_api_resources: SharedApiResources,
}

impl EventController {
    pub fn new(
        base: PollerBase,
        rx: Receiver<Message>,
        contexts: Vec<String>,
        shared_target_api_resources: SharedTargetApiResources,
        shared_api_resources: SharedApiResources,
    ) -> Self {
        Self {
            base,
            rx,
            contexts,
            shared_target_api_resources,
            shared_api_resources,
        }
    }
}

#[async_trait]
impl Worker for EventController {
    type Output = WorkerResult;

    async fn run(&self) -> Self::Output {
        let mut log_handler: Option<AbortHandle> = None;
        let mut config_handler: Option<AbortHandle> = None;
        let mut network_handler: Option<AbortHandle> = None;
        let mut yaml_handler: Option<AbortHandle> = None;
        let mut get_handler: Option<AbortHandle> = None;

        let EventController {
            base: poll_worker,
            rx,
            contexts,
            shared_target_api_resources,
            shared_api_resources,
        } = self;

        let PollerBase {
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

                            if let Some(handler) = get_handler {
                                handler.abort();
                                get_handler = None;
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

                    Kube::Api(ApiMessage::Request(req)) => {
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
                            Yaml(req) => {
                                if let Some(handler) = yaml_handler {
                                    handler.abort();
                                }

                                yaml_handler = Some(
                                    YamlWorker::new(
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
                        }
                    }

                    Kube::Get(GetMessage::Request(req)) => {
                        if let Some(handler) = get_handler {
                            handler.abort();
                        }

                        get_handler = Some(
                            GetYamlWorker::new(is_terminated.clone(), tx, kube_client.clone(), req)
                                .spawn(),
                        );
                        task::yield_now().await;
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
