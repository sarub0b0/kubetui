use std::{
    collections::HashMap,
    fmt::Display,
    ops::{Deref, DerefMut},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use anyhow::anyhow;
use async_trait::async_trait;
use crossbeam::channel::Sender;
use futures::{StreamExt, TryStreamExt};
use k8s_openapi::api::core::v1::{ContainerState, ContainerStatus, Pod};
use kube::{api::WatchParams, core::WatchEvent, Api, ResourceExt};
use regex::Regex;
use tokio::task::AbortHandle;

use crate::{
    event::{
        kubernetes::{
            client::KubeClient,
            worker::{AbortWorker, Worker},
        },
        Event,
    },
    logger, send_response,
};

use super::{
    collector::LogBuffer,
    log_stream::{ContainerLogStreamer, ContainerLogStreamerOptions, ContainerLogStreamerTarget},
};

#[derive(Debug, Clone)]
pub struct PodWatcherFilter {
    pub pod_filter: Option<Regex>,
    pub label_selector: Option<String>,
    pub field_selector: Option<String>,
}

impl Display for PodWatcherFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut buf = Vec::new();

        if let Some(regex) = &self.pod_filter {
            buf.push(format!("pod_filter={}", regex.as_str()));
        }

        if let Some(label_selector) = &self.label_selector {
            buf.push(label_selector.to_string());
        }

        if let Some(field_selector) = &self.field_selector {
            buf.push(format!("field_selector={}", field_selector));
        }

        write!(f, "{}", buf.join(" "))
    }
}

#[derive(Clone)]
pub struct PodWatcher {
    tx: Sender<Event>,
    client: KubeClient,
    log_buffer: LogBuffer,
    namespace: String,
    filter: PodWatcherFilter,
    log_streamer_options: ContainerLogStreamerOptions,
}

impl PodWatcher {
    pub fn new(
        tx: Sender<Event>,
        client: KubeClient,
        log_buffer: LogBuffer,
        namespace: String,
        filter: PodWatcherFilter,
        log_streamer_options: ContainerLogStreamerOptions,
    ) -> Self {
        Self {
            tx,
            client,
            log_buffer,
            namespace,
            filter,
            log_streamer_options,
        }
    }

    fn watch_params(&self) -> WatchParams {
        let mut lp = WatchParams::default().timeout(180);

        if let Some(label_selector) = &self.filter.label_selector {
            lp = lp.labels(label_selector);
        }

        if let Some(field_selector) = &self.filter.field_selector {
            lp = lp.fields(field_selector);
        }

        lp
    }

    fn is_exclude(&self, s: &str) -> bool {
        self.filter
            .pod_filter
            .as_ref()
            .is_some_and(|re| !re.is_match(s))
    }
}

#[async_trait]
impl Worker for PodWatcher {
    type Output = ();

