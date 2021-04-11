use k8s_openapi::api::core::v1::ContainerStateTerminated;

use super::{Event, Kube};
use crate::kubernetes::Handlers;

use futures::{StreamExt, TryStreamExt};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use std::{sync::Arc, time, vec};

use crossbeam::channel::Sender;

use k8s_openapi::api::core::v1::Pod;

use kube::{api::LogParams, Api, Client, Result};

use color::Color;

pub async fn log_stream(
    tx: Sender<Event>,
    client: Client,
    ns: String,
    pod_name: String,
) -> Handlers {
    let pod: Api<Pod> = Api::namespaced(client.clone(), &ns);
    let mut lp = LogParams::default();
    lp.follow = true;

    // バッチでログストリームを渡す
    let buf = Arc::new(RwLock::new(Vec::new()));

    let mut container_handler = Vec::new();

    if let Ok(init) = pod.get(&pod_name).await {
        let status = init.status.unwrap();
        let mut color = Color::new();

        // initContainersのログ取得
        // まだ実行中ならlog_stream, 何かしらで実行終わっていればlogs
        if let Some(ref containers) = status.init_container_statuses {
            for (i, c) in containers.iter().enumerate() {
                let state = c.state.as_ref().unwrap();

                let mut lp = lp.clone();

                lp.container = Some(c.name.clone());

                let prefix = Some(format!(
                    "\x1b[{}m[init-{}:{}]\x1b[39m",
                    color.next().unwrap(),
                    i,
                    c.name
                ));

                if is_terminated_container(&state.terminated) {
                    let handler = container_logs(
                        tx.clone(),
                        pod.clone(),
                        pod_name.clone(),
                        lp,
                        Arc::clone(&buf),
                        prefix,
                    );
                    handler.await.unwrap();
                } else {
                    let mut handlers = container_log_stream(
                        tx.clone(),
                        pod.clone(),
                        pod_name.clone(),
                        lp,
                        Arc::clone(&buf),
                        prefix,
                    );
                    container_handler.append(&mut handlers);
                }
            }
        }

        if let Some(containers) = status.container_statuses {
            for c in &containers {
                let mut lp = lp.clone();
                let tx = tx.clone();

                lp.container = Some(c.name.clone());

                let prefix = if 1 < containers.len() || status.init_container_statuses.is_some() {
                    Some(format!(
                        "\x1b[{}m[{}]\x1b[39m",
                        color.next().unwrap(),
                        c.name
                    ))
                } else {
                    None
                };

                let mut handlers = container_log_stream(
                    tx.clone(),
                    pod.clone(),
                    pod_name.clone(),
                    lp,
                    Arc::clone(&buf),
                    prefix,
                );

                container_handler.append(&mut handlers);
            }
        }
    }

    Handlers(container_handler)
}

fn container_logs(
    tx: Sender<Event>,
    pod: Api<Pod>,
    pod_name: String,
    lp: LogParams,
    buf: Arc<RwLock<Vec<String>>>,
    log_prefix: Option<String>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let prefix = if let Some(ref p) = log_prefix {
            p.to_owned() + " "
        } else {
            "".to_string()
        };

        let logs = pod.logs(&pod_name, &lp).await.unwrap();

        for line in logs.lines() {
            let mut buf = buf.write().await;
            buf.push(format!("{}{}", prefix, line));
        }

        let mut buf = buf.write().await;

        tx.send(Event::Kube(Kube::LogStreamResponse(buf.clone())))
            .unwrap();

        buf.clear();
    })
}

fn container_log_stream(
    tx: Sender<Event>,
    pod: Api<Pod>,
    pod_name: String,
    mut lp: LogParams,
    buf: Arc<RwLock<Vec<String>>>,
    log_prefix: Option<String>,
) -> Vec<JoinHandle<()>> {
    let buf_ = buf.clone();
    let buf_handle = tokio::spawn(async move {
        let prefix = if let Some(ref p) = log_prefix.clone() {
            p.to_owned() + " "
        } else {
            "".to_string()
        };

        loop {
            let pod_ = pod.clone();
            let pod_name_ = pod_name.clone();
            let prefix_ = prefix.clone();
            let lp_ = lp.clone();
            let buf__ = buf_.clone();

            let stream: Result<(), kube::Error> = async move {
                let mut logs = pod_.log_stream(&pod_name_, &lp_).await?.boxed();

                while let Some(line) = logs.try_next().await? {
                    let mut buf = buf__.write().await;
                    buf.push(format!("{}{}", prefix_, String::from_utf8_lossy(&line)));
                }

                Ok(())
            }
            .await;

            let buf__ = buf_.clone();
            match stream {
                Ok(()) => break,
                Err(err) => match err {
                    kube::Error::HyperError(_) => {
                        lp.tail_lines = Some(0);
                    }
                    _ => {
                        let mut buf = buf__.write().await;
                        buf.push(msg::error(format!("log_stream ERR: {}", err)));
                        break;
                    }
                },
            }
        }
    });

    let buf_ = buf.clone();
    let send_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(time::Duration::from_millis(200));
        loop {
            interval.tick().await;
            let mut buf = buf_.write().await;
            if !buf.is_empty() {
                tx.send(Event::Kube(Kube::LogStreamResponse(buf.clone())))
                    .unwrap();

                buf.clear();
            }
        }
    });

    vec![buf_handle, send_handle]
}

fn is_terminated_container(terminated: &Option<ContainerStateTerminated>) -> bool {
    if terminated.is_some() {
        true
    } else {
        false
    }
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
    }

    impl Iterator for Color {
        type Item = u8;

        fn next(&mut self) -> Option<Self::Item> {
            if COLOR.len() <= self.index {
                self.index = 0;
            }
            self.index += 1;
            Some(COLOR[self.index - 1])
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn color_default() {
            let mut color = Color::new();
            assert_eq!(color.next().unwrap(), 32)
        }

        #[test]
        fn color_next_1() {
            let mut color = Color::new();
            color.next();
            assert_eq!(color.next().unwrap(), 33)
        }

        #[test]
        fn color_next_last() {
            let mut color = Color::new();
            color.next();
            color.next();
            color.next();
            color.next();
            color.next();
            assert_eq!(color.next().unwrap(), 37)
        }

        #[test]
        fn color_next_loop() {
            let mut color = Color::new();
            color.next();
            color.next();
            color.next();
            color.next();
            color.next();
            color.next();
            assert_eq!(color.next().unwrap(), 32)
        }
    }
}
