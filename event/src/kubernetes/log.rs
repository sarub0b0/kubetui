use super::{Event, Kube};
use crate::{error::PodError, kubernetes::Handlers};

use futures::{future::try_join_all, StreamExt, TryStreamExt};
use tokio::{sync::RwLock, task};

use std::{sync::Arc, time};

use crossbeam::channel::Sender;

use k8s_openapi::{
    api::core::v1::{ContainerState, ContainerStatus, Event as v1Event, Pod},
    Metadata,
};

use kube::{
    api::{ListParams, LogParams, WatchEvent},
    Api, Client,
};

use color::Color;

use crate::error::{anyhow, Error, Result};

type BufType = Arc<RwLock<Vec<String>>>;

#[cfg(not(feature = "new-log-stream"))]
async fn fetch_init_container_log(
    pod_api: &Api<Pod>,
    pod_name: &str,
    lp: LogParams,
    log_prefix: Option<String>,
) -> Result<Vec<String>> {
    let prefix = if let Some(p) = log_prefix {
        p + " "
    } else {
        "".to_string()
    };

    let logs = pod_api.logs(&pod_name, &lp).await?;

    let mut buf = Vec::new();
    for line in logs.lines() {
        buf.push(format!("{}{}", prefix, line));
    }

    #[cfg(feature = "logging")]
    ::log::debug!("fetch_init_container_log {}: {:?}", pod_name, buf);

    #[cfg(feature = "logging")]
    ::log::info!("fetch_init_container_log finished {}", pod_name);

    Ok(buf)
}

#[cfg(not(feature = "new-log-stream"))]
async fn _log_stream(
    tx: Sender<Event>,
    pod_api: Api<Pod>,
    pod_resource: Pod,
    pod_name: &str,
) -> Result<Vec<JoinHandle<()>>> {
    let mut container_handler = Vec::new();

    let lp = LogParams {
        follow: true,
        ..Default::default()
    };

    let mut container_count = 0;

    // バッチでログストリームを渡す
    let buf = Arc::new(RwLock::new(Vec::new()));

    let status = pod_resource.status.unwrap();
    let mut color = Color::new();

    // initContainersのログ取得
    // まだ実行中ならlog_stream, 何かしらで実行終わっていればlogs
    let init_containers = status.init_container_statuses;
    let containers = status.container_statuses;

    for (i, c) in init_containers.iter().enumerate() {
        let state = c.state.as_ref().unwrap();

        let mut lp = lp.clone();

        lp.container = Some(c.name.clone());

        let prefix = Some(format!(
            "\x1b[{}m[init-{}:{}]\x1b[39m",
            color.next_color(),
            i,
            c.name
        ));

        if state.terminated.is_some() {
            let ret = fetch_init_container_log(&pod_api, pod_name, lp, prefix).await;

            if let Err(err) = ret {
                tx.send(Event::Kube(Kube::LogStreamResponse(Err(anyhow!(err)))))
                    .unwrap();
            }
        } else {
            let handlers = spawn_follow_container_log_stream(
                tx.clone(),
                pod_api.clone(),
                pod_name,
                lp,
                Arc::clone(&buf),
                prefix,
            );

            container_handler.push(handlers);
        }
    }

    container_count += init_containers.len();
    container_count += containers.len();

    for c in &containers {
        let tx = tx.clone();

        let mut lp = lp.clone();

        lp.container = Some(c.name.clone());

        let prefix = if 1 < container_count {
            Some(format!("\x1b[{}m[{}]\x1b[39m", color.next_color(), c.name))
        } else {
            None
        };

        let handlers = spawn_follow_container_log_stream(
            tx.clone(),
            pod_api.clone(),
            pod_name,
            lp,
            Arc::clone(&buf),
            prefix,
        );

        container_handler.push(handlers);
    }

    let handler = tokio::spawn(send_loop(tx, buf));

    container_handler.push(handler);

    Ok(container_handler)
}

