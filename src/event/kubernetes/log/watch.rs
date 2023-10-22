use std::{
    collections::HashMap,
    fmt::Display,
    ops::{Deref, DerefMut},
};

use anyhow::anyhow;
use async_trait::async_trait;
use crossbeam::channel::Sender;
use futures::{StreamExt, TryStreamExt};
use k8s_openapi::api::core::v1::{ContainerStatus, Pod};
use kube::{api::WatchParams, core::WatchEvent, Api, ResourceExt};
use tokio::task::JoinHandle;

use crate::{
    event::{
        kubernetes::{client::KubeClient, worker::Worker},
        Event,
    },
    logger, send_response,
};

use super::{
    log_collector::LogBuffer,
    log_stream::{ContainerLogStreamer, ContainerLogStreamerOptions, COLOR_LIST},
};

#[derive(Clone)]
pub struct PodWatcher {
    tx: Sender<Event>,
    client: KubeClient,
    log_buffer: LogBuffer,
    namespace: String,
    pod_name: String,
}

impl PodWatcher {
    pub fn new(
        tx: Sender<Event>,
        client: KubeClient,
        message_buffer: LogBuffer,
        namespace: String,
        target: String,
    ) -> Self {
        Self {
            tx,
            client,
            log_buffer: message_buffer,
            namespace,
            pod_name: target,
        }
    }
}

#[async_trait]
impl Worker for PodWatcher {
    type Output = ();
    async fn run(&self) -> Self::Output {
        let lp = WatchParams::default().timeout(180);

        let api: Api<Pod> = Api::namespaced(self.client.to_client(), &self.namespace);

        let mut task_controller = TaskController::new(
            self.client.clone(),
            self.log_buffer.clone(),
            self.namespace.clone(),
            self.pod_name.clone(),
        );

        loop {
            let Ok(stream) = api.watch(&lp, "0").await else {
                continue;
            };

            let mut stream = stream.boxed();

            while let Ok(Some(status)) = stream.try_next().await {
                use WatchEvent::*;

                match status {
                    Added(pod) | Modified(pod) => {
                        let Some(name) = &pod.metadata.name else {
                            continue;
                        };

                        if name != &self.pod_name {
                            continue;
                        }

                        task_controller.spawn_tasks(&pod);
                    }
                    Deleted(pod) => {
                        task_controller.abort_tasks(&pod);
                    }
                    Bookmark(_) => {}
                    Error(err) => {
                        send_response!(self.tx, Err(anyhow!(err)));
                    }
                }
            }
        }
    }
}

struct TaskController {
    client: KubeClient,
    log_buffer: LogBuffer,
    namespace: String,
    pod_name: String,
    tasks: Tasks,
}

impl TaskController {
    fn new(client: KubeClient, log_buffer: LogBuffer, namespace: String, pod_name: String) -> Self {
        Self {
            client,
            log_buffer,
            namespace,
            pod_name,
            tasks: Tasks::default(),
        }
    }

