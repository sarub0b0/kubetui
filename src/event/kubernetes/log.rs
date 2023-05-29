use super::{client::KubeClient, Event, Handlers, Kube, Worker};
use crate::{error::PodError, event::kubernetes::color::Color, logger};

use async_trait::async_trait;
use chrono::Local;
use futures::{future::try_join_all, StreamExt, TryStreamExt};
use tokio::sync::RwLock;

use std::{sync::Arc, time};

use crossbeam::channel::Sender;

use k8s_openapi::{
    api::core::v1::{Container, ContainerState, ContainerStatus, Event as v1Event, Pod},
    apimachinery::pkg::apis::meta::v1::Time,
};

use kube::{
    api::{ListParams, LogParams, WatchEvent, WatchParams},
    Api, ResourceExt,
};

use crate::error::{anyhow, Error, Result};

type BufType = Arc<RwLock<Vec<String>>>;
type PodType = Arc<RwLock<Pod>>;

#[derive(Debug)]
pub enum LogStreamMessage {
    Request { namespace: String, name: String },
    Response(Result<Vec<String>>),
}

impl From<LogStreamMessage> for Event {
    fn from(m: LogStreamMessage) -> Event {
        Event::Kube(Kube::LogStream(m))
    }
}

fn as_ref_container_statuses(pod: &Pod) -> Result<Option<&Vec<ContainerStatus>>> {
    if let Some(status) = &pod.status {
        Ok(status.container_statuses.as_ref())
    } else {
        Err(anyhow!(Error::Raw("PodStatus is None".into())))
    }
}

fn as_ref_init_container_statuses(pod: &Pod) -> Result<Option<&Vec<ContainerStatus>>> {
    if let Some(status) = &pod.status {
        Ok(status.init_container_statuses.as_ref())
    } else {
        Err(anyhow!(Error::Raw("PodStatus is None".into())))
    }
}

enum ContainerType {
    InitContainer,
    Container,
}

enum ContainerStateType {
    Terminated,

    #[allow(dead_code)]
    Running,

    Or,
}

async fn wait_container_log(
    pod: &PodType,
    container_index: usize,
    container_type: ContainerType,
    container_state_type: ContainerStateType,
) -> Result<()> {
    let mut interval = tokio::time::interval(time::Duration::from_millis(200));
    loop {
        interval.tick().await;

        let pod = pod.read().await;
        let pod_name = pod.name_any();

        let container_name = match container_type {
            ContainerType::InitContainer => pod
                .spec
                .as_ref()
                .and_then(|spec| {
                    spec.init_containers
                        .as_ref()
                        .map(|c| c[container_index].name.to_string())
                })
                .unwrap_or_else(|| "unknown".to_string()),

            ContainerType::Container => pod
                .spec
                .as_ref()
                .map(|spec| spec.containers[container_index].name.to_string())
                .unwrap_or_else(|| "unknown".to_string()),
        };

        let statuses = match container_type {
            ContainerType::InitContainer => as_ref_init_container_statuses(&pod)?,
            ContainerType::Container => as_ref_container_statuses(&pod)?,
        };

        let state = if let Some(statuses) = statuses {
            statuses[container_index].state.as_ref()
        } else {
            None
        };

        let last_state = if let Some(statuses) = statuses {
            statuses[container_index].last_state.as_ref()
        } else {
            None
        };

        let type_str = match container_state_type {
            ContainerStateType::Terminated => "terminated",
            ContainerStateType::Running => "running",
            ContainerStateType::Or => "running or terminated",
        };

        logger!(
            info,
            "Wait to fetch until this container is {} [{}:{}]",
            type_str,
            pod_name,
            container_name
        );

        match container_state_type {
            ContainerStateType::Terminated => {
                if let Some(state) = last_state {
                    if state.terminated.is_some() {
                        logger!(
                            info,
                            "This container is terminated [{}:{}]",
                            pod_name,
                            container_name
                        );

                        break;
                    }
                }

                if let Some(state) = state {
                    if state.terminated.is_some() {
                        logger!(
                            info,
                            "This container is terminated [{}:{}]",
                            pod_name,
                            container_name
                        );

                        break;
                    }
                }
            }

            ContainerStateType::Running => {
                if let Some(state) = state {
                    if state.running.is_some() {
                        logger!(
                            info,
                            "This container is running [{}:{}]",
                            pod_name,
                            container_name
                        );

                        break;
                    }
                }
            }

            ContainerStateType::Or => {
                if let Some(state) = state {
                    if let Some(waiting) = &state.waiting {
                        if let Some(reason) = &waiting.reason {
                            if reason == "PodInitializing" {
                                logger!(
                                    info,
                                    "This container is initializing [{}:{}]",
                                    pod_name,
                                    container_name
                                );

                                continue;
                            }

                            if reason == "CrashLoopBackOff" {
                                logger!(
                                    info,
                                    "This container is crashed [{}:{}]",
                                    pod_name,
                                    container_name
                                );

                                break;
                            }
                        }
                    } else {
                        logger!(
                            info,
                            "This container is running or terminated [{}:{}]",
                            pod_name,
                            container_name
                        );

                        break;
                    }
                }
            }
        }
    }

    Ok(())
}