#[cfg(not(feature = "new-log-stream"))]
pub async fn log_stream(tx: Sender<Event>, client: Client, ns: &str, pod_name: &str) -> Handlers {
    let pod_api: Api<Pod> = Api::namespaced(client.clone(), ns);
    let mut container_handler = Vec::new();

    match pod_api.get(&pod_name).await {
        Ok(pod_resource) => {
            let handlers = _log_stream(tx, pod_api, pod_resource, pod_name)
                .await
                .unwrap();

            container_handler = handlers;
        }
        Err(err) => tx
            .send(Event::Kube(Kube::LogStreamResponse(Err(anyhow!(
                Error::Kube(err)
            )))))
            .unwrap(),
    }

    Handlers(container_handler)
}

type PodType = Arc<RwLock<Pod>>;

async fn watch_pod_status(
    client: Client,
    ns: String,
    pod_name: String,
    pod: PodType,
) -> Result<()> {
    let pod_api: Api<Pod> = Api::namespaced(client.clone(), &ns);

    let lp = ListParams::default()
        .fields(&format!("metadata.name={}", pod_name))
        .timeout(180);

    let mut watch = pod_api.watch(&lp, "0").await?.boxed();

    while let Some(status) = watch.try_next().await? {
        match status {
            WatchEvent::Added(p) | WatchEvent::Modified(p) | WatchEvent::Deleted(p) => {
                let mut pod = pod.write().await;
                *pod = p;
            }
            WatchEvent::Bookmark(_) => {}
            WatchEvent::Error(err) => return Err(anyhow!(err)),
        }
    }

    Ok(())
}

#[cfg(feature = "new-log-stream")]
pub async fn log_stream(
    tx: Sender<Event>,
    client: Client,
    ns: impl Into<String>,
    pod_name: impl Into<String>,
) -> Handlers {
    let ns = ns.into();
    let pod_name = pod_name.into();

    let buf: Arc<RwLock<Vec<String>>> = Arc::new(RwLock::new(Vec::new()));
    let pod: Arc<RwLock<Pod>> = Arc::new(RwLock::new(Pod::default()));

    let send_handler = tokio::spawn(send_buffer(tx.clone(), buf.clone()));

    let watch_handler = tokio::spawn(watch_pod_status(
        client.clone(),
        ns.to_string(),
        pod_name.to_string(),
        pod.clone(),
    ));

    let handler = tokio::spawn(async move {
        let pod_api: Api<Pod> = Api::namespaced(client.clone(), &ns);

        match pod_api.get(&pod_name).await {
            Ok(p) => {
                {
                    let mut pod = pod.write().await;
                    *pod = p.clone();
                }

                let pod_status = p.status.as_ref().unwrap();

                // initContainers phase
                let mut container_count = pod_status.init_container_statuses.len();

                let mut color = Color::new();

                let ret = phase_init_container_log(
                    client.clone(),
                    &pod_api,
                    &pod_name,
                    pod.clone(),
                    &mut color,
                    buf.clone(),
                )
                .await;

                #[cfg(feature = "logging")]
                ::log::info!("log_stream: phase_init_container_log done");

                if let Err(err) = ret {
                    if let Some(PodError::ContainerExitCodeNotZero(_name, msg)) =
                        err.downcast_ref::<PodError>()
                    {
                        tx.send(Event::Kube(Kube::LogStreamResponse(Ok(msg.to_vec()))))?;
                    }

                    return Err(err);
                }

                // containers phase
                container_count += pod_status.container_statuses.len();

                let ret = phase_container_log(
                    tx.clone(),
                    client.clone(),
                    &pod_api,
                    &pod_name,
                    pod.clone(),
                    &mut color,
                    buf.clone(),
                    container_count,
                )
                .await?;

                #[cfg(feature = "logging")]
                ::log::info!("log_stream: phase_container_log done");

                for r in ret {
                    if let Err(e) = r {
                        if let Some(PodError::ContainerExitCodeNotZero(_name, e)) =
                            e.downcast_ref::<PodError>()
                        {
                            tx.send(Event::Kube(Kube::LogStreamResponse(Ok(e.to_vec()))))?;
                        }

                        return Err(e);
                    }
                }
            }
            Err(err) => tx.send(Event::Kube(Kube::LogStreamResponse(Err(anyhow!(
                Error::Kube(err)
            )))))?,
        }

        Ok(())
    });

    Handlers(vec![handler, send_handler, watch_handler])
}

