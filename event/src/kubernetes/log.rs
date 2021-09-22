use super::{Event, Kube};
use crate::{error::PodError, kubernetes::Handlers};

use futures::{future::try_join_all, StreamExt, TryStreamExt};
use tokio::{sync::RwLock, task::JoinHandle};

use std::{sync::Arc, time};

use crossbeam::channel::Sender;

use k8s_openapi::{
    api::core::v1::{Container, ContainerState, ContainerStatus, Event as v1Event, Pod},
    apimachinery::pkg::apis::meta::v1::ObjectMeta,
};

use kube::{
    api::{ListParams, LogParams, WatchEvent},
    Api, Client,
};

use color::Color;

use crate::error::{anyhow, Error, Result};

type BufType = Arc<RwLock<Vec<String>>>;
type PodType = Arc<RwLock<Pod>>;

#[allow(dead_code)]
fn write_error(tx: &Sender<Event>, e: Error) -> Result<()> {
    #[cfg(feature = "logging")]
    ::log::error!("[log] {}", e.to_string());

    tx.send(Event::Kube(Kube::LogStreamResponse(Err(anyhow!(e)))))?;

    Ok(())
}

fn container_statuses(pod: &Pod) -> Result<&[ContainerStatus]> {
    if let Some(status) = &pod.status {
        Ok(&status.container_statuses)
    } else {
        Err(anyhow!(Error::Raw("container_statuses is None".into())))
    }
}