    fn spawn_tasks(&mut self, pod: &Pod) {
        let Some(pod_uid) = pod.uid() else {
            logger!(error, "Not found pod UID {}", pod.name_any());
            return;
        };

        // コンテナステータスを集約
        let container_statuses = Self::aggregate_container_statuses(pod);

        // コンテナごとにタスク生成
        for status in container_statuses {
            let task_id = TaskId {
                pod_name: self.pod_name.clone(),
                container_name: status.name.clone(),
            };

            let Some(container_id) = Self::is_container_log_available(&status) else {
                logger!(info, "Container ID is empty. task_id={}", task_id,);
                continue;
            };

            // すでにタスク生成されている場合はスキップ
            if let Some(state) = self.tasks.get(&task_id) {
                if state.container_id == container_id {
                    logger!(
                        info,
                        "Container ID is the same. task_id={} container_id={}",
                        task_id,
                        container_id
                    );
                    continue;
                }

                logger!(
                    info,
                    "Container ID was chaned. task_id={} container_id={}->{}",
                    task_id,
                    state.container_id,
                    container_id
                );
            }

            let options = ContainerLogStreamerOptions {
                color: COLOR_LIST[self.tasks.len() % COLOR_LIST.len()],
            };

            let task = ContainerLogStreamer::new(
                self.client.clone(),
                self.namespace.clone(),
                self.pod_name.clone(),
                status.name.clone(),
                self.log_buffer.clone(),
                options,
            )
            .spawn();

            let task_state = TaskState {
                handler: task,
                pod_name: self.pod_name.clone(),
                pod_uid: pod_uid.to_string(),
                container_name: status.name.clone(),
                container_id: container_id.clone(),
            };

            logger!(
                info,
                "task start: pod_name={} pod_uid={} container_name={} container_id={}",
                task_state.pod_name,
                task_state.pod_uid,
                task_state.container_name,
                task_state.container_id
            );

            self.tasks.insert(task_id, task_state);
        }
    }

    fn abort_tasks(&mut self, pod: &Pod) {
        if let Some(pod_uid) = pod.uid() {
            self.tasks.abort_with_pod_uid(&pod_uid);
        } else {
            self.tasks.abort_with_pod_name(&self.pod_name);

            logger!(error, "Not found pod UID {}", pod.name_any());
        }
    }

    fn aggregate_container_statuses(pod: &Pod) -> Vec<ContainerStatus> {
        pod.status.as_ref().map_or(Vec::default(), |status| {
            let init_container_statuses = status.init_container_statuses.iter().flatten().cloned();

            let ephemeral_container_statuses = status
                .ephemeral_container_statuses
                .iter()
                .flatten()
                .cloned();

            let container_statuses = status.container_statuses.iter().flatten().cloned();

            init_container_statuses
                .chain(ephemeral_container_statuses)
                .chain(container_statuses)
                .collect()
        })
    }

    fn is_container_log_available(status: &ContainerStatus) -> Option<String> {
        let last_state = &status.last_state;

        let Some(state) = &status.state else {
            return None;
        };

        if state.running.is_some() {
            return status.container_id.clone();
        }

        if let Some(terminated) = &state.terminated {
            if let Some(container_id) = &terminated.container_id {
                return Some(container_id.clone());
            } else if let Some(last_state) = &last_state {
                if let Some(last_state_terminated) = &last_state.terminated {
                    if let Some(container_id) = &last_state_terminated.container_id {
                        return Some(container_id.clone());
                    }
                }
            }
        }

        if let Some(last_state) = &last_state {
            if let Some(terminated) = &last_state.terminated {
                if let Some(container_id) = &terminated.container_id {
                    return Some(container_id.clone());
                }
            }
        }

        None
    }
}

#[derive(Default)]
struct Tasks(HashMap<TaskId, TaskState>);

impl Tasks {
    fn abort_with_pod_uid(&mut self, pod_uid: &str) {
        self.0.retain(|_, v| v.pod_uid == pod_uid)
    }

    fn abort_with_pod_name(&mut self, pod_name: &str) {
        self.0.retain(|k, _| k.pod_name == pod_name)
    }
}

impl Deref for Tasks {
    type Target = HashMap<TaskId, TaskState>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Tasks {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Hash, PartialEq, Eq, PartialOrd, Ord)]
struct TaskId {
    pod_name: String,
    container_name: String,
}

impl Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.pod_name, self.container_name)
    }
}

struct TaskState {
    handler: JoinHandle<()>,
    pod_name: String,
    pod_uid: String,
    container_name: String,
    container_id: String,
}

impl Drop for TaskState {
    fn drop(&mut self) {
        self.handler.abort();

        logger!(
            info,
            "task abort: pod_name={} pod_uid={} container_name={} container_id={}",
            self.pod_name,
            self.pod_uid,
            self.container_name,
            self.container_id
        );
    }
}