async fn phase_init_container_log(
    client: Client,
    pod_api: &Api<Pod>,
    pod_name: &str,
    shared_pod: PodType,
    color: &mut Color,
    buf: BufType,
) -> Result<()> {
    let pod = shared_pod.read().await.clone();
    let pod_status = pod.status.unwrap();

    let containers_len = pod_status.init_container_statuses.len();

    for i in 0..containers_len {
        let pod_status = shared_pod.read().await.status.clone().unwrap();

        let c = &pod_status.init_container_statuses[i];

        let mut log_params = LogParams {
            follow: true,
            ..Default::default()
        };

        // TODO initContainersの数が2以上ならプレフィックスに数字をいれる
        let prefix = Some(format!(
            "\x1b[{}m[init-{}:{}]\x1b[39m",
            color.next_color(),
            i,
            c.name
        ));

        // Terminated || Runningになるまで待機する
        wait_container_log(
            shared_pod.clone(),
            i,
            ContainerType::InitContainer,
            ContainerStateType::Or,
        )
        .await;

        log_params.container = Some(c.name.clone());

        let args = LogStreamArgs {
            pod: pod_api.clone(),
            pod_name: pod_name.to_string(),
            log_params,
            prefix: prefix.clone(),
        };

        // ログとってくる
        follow_container_log_stream(buf.clone(), args).await?;

        // Terminated
        wait_container_log(
            shared_pod.clone(),
            i,
            ContainerType::InitContainer,
            ContainerStateType::Terminated,
        )
        .await;

        // pod status取得
        let pod = shared_pod.read().await.clone();

        let pod_status = pod.status.as_ref().unwrap();
        let container = &pod_status.init_container_statuses[i];

        // exit_code を確認
        if let (true, Some(state)) = is_terminated(container) {
            let terminated = state.terminated.unwrap();
            let mut msg = Vec::new();

            let title = format!(" Error {} ", container.name);
            let msg_header = format!("\n\x1b[31m{:=^1$}\x1b[39m\n", title, 30);

            let msg_footer = format!("\n\x1b[31m{}\n\x1b[39m", "=".repeat(30));

            msg.push(msg_header);

            msg.push("Info:".into());
            msg.push(format!("  ExitCode: {}", terminated.exit_code));

            if let Some(message) = &terminated.message {
                msg.push(format!("  Message: {}", message));
            }

            if let Some(spec) = &pod.spec {
                let c = &spec.containers[i];
                msg.push(format!("  Command: {:?}", c.command));
                msg.push(format!("  Args: {:?}", c.args));
            }

            let rpod = shared_pod.read().await;
            let ns = rpod.metadata().namespace.clone().unwrap_or_default();
            let uid = rpod.metadata().uid.clone().unwrap_or_default();
            let name = rpod.metadata().name.clone().unwrap_or_default();

            let event: Api<v1Event> = Api::namespaced(client, &ns);

            let lp = ListParams::default().fields(&format!(
                "involvedObject.name={},involvedObject.namespace={},involvedObject.uid={}",
                name, ns, uid,
            ));

            let event_result = event.list(&lp).await?;

            msg.push("Event:".into());
            event_result.iter().for_each(|e| {
                #[cfg(feature = "logging")]
                ::log::info!("phase_init_container_log event {:?}", e);

                if let Some(m) = e.message.as_ref() {
                    msg.push(format!("  {}", m));
                }
            });

            msg.push(msg_footer);

            return Err(anyhow!(PodError::ContainerExitCodeNotZero(
                container.name.to_string(),
                msg
            )));
        }
    }

    Ok(())
}