fn is_terminated(status: &ContainerStatus) -> bool {
    if let Some(last_state) = &status.last_state {
        if let Some(state) = &status.state {
            if let Some(waiting) = &state.waiting {
                if let Some(reason) = &waiting.reason {
                    if reason == "CrashLoopBackOff" {
                        return true;
                    }
                }
            }
        }

        if let Some(terminated) = &last_state.terminated {
            if terminated.exit_code != 0 {
                return true;
            }
        }
    }

    if let Some(state) = &status.state {
        if let Some(terminated) = &state.terminated {
            if terminated.exit_code != 0 {
                return true;
            }
        }
    }

    false
}

pub struct LogWorkerBuilder {
    tx: Sender<Event>,
    client: KubeClient,
    ns: String,
    pod_name: String,
}

impl LogWorkerBuilder {
    pub fn new(
        tx: Sender<Event>,
        client: KubeClient,
        ns: impl Into<String>,
        pod_name: impl Into<String>,
    ) -> Self {
        Self {
            tx,
            client,
            ns: ns.into(),
            pod_name: pod_name.into(),
        }
    }

    pub fn build(self) -> LogWorker {
        LogWorker {
            tx: self.tx,
            client: self.client,
            ns: self.ns,
            pod_name: self.pod_name,
            message_buffer: Default::default(),
            pod: Default::default(),
        }
    }
}

#[derive(Clone)]
pub struct LogWorker {
    tx: Sender<Event>,
    client: KubeClient,
    ns: String,
    pod_name: String,
    message_buffer: BufType,
    pod: PodType,
}

#[derive(Clone)]
struct WatchPodStatusWorker {
    client: KubeClient,
    ns: String,
    pod_name: String,
    pod: PodType,
}

#[derive(Clone)]
struct SendMessageWorker {
    buf: BufType,
    tx: Sender<Event>,
}

#[derive(Clone)]
struct FetchLogStreamWorker {
    tx: Sender<Event>,
    client: KubeClient,
    ns: String,
    pod_name: String,
    pod: PodType,
    pod_api: Api<Pod>,
    buf: BufType,
}

impl LogWorker {
    pub fn spawn(&self) -> Handlers {
        Handlers(vec![
            self.to_send_message_worker().spawn(),
            self.to_watch_pod_status_worker().spawn(),
            self.to_fetch_log_stream_worker().spawn(),
        ])
    }

    fn to_watch_pod_status_worker(&self) -> WatchPodStatusWorker {
        WatchPodStatusWorker {
            client: self.client.clone(),
            ns: self.ns.clone(),
            pod_name: self.pod_name.clone(),
            pod: self.pod.clone(),
        }
    }

    fn to_send_message_worker(&self) -> SendMessageWorker {
        SendMessageWorker {
            buf: self.message_buffer.clone(),
            tx: self.tx.clone(),
        }
    }

    fn to_fetch_log_stream_worker(&self) -> FetchLogStreamWorker {
        let pod_api = Api::namespaced(self.client.as_client().clone(), &self.ns);
        FetchLogStreamWorker {
            client: self.client.clone(),
            ns: self.ns.clone(),
            pod_name: self.pod_name.clone(),
            pod_api,
            pod: self.pod.clone(),
            buf: self.message_buffer.clone(),
            tx: self.tx.clone(),
        }
    }
}