fn init_container_statuses(pod: &Pod) -> Result<&[ContainerStatus]> {
    if let Some(status) = &pod.status {
        Ok(&status.init_container_statuses)
    } else {
        Err(anyhow!(Error::Raw(
            "init_container_statuses is None".into()
        )))
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
) {
    let mut interval = tokio::time::interval(time::Duration::from_millis(200));
    loop {
        interval.tick().await;

        let pod = pod.read().await;
        let pod_status = pod.status.as_ref().unwrap();

        let statuses = match container_type {
            ContainerType::InitContainer => &pod_status.init_container_statuses,
            ContainerType::Container => &pod_status.container_statuses,
        };

        let state = statuses[container_index].state.as_ref().unwrap();
        let last_state = statuses[container_index].last_state.as_ref();

        match container_state_type {
            ContainerStateType::Terminated => {
                if let Some(state) = last_state {
                    if state.terminated.is_some() {
                        return;
                    }
                }

                if state.terminated.is_some() {
                    return;
                }
            }

            ContainerStateType::Running => {
                if state.running.is_some() {
                    return;
                }
            }

            ContainerStateType::Or => {
                if let Some(waiting) = &state.waiting {
                    if let Some(reason) = &waiting.reason {
                        if reason == "PodInitializing" {
                            continue;
                        }

                        if reason == "CrashLoopBackOff" {
                            return;
                        }
                    }
                }

                if state.waiting.is_none() {
                    return;
                }
            }
        }
    }
}

fn is_terminated(status: &ContainerStatus) -> (bool, Option<ContainerState>) {
    if let Some(last_state) = &status.last_state {
        if let Some(state) = &status.state {
            if let Some(waiting) = &state.waiting {
                if let Some(reason) = &waiting.reason {
                    if reason == "CrashLoopBackOff" {
                        return (true, Some(last_state.clone()));
                    }
                }
            }
        }

        if let Some(terminated) = &last_state.terminated {
            if terminated.exit_code != 0 {
                return (true, Some(last_state.clone()));
            }
        }
    }

    if let Some(state) = &status.state {
        if let Some(terminated) = &state.terminated {
            if terminated.exit_code != 0 {
                return (true, Some(state.clone()));
            }
        }
    }

    (false, None)
}

pub struct LogWorkerBuilder {
    tx: Sender<Event>,
    client: Client,
    ns: String,
    pod_name: String,
}

impl LogWorkerBuilder {
    pub fn new(
        tx: Sender<Event>,
        client: Client,
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

use async_trait::async_trait;
#[async_trait]
trait Worker {
    async fn run(&self) -> Result<()>;

    fn spawn(&self) -> JoinHandle<Result<()>>
    where
        Self: Clone + Send + Sync + 'static,
    {
        let worker = self.clone();
        tokio::spawn(async move { worker.run().await })
    }
}

#[derive(Clone)]
pub struct LogWorker {
    tx: Sender<Event>,
    client: Client,
    ns: String,
    pod_name: String,
    message_buffer: BufType,
    pod: PodType,
}

#[derive(Clone)]
struct WatchPodStatusWorker {
    client: Client,
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
    client: Client,
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
        let pod_api = Api::namespaced(self.client.clone(), &self.ns);
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
    async fn run(&self) -> Result<()> {
        let pod_api: Api<Pod> = Api::namespaced(self.client.clone(), &self.ns);

        let lp = ListParams::default()
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
    async fn run(&self) -> Result<()> {
        let mut interval = tokio::time::interval(time::Duration::from_millis(200));

        loop {
            interval.tick().await;
            let mut buf = self.buf.write().await;

            if !buf.is_empty() {
                #[cfg(feature = "logging")]
                ::log::debug!("log_stream Send log stream {}", buf.len());

                self.tx
                    .send(Event::Kube(Kube::LogStreamResponse(Ok(buf.clone()))))?;

                buf.clear();
            }
        }
    }
}

#[async_trait]
impl Worker for FetchLogStreamWorker {
    async fn run(&self) -> Result<()> {
        let pod_api: Api<Pod> = Api::namespaced(self.client.clone(), &self.ns);

        match pod_api.get(&self.pod_name).await {
            Ok(p) => self.fetch_log_stream(p).await?,
            Err(err) => self
                .tx
                .send(Event::Kube(Kube::LogStreamResponse(Err(anyhow!(
                    Error::Kube(err)
                )))))?,
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

        #[cfg(feature = "logging")]
        ::log::info!("log_stream: phase_init_container_log done");

        // containers phase
        let pod = self.pod.read().await;
        let pod_status = pod.status.as_ref().unwrap();
        let enable_prefix = !pod_status.init_container_statuses.is_empty();

        let ret = self.phase_container_log(&mut color, enable_prefix).await?;

        #[cfg(feature = "logging")]
        ::log::info!("log_stream: phase_container_log done");

        for r in ret {
            r?
        }

        Ok(())
    }

    async fn phase_init_container_log(&self, color: &mut Color) -> Result<()> {
        let pod = self.pod.read().await.clone();
        let containers = init_container_statuses(&pod)?;

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
            .await;

            // ログとってくる
            let fetch_log_stream = FetchLogStream {
                buf: self.buf.clone(),
                pod_api: self.pod_api.clone(),
                pod_name: self.pod_name.clone(),
                prefix,
                log_params,
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
            .await;

            // pod status取得
            let pod = self.pod.read().await;
            let statuses = init_container_statuses(&pod)?;
            let status = &statuses[i];
            let metadata = &pod.metadata;

            // exit_code を確認
            if let (true, Some(state)) = is_terminated(status) {
                let container = pod.spec.as_ref().map(|spec| &spec.init_containers[i]);

                let msg = self
                    .terminated_description(
                        container,
                        metadata,
                        &state,
                        status,
                        ContainerType::InitContainer,
                    )
                    .await?;

                self.tx
                    .send(Event::Kube(Kube::LogStreamResponse(Err(anyhow!(
                        Error::Raw(msg),
                    )))))?;

                return Err(anyhow!(PodError::ContainerExitCodeNotZero(
                    container_name,
                    vec![]
                )));
            }
        }
        Ok(())
    }

    async fn phase_container_log(
        &self,
        color: &mut Color,
        enable_prefix: bool,
    ) -> Result<Vec<Result<()>>>
    where
        Self: Clone + Send + Sync + 'static,
    {
        let mut container_handler = Vec::new();

        let pod = self.pod.read().await;
        let containers = container_statuses(&pod)?;

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
                log_params: lp,
                container_name: container_name.clone(),
            };

            let worker = self.clone();

            let handle = tokio::spawn(async move {
                let pod = worker.pod.clone();
                let tx = &worker.tx;
                // // Terminated || Runningになるまで待機する
                wait_container_log(&pod, i, ContainerType::Container, ContainerStateType::Or).await;

                // // ログとってくる
                fetch_log_stream.run().await?;

                // // Terminated
                wait_container_log(
                    &pod,
                    i,
                    ContainerType::Container,
                    ContainerStateType::Terminated,
                )
                .await;

                // // pod status取得
                let pod = pod.read().await;
                let statuses = container_statuses(&pod)?;
                let status = &statuses[i];
                let metadata = &pod.metadata;

                // // exit_code を確認
                if let (true, Some(state)) = is_terminated(status) {
                    let container = pod.spec.as_ref().map(|spec| &spec.containers[i]);

                    let msg = worker
                        .terminated_description(
                            container,
                            metadata,
                            &state,
                            status,
                            ContainerType::Container,
                        )
                        .await?;

                    tx.send(Event::Kube(Kube::LogStreamResponse(Err(anyhow!(
                        Error::Raw(msg),
                    )))))?;

                    Err(anyhow!(PodError::ContainerExitCodeNotZero(
                        container_name,
                        vec![]
                    )))
                } else {
                    Ok(())
                }
            });

            container_handler.push(handle);
        }

        Ok(try_join_all(container_handler).await?)
    }

    // TODO initContainersとcontainersの分岐がややこしいから整理したい
    async fn terminated_description(
        &self,
        container: Option<&Container>,
        metadata: &ObjectMeta,
        state: &ContainerState,
        status: &ContainerStatus,
        ty: ContainerType,
    ) -> Result<String> {
        let mut msg = Vec::new();
        // terminatedはある前提

        let title = format!(" Error {} ", status.name);
        let msg_header = format!("\n\x1b[31m{:=^1$}\x1b[39m\n", title, 30);
        let msg_footer = format!("\n\x1b[31m{}\n\x1b[39m", "=".repeat(30));

        msg.push(msg_header);

        msg.push("Info:".into());
        if let Some(terminated) = &state.terminated {
            msg.push(format!("  ExitCode: {}", terminated.exit_code));

            if let Some(message) = &terminated.message {
                msg.push(format!("  Message: {}", message));
            }

            if let Some(reason) = &terminated.reason {
                msg.push(format!("  Reason: {}", reason));
            }
        }

        if let Some(c) = container {
            if let Some(image) = &c.image {
                msg.push(format!("  Image: {}", image));
            }
            msg.push(format!("  Command: {:?}", c.command));
            msg.push(format!("  Args: {:?}", c.args));
        }

        let event: Api<v1Event> = Api::namespaced(self.client.clone(), &self.ns);

        let mut request_params = format!(
            "involvedObject.name={},involvedObject.namespace={}",
            self.pod_name, self.ns
        );

        match ty {
            ContainerType::InitContainer => {
                request_params += &format!(
                    ",involvedObject.fieldPath=spec.initContainers{{{}}}",
                    status.name
                );
            }
            ContainerType::Container => {
                request_params += &format!(
                    ",involvedObject.fieldPath=spec.containers{{{}}}",
                    status.name
                );
            }
        }

        if let Some(uid) = &metadata.uid {
            request_params += &(",involvedObject.uid=".to_string() + uid);
        }

        let lp = ListParams::default().fields(&request_params);

        let event_result = event.list(&lp).await?;

        msg.push("Event:".into());

        event_result.iter().for_each(|e| {
            #[cfg(feature = "logging")]
            ::log::debug!("phase_container_log event {:?}", e);

            if let Some(m) = &e.message {
                msg.push(format!("  {}", m));
            }
        });

        msg.push(msg_footer);

        Ok(msg.join("\n"))
    }
}

#[derive(Clone)]
struct FetchLogStream {
    buf: BufType,
    pod_api: Api<Pod>,
    pod_name: String,
    prefix: Option<String>,
    log_params: LogParams,
    container_name: String,
}

#[async_trait]
impl Worker for FetchLogStream {
    #[cfg(not(any(feature = "mock", feature = "mock-failed")))]
    async fn run(&self) -> Result<()> {
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

        while let Some(line) = logs.try_next().await? {
            let mut buf = self.buf.write().await;
            buf.push(format!("{}{}", prefix, String::from_utf8_lossy(&line)));

            #[cfg(feature = "logging")]
            ::log::debug!(
                "follow_container_log_stream {}: {}",
                self.pod_name,
                String::from_utf8_lossy(&line)
            );
        }

        #[cfg(feature = "logging")]
        ::log::info!(
            "follow_container_log_stream finished {}:{}",
            self.pod_name,
            self.container_name
        );

        Ok(())
    }

    #[cfg(feature = "mock")]
    async fn run(&self) -> Result<()> {
        async {
            let stream = vec!["line 0", "line 1", "line 2", "line 3", "line 4"];

            for s in stream {
                let mut buf = self.buf.write().await;
                buf.push(s.to_string());
            }
        }
        .await;

        Err(anyhow!(Error::Mock("follow_container_log_stream failed")))
    }

    #[cfg(feature = "mock-failed")]
    async fn run(&self) -> Result<()> {
        Err(anyhow!(Error::Mock("follow_container_log_stream failed")))
    }
}

mod color {
    const COLOR: [u8; 6] = [32, 33, 34, 35, 36, 37];

    pub struct Color {
        index: usize,
    }

    impl Color {
        pub fn new() -> Self {
            Self { index: 0 }
        }

        pub fn next_color(&mut self) -> u8 {
            if COLOR.len() <= self.index {
                self.index = 0;
            }
            self.index += 1;
            COLOR[self.index - 1]
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn color_default() {
            let mut color = Color::new();
            assert_eq!(color.next_color(), 32)
        }

        #[test]
        fn color_next_1() {
            let mut color = Color::new();
            color.next_color();
            assert_eq!(color.next_color(), 33)
        }

        #[test]
        fn color_next_last() {
            let mut color = Color::new();
            color.next_color();
            color.next_color();
            color.next_color();
            color.next_color();
            color.next_color();
            assert_eq!(color.next_color(), 37)
        }

        #[test]
        fn color_next_loop() {
            let mut color = Color::new();
            color.next_color();
            color.next_color();
            color.next_color();
            color.next_color();
            color.next_color();
            color.next_color();
            assert_eq!(color.next_color(), 32)
        }
    }
}