async fn phase_container_log(
    tx: Sender<Event>,
    client: Client,
    pod_api: &Api<Pod>,
    pod_name: &str,
    shared_pod: PodType,
    color: &mut Color,
    buf: BufType,
    container_count: usize,
) -> Result<Vec<Result<()>>> {
    let pod = shared_pod.read().await.clone();
    let pod_status = pod.status.unwrap();
    let containers = pod_status.container_statuses;
    let mut container_handler = Vec::new();

    for (i, c) in containers.iter().enumerate() {
        let mut lp = LogParams {
            follow: true,
            ..Default::default()
        };

        lp.container = Some(c.name.clone());

        let prefix = if 1 < container_count {
            Some(format!("\x1b[{}m[{}]\x1b[39m", color.next_color(), c.name))
        } else {
            None
        };

        let args = LogStreamArgs {
            pod: pod_api.clone(),
            pod_name: pod_name.to_string(),
            log_params: lp,
            prefix: prefix.clone(),
        };

        let buf = buf.clone();
        let shared_pod = shared_pod.clone();
        let client = client.clone();
        let tx = tx.clone();

        let handle = task::spawn(async move {
            // Terminated || Runningになるまで待機する
            wait_container_log(
                shared_pod.clone(),
                i,
                ContainerType::Container,
                ContainerStateType::Or,
            )
            .await;

            follow_container_log_stream(buf.clone(), args).await?;

            // Terminated
            wait_container_log(
                shared_pod.clone(),
                i,
                ContainerType::Container,
                ContainerStateType::Terminated,
            )
            .await;

            // pod status取得
            let pod = shared_pod.read().await.clone();

            let pod_status = pod.status.as_ref().unwrap();
            let container = &pod_status.container_statuses[i];

            // exit_code を確認
            if let (true, Some(state)) = is_terminated(container) {
                let terminated = state.terminated.unwrap();
                let mut msg = Vec::new();

                let title = format!(" Error {} ", container.name);
                let msg_header = format!("\n\x1b[31m{:=^1$}\x1b[39m\n", title, 30);

                let msg_footer = format!("\n\x1b[31m{}\n\x1b[39m", "=".repeat(30));

                msg.push(msg_header);

                msg.push("Info:".into());
                msg.push(format!("  ExitCode: {}", terminated.exit_code));

                if let Some(message) = &terminated.message {
                    msg.push(format!("  Message: {}", message));
                }

                if let Some(spec) = &pod.spec {
                    let c = &spec.containers[i];
                    msg.push(format!("  Command: {:?}", c.command));
                    msg.push(format!("  Args: {:?}", c.args));
                }

                let rpod = shared_pod.read().await;
                let ns = rpod.metadata().namespace.clone().unwrap_or_default();
                let uid = rpod.metadata().uid.clone().unwrap_or_default();
                let name = rpod.metadata().name.clone().unwrap_or_default();

                let event: Api<v1Event> = Api::namespaced(client, &ns);

                let lp = ListParams::default().fields(&format!(
                    "involvedObject.name={},involvedObject.namespace={},involvedObject.uid={}",
                    name, ns, uid,
                ));

                let event_result = event.list(&lp).await?;

                msg.push("Event:".into());
                event_result.iter().for_each(|e| {
                    #[cfg(feature = "logging")]
                    ::log::info!("phase_container_log event {:?}", e);

                    if let Some(m) = e.message.as_ref() {
                        msg.push(format!("  {}", m));
                    }
                });

                msg.push(msg_footer);

                tx.send(Event::Kube(Kube::LogStreamResponse(Err(anyhow!(
                    Error::Raw(msg.join("\n")),
                )))))
                .unwrap();
            }
            Ok(())
        });

        container_handler.push(handle);
    }

    Ok(try_join_all(container_handler).await?)
}

async fn send_buffer(tx: Sender<Event>, buf: BufType) -> Result<()> {
    let mut interval = tokio::time::interval(time::Duration::from_millis(200));

    loop {
        interval.tick().await;
        let mut buf = buf.write().await;

        if !buf.is_empty() {
            #[cfg(feature = "logging")]
            ::log::debug!("log_stream Send log stream {}", buf.len());

            tx.send(Event::Kube(Kube::LogStreamResponse(Ok(buf.clone()))))?;

            buf.clear();
        }
    }
}