#[async_trait]
impl Worker for WatchPodStatusWorker {
    type Output = Result<()>;
    async fn run(&self) -> Self::Output {
        let pod_api: Api<Pod> = Api::namespaced(self.client.as_client().clone(), &self.ns);

        let lp = WatchParams::default()
            .fields(&format!("metadata.name={}", self.pod_name))
            .timeout(180);

        let mut watch = pod_api.watch(&lp, "0").await?.boxed();

        while let Some(status) = watch.try_next().await? {
            match status {
                WatchEvent::Added(p) | WatchEvent::Modified(p) | WatchEvent::Deleted(p) => {
                    let mut pod = self.pod.write().await;
                    *pod = p;
                }
                WatchEvent::Bookmark(_) => {}
                WatchEvent::Error(err) => return Err(anyhow!(err)),
            }
        }

        Ok(())
    }
}

#[async_trait]
impl Worker for SendMessageWorker {
    type Output = Result<()>;
    async fn run(&self) -> Self::Output {
        let mut interval = tokio::time::interval(time::Duration::from_millis(200));

        loop {
            interval.tick().await;
            let mut buf = self.buf.write().await;

            if !buf.is_empty() {
                self.tx
                    .send(LogStreamMessage::Response(Ok(buf.clone())).into())?;

                buf.clear();
            }
        }
    }
}

#[async_trait]
impl Worker for FetchLogStreamWorker {
    type Output = Result<()>;
    async fn run(&self) -> Self::Output {
        let pod_api: Api<Pod> = Api::namespaced(self.client.as_client().clone(), &self.ns);

        match pod_api.get(&self.pod_name).await {
            Ok(p) => self.fetch_log_stream(p).await?,
            Err(err) => self
                .tx
                .send(LogStreamMessage::Response(Err(anyhow!(Error::Kube(err)))).into())?,
        }

        Ok(())
    }
}

impl FetchLogStreamWorker {
    async fn fetch_log_stream(&self, pod: Pod) -> Result<()> {
        // watchワーカーが更新できていないことがあるため、最新のデータをここで設定する
        {
            let mut p = self.pod.write().await;
            *p = pod;
        }

        let mut color = Color::new();

        // initContainers phase
        self.phase_init_container_log(&mut color).await?;

        let ret = self.phase_container_log(&mut color).await?;

        for r in ret {
            r?
        }

        Ok(())
    }

    async fn phase_init_container_log(&self, color: &mut Color) -> Result<()> {
        let pod = self.pod.read().await.clone();

        if let Some(containers) = as_ref_init_container_statuses(&pod)? {
            let containers_len = containers.len();

            for (i, c) in containers.iter().enumerate() {
                let mut log_params = LogParams {
                    follow: true,
                    ..Default::default()
                };

                let container_name = c.name.clone();

                log_params.container = Some(container_name.clone());

                let prefix = if 1 < containers_len {
                    Some(format!(
                        "\x1b[{}m[init-{}:{}]\x1b[39m",
                        color.next_color(),
                        i,
                        c.name
                    ))
                } else {
                    Some(format!(
                        "\x1b[{}m[init:{}]\x1b[39m",
                        color.next_color(),
                        c.name
                    ))
                };

                // Terminated || Runningになるまで待機する
                wait_container_log(
                    &self.pod,
                    i,
                    ContainerType::InitContainer,
                    ContainerStateType::Or,
                )
                .await?;

                // ログとってくる
                let fetch_log_stream = FetchLogStream {
                    buf: self.buf.clone(),
                    pod_api: self.pod_api.clone(),
                    pod_name: self.pod_name.clone(),
                    prefix,
                    container_name: container_name.clone(),
                };

                fetch_log_stream.run().await?;

                // Terminated
                wait_container_log(
                    &self.pod,
                    i,
                    ContainerType::InitContainer,
                    ContainerStateType::Terminated,
                )
                .await?;

                // pod status取得
                let pod = self.pod.read().await;

                if let Some(statuses) = as_ref_init_container_statuses(&pod)? {
                    let status = &statuses[i];

                    // exit_code を確認
                    if is_terminated(status) {
                        let container = if let Some(spec) = &pod.spec {
                            spec.init_containers.as_ref().map(|c| &c[i])
                        } else {
                            None
                        };

                        let mut selector = format!(
                        "involvedObject.name={},involvedObject.namespace={},involvedObject.fieldPath=spec.containers{{{}}}",
                        self.pod_name, self.ns, status.name
                    );

                        if let Some(uid) = &pod.metadata.uid {
                            selector += &format!(",involvedObject.uid={}", uid);
                        }

                        let msg = self
                            .terminated_description(container, status, &selector)
                            .await?;

                        self.tx.send(
                            LogStreamMessage::Response(Err(anyhow!(Error::VecRaw(msg),))).into(),
                        )?;

                        return Err(anyhow!(PodError::ContainerExitCodeNotZero(
                            container_name,
                            vec![]
                        )));
                    }
                }
            }
        }
        Ok(())
    }