    async fn run(&self) -> Self::Output {
        let lp = self.watch_params();

        let api: Api<Pod> = Api::namespaced(self.client.to_client(), &self.namespace);

        let mut task_controller = TaskController::new(
            self.client.clone(),
            self.log_buffer.clone(),
            self.namespace.clone(),
            self.log_streamer_options.clone(),
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
                        let Some(pod_uid) = pod.uid() else {
                            logger!(error, "Not found pod UID {}", pod.name_any());
                            continue;
                        };

                        let Some(pod_name) = &pod.metadata.name else {
                            logger!(error, "Not found pod name {}", pod.name_any());
                            continue;
                        };

                        logger!(
                            info,
                            "event=added/modified namespace={} pod_name={} pod_uid={}",
                            self.namespace,
                            pod_name,
                            pod_uid
                        );

                        if self.is_exclude(pod_name) {
                            continue;
                        }

                        task_controller.spawn_tasks(&pod, pod_name.to_string(), pod_uid);
                    }
                    Deleted(pod) => {
                        let Some(name) = &pod.metadata.name else {
                            continue;
                        };

                        logger!(
                            info,
                            "event=deleted namespace={} pod_name={} pod_uid={:?}",
                            self.namespace,
                            name,
                            pod.uid()
                        );

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
    tasks: Tasks,
    log_streamer_options: ContainerLogStreamerOptions,
}

impl TaskController {
    fn new(
        client: KubeClient,
        log_buffer: LogBuffer,
        namespace: String,
        log_streamer_options: ContainerLogStreamerOptions,
    ) -> Self {
        Self {
            client,
            log_buffer,
            namespace,
            tasks: Tasks::default(),
            log_streamer_options,
        }
    }

    fn spawn_tasks(&mut self, pod: &Pod, pod_name: String, pod_uid: String) {
        // コンテナステータスを集約
        let container_statuses = Self::aggregate_container_statuses(pod);

        // コンテナごとにタスク生成
        for status in container_statuses {
            let container_name = status.name.clone();

            let task_id = TaskId {
                namespace: self.namespace.clone(),
                pod_name: pod_name.clone(),
                container_name: container_name.clone(),
            };

            let Some(container_id) = Self::is_container_log_available(&status) else {
                logger!(
                    info,
                    "Container ID is empty. state={} task_id={}",
                    Self::container_state_to_string(&status),
                    task_id
                );
                continue;
            };

            // すでにタスク生成されている場合はスキップ
            if let Some(state) = self.tasks.get(&task_id) {
                state
                    .is_terminated
                    .store(Self::is_terminated(&status), Ordering::Relaxed);

                if state.container_id == container_id {
                    logger!(
                        info,
                        "Container ID is the same. state={} task_id={} container_id={}",
                        Self::container_state_to_string(&status),
                        task_id,
                        container_id
                    );
                    continue;
                }

                logger!(
                    info,
                    "Container ID was chaned. state={} task_id={} container_id={}->{}",
                    Self::container_state_to_string(&status),
                    task_id,
                    state.container_id,
                    container_id
                );
            }

            let target = ContainerLogStreamerTarget {
                namespace: self.namespace.clone(),
                pod_name: pod_name.clone(),
                container_name: container_name.clone(),
            };

            let is_terminated = Arc::new(AtomicBool::new(Self::is_terminated(&status)));

            let task = ContainerLogStreamer::new(
                self.client.clone(),
                self.log_buffer.clone(),
                is_terminated.clone(),
                target,
                self.log_streamer_options.clone(),
            )
            .spawn();

            let task_state = TaskState {
                handler: task,
                is_terminated,
                pod_name: pod_name.clone(),
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
        } else if let Some(pod_name) = &pod.metadata.name {
            self.tasks.abort_with_pod_name(pod_name);

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

    fn is_terminated(status: &ContainerStatus) -> bool {
        status
            .state
            .as_ref()
            .is_some_and(|state| state.terminated.is_some())
            || status
                .last_state
                .as_ref()
                .is_some_and(|last_state| last_state.terminated.is_some())
    }

    fn container_state_to_string(status: &ContainerStatus) -> &'static str {
        fn to_string(state: &ContainerState) -> &'static str {
            if state.running.is_some() {
                return "running";
            }

            if state.terminated.is_some() {
                return "terminated";
            }

            if state.waiting.is_some() {
                return "waiting";
            }

            "unknown"
        }

        if let Some(state) = &status.state {
            return to_string(state);
        }

        if let Some(state) = &status.last_state {
            return to_string(state);
        }

        "unknown"
    }
}

#[derive(Debug, Default)]
struct Tasks(HashMap<TaskId, TaskState>);

impl Tasks {
    fn abort_with_pod_uid(&mut self, pod_uid: &str) {
        logger!(info, "abort before. {:?}", self);

        self.0.retain(|_, v| v.pod_uid != pod_uid);

        logger!(info, "abort after. {:?}", self);
    }

    fn abort_with_pod_name(&mut self, pod_name: &str) {
        self.0.retain(|k, _| k.pod_name != pod_name)
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

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
struct TaskId {
    namespace: String,
    pod_name: String,
    container_name: String,
}

impl Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}/{}/{}",
            self.namespace, self.pod_name, self.container_name
        )
    }
}

#[derive(Debug)]
struct TaskState {
    handler: AbortHandle,
    is_terminated: Arc<AtomicBool>,
    pod_name: String,
    pod_uid: String,
    container_name: String,
    container_id: String,
}

impl Drop for TaskState {
    fn drop(&mut self) {
        self.is_terminated.store(true, Ordering::Relaxed);
        self.handler.abort();

        logger!(
            info,
            "task abort: job={:?} pod_name={} pod_uid={} container_name={} container_id={}",
            self.handler,
            self.pod_name,
            self.pod_uid,
            self.container_name,
            self.container_id
        );
    }
}