struct LogStreamArgs {
    pod: Api<Pod>,
    pod_name: String,
    prefix: Option<String>,
    log_params: LogParams,
}

enum ContainerType {
    InitContainer,
    Container,
}

#[allow(dead_code)]
enum ContainerStateType {
    Terminated,
    Running,
    Or,
}

async fn wait_container_log(
    pod: PodType,
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

#[cfg(not(any(feature = "mock", feature = "mock-failed")))]
async fn follow_container_log_stream(buf: BufType, args: LogStreamArgs) -> Result<()> {
    let LogStreamArgs {
        pod,
        pod_name,
        prefix,
        log_params: lp,
    } = args;

    let prefix = if let Some(p) = prefix {
        p + " "
    } else {
        "".to_string()
    };

    let mut logs = pod.log_stream(&pod_name, &lp).await?.boxed();

    while let Some(line) = logs.try_next().await? {
        let mut buf = buf.write().await;
        buf.push(format!("{}{}", prefix, String::from_utf8_lossy(&line)));

        #[cfg(feature = "logging")]
        ::log::debug!(
            "follow_container_log_stream {}: {}",
            pod_name,
            String::from_utf8_lossy(&line)
        );
    }

    #[cfg(feature = "logging")]
    ::log::info!(
        "follow_container_log_stream finished {}:{}",
        pod_name,
        lp.container.unwrap()
    );
    Ok(())
}

#[cfg(feature = "mock")]
async fn follow_container_log_stream(buf: BufType, _: LogStreamArgs) -> Result<()> {
    async {
        let stream = vec!["line 0", "line 1", "line 2", "line 3", "line 4"];

        for s in stream {
            let mut buf = buf.write().await;
            buf.push(s.to_string());
        }
    }
    .await;

    Err(Error::Mock("follow_container_log_stream failed"))
}

#[cfg(feature = "mock-failed")]
async fn follow_container_log_stream(buf: BufType, _: LogStreamArgs) -> Result<()> {
    Err(anyhow!(Error::Mock("follow_container_log_stream failed")))
}

#[cfg(not(feature = "new-log-stream"))]
fn spawn_follow_container_log_stream(
    tx: Sender<Event>,
    pod: Api<Pod>,
    pod_name: &str,
    lp: LogParams,
    buf: BufType,
    log_prefix: Option<String>,
) -> JoinHandle<()> {
    let pod_name = pod_name.into();
    tokio::spawn(async move {
        let args = LogStreamArgs {
            pod,
            pod_name,
            log_params: lp,
            prefix: log_prefix,
        };

        let stream = follow_container_log_stream(buf, args).await;

        if let Err(err) = stream {
            tx.send(Event::Kube(Kube::LogStreamResponse(Err(err))))
                .unwrap();
        }
    })
}

#[allow(dead_code)]
mod msg {
    const DEBUG: &str = "\x1b[90m";
    const INFO: &str = "\x1b[90m";
    const WARN: &str = "\x1b[33m";
    const ERR: &str = "\x1b[31m";

    const DEFAULT_COLOR: &str = "\x1b[37m";

    #[inline]
    pub fn debug(fmt: impl Into<String>) -> String {
        format!("{}{}{}", DEBUG, fmt.into(), DEFAULT_COLOR)
    }

    #[inline]
    pub fn info(fmt: impl Into<String>) -> String {
        format!("{}{}{}", INFO, fmt.into(), DEFAULT_COLOR)
    }

    #[inline]
    pub fn warn(fmt: impl Into<String>) -> String {
        format!("{}{}{}", WARN, fmt.into(), DEFAULT_COLOR)
    }

    #[inline]
    pub fn error(fmt: impl Into<String>) -> String {
        format!("{}{}{}", ERR, fmt.into(), DEFAULT_COLOR)
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