    async fn phase_container_log(&self, color: &mut Color) -> Result<Vec<Result<()>>>
    where
        Self: Clone + Send + Sync + 'static,
    {
        let mut container_handler = Vec::new();

        let pod = self.pod.read().await.clone();

        // containersが１つの時だけプレフィックスをつけない
        let enable_prefix = {
            if let Some(status) = &pod.status {
                if let Some(statuses) = &status.init_container_statuses {
                    !statuses.is_empty()
                } else if let Some(statuses) = &status.container_statuses {
                    1 < statuses.len()
                } else {
                    false
                }
            } else {
                false
            }
        };

        if let Some(containers) = as_ref_container_statuses(&pod)? {
            for (i, c) in containers.iter().enumerate() {
                let mut lp = LogParams {
                    follow: true,
                    ..Default::default()
                };

                let container_name = c.name.clone();

                lp.container = Some(c.name.clone());

                let prefix = if enable_prefix {
                    Some(format!("\x1b[{}m[{}]\x1b[39m", color.next_color(), c.name))
                } else {
                    None
                };

                let fetch_log_stream = FetchLogStream {
                    buf: self.buf.clone(),
                    pod_api: self.pod_api.clone(),
                    pod_name: self.pod_name.clone(),
                    prefix,
                    container_name: container_name.clone(),
                };

                let worker = self.clone();

                let handle = tokio::spawn(async move {
                    let tx = &worker.tx;

                    // Terminated || Runningになるまで待機する
                    wait_container_log(
                        &worker.pod,
                        i,
                        ContainerType::Container,
                        ContainerStateType::Or,
                    )
                    .await?;

                    // ログとってくる
                    fetch_log_stream.run().await?;

                    // Terminated
                    wait_container_log(
                        &worker.pod,
                        i,
                        ContainerType::Container,
                        ContainerStateType::Terminated,
                    )
                    .await?;

                    // // pod status取得
                    let pod = worker.pod.read().await;

                    if let Some(statuses) = as_ref_container_statuses(&pod)? {
                        let status = &statuses[i];

                        // // exit_code を確認
                        if is_terminated(status) {
                            let container = pod.spec.as_ref().map(|spec| &spec.containers[i]);

                            let mut selector = format!(
                                "involvedObject.name={},involvedObject.namespace={},involvedObject.fieldPath=spec.containers{{{}}}",
                                worker.pod_name, worker.ns ,status.name
                            );

                            if let Some(uid) = &pod.metadata.uid {
                                selector += &format!(",involvedObject.uid={}", uid);
                            }

                            let msg = worker
                                .terminated_description(container, status, &selector)
                                .await?;

                            tx.send(
                                LogStreamMessage::Response(Err(anyhow!(Error::VecRaw(msg)))).into(),
                            )?;

                            return Err(anyhow!(PodError::ContainerExitCodeNotZero(
                                container_name,
                                vec![]
                            )));
                        }
                    }
                    Ok(())
                });

                container_handler.push(handle);
            }
        }

        Ok(try_join_all(container_handler).await?)
    }

    async fn terminated_description(
        &self,
        container: Option<&Container>,
        status: &ContainerStatus,
        selector: &str,
    ) -> Result<Vec<String>> {
        fn time_to_string(time: &Time) -> String {
            time.0
                .with_timezone(&Local)
                .format("%a, %d %b %Y %T %z")
                .to_string()
        }

        fn container_info(buf: &mut Vec<String>, container: Option<&Container>) {
            if let Some(c) = container {
                if let Some(image) = &c.image {
                    buf.push(format!("  Image:       {}", image));
                }

                if let Some(command) = &c.command {
                    buf.push("  Command:".into());

                    for cmd in command {
                        buf.push(format!("    {}", cmd));
                    }
                }

                if let Some(args) = &c.args {
                    buf.push("  Args:".into());

                    for arg in args {
                        buf.push(format!("    {}", arg));
                    }
                }
            }
        }
        fn container_status(buf: &mut Vec<String>, status: &ContainerStatus) {
            fn terminated(buf: &mut Vec<String>, state: &ContainerState, state_type: &str) {
                if let Some(terminated) = &state.terminated {
                    buf.push(state_type.into());

                    buf.push(format!("    Exit Code: {}", terminated.exit_code));

                    if let Some(message) = &terminated.message {
                        buf.push(format!("    Message:   {}", message));
                    }

                    if let Some(reason) = &terminated.reason {
                        buf.push(format!("    Reason:    {}", reason));
                    }

                    if let Some(started_at) = &terminated.started_at {
                        buf.push(format!("    Started:   {}", time_to_string(started_at)));
                    }

                    if let Some(finished_at) = &terminated.finished_at {
                        buf.push(format!("    Finished:  {}", time_to_string(finished_at)));
                    }
                }
            }

            fn waiting(buf: &mut Vec<String>, state: &ContainerState, state_type: &str) {
                if let Some(waiting) = &state.waiting {
                    buf.push(state_type.into());

                    // if let Some(message) = &waiting.message {
                    //     buf.push(format!("    Message:   {}", message));
                    // }

                    if let Some(reason) = &waiting.reason {
                        buf.push(format!("    Reason:    {}", reason));
                    }
                }
            }

            fn running(buf: &mut Vec<String>, state: &ContainerState, state_type: &str) {
                if let Some(running) = &state.running {
                    buf.push(state_type.into());

                    if let Some(started_at) = &running.started_at {
                        buf.push(format!("    Started:   {}", time_to_string(started_at)));
                    }
                }
            }

            if let Some(state) = &status.state {
                terminated(buf, state, "  State:       Terminated");
                waiting(buf, state, "  State:       Waiting");
                running(buf, state, "  State:       Running");
            }

            if let Some(last_state) = &status.last_state {
                terminated(buf, last_state, "  Last State:  Terminated");
                waiting(buf, last_state, "  Last State:  Waiting");
                running(buf, last_state, "  Last State:  Running");
            }
        }

        let mut msg = Vec::new();

        let title = format!(" Error {} ", status.name);
        let msg_header = format!("\n\x1b[31m{:=^1$}\x1b[39m\n", title, 50);
        let msg_footer = format!("\n\x1b[31m{}\n\x1b[39m", "=".repeat(50));

        msg.push(msg_header);

        msg.push("Info:".into());

        container_info(&mut msg, container);
        container_status(&mut msg, status);

        let event: Api<v1Event> = Api::namespaced(self.client.as_client().clone(), &self.ns);
        let lp = ListParams::default().fields(selector);

        let event_result = event.list(&lp).await?;

        msg.push("Event:".into());

        event_result.iter().for_each(|e| {
            logger!(debug, "Event {:?}", e);

            if let Some(m) = &e.message {
                msg.push(format!("  {}", m));
            }
        });

        msg.push(msg_footer);

        Ok(msg)
    }
}

#[derive(Clone)]
struct FetchLogStream {
    buf: BufType,
    pod_api: Api<Pod>,
    pod_name: String,
    prefix: Option<String>,
    container_name: String,
}

#[async_trait]
impl Worker for FetchLogStream {
    type Output = Result<()>;

    #[cfg(not(any(feature = "mock", feature = "mock-failed")))]
    async fn run(&self) -> Self::Output {
        logger!(
            info,
            "Start fetching this container log [{}:{}]",
            self.pod_name,
            self.container_name
        );

        let lp = LogParams {
            follow: true,
            container: Some(self.container_name.to_string()),
            ..Default::default()
        };

        let prefix = if let Some(p) = &self.prefix {
            p.to_owned() + " "
        } else {
            "".to_string()
        };

        let mut logs = self.pod_api.log_stream(&self.pod_name, &lp).await?.boxed();

        while let Some(bytes) = logs.try_next().await? {
            let mut buf = self.buf.write().await;

            let logs = String::from_utf8_lossy(&bytes);

            logger!(
                debug,
                "Container log stream [{}:{}] - {}",
                self.pod_name,
                self.container_name,
                logs
            );

            for line in logs.lines() {
                buf.push(format!("{}{}", prefix, line));

                logger!(
                    debug,
                    "Container log stream [{}:{}] - {}",
                    self.pod_name,
                    self.container_name,
                    line
                );
            }
        }

        logger!(
            info,
            "Finish fetching this container log [{}:{}]",
            self.pod_name,
            self.container_name
        );

        Ok(())
    }
}
