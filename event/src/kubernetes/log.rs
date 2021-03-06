use super::{Event, Kube};
use crate::kubernetes::Handlers;

use futures::{StreamExt, TryStreamExt};
use tokio::{sync::RwLock, task::JoinHandle};

use std::{sync::Arc, time};

use crossbeam::channel::Sender;

use k8s_openapi::api::core::v1::Pod;

use kube::{api::LogParams, Api, Client};

use color::Color;

use crate::error::{Error, Result};

type BufType = Arc<RwLock<Vec<String>>>;

pub async fn log_stream(tx: Sender<Event>, client: Client, ns: &str, pod_name: &str) -> Handlers {
    let pod: Api<Pod> = Api::namespaced(client, ns);
    let lp = LogParams {
        follow: true,
        ..Default::default()
    };

    // バッチでログストリームを渡す
    let buf = Arc::new(RwLock::new(Vec::new()));

    let mut container_handler = Vec::new();

    let mut container_count = 0;
    match pod.get(&pod_name).await {
        Ok(init) => {
            let status = init.status.unwrap();
            let mut color = Color::new();

            // initContainersのログ取得
            // まだ実行中ならlog_stream, 何かしらで実行終わっていればlogs
            let containers = status.init_container_statuses;
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

                if state.terminated.is_some() {
                    let handler = container_logs(
                        tx.clone(),
                        Arc::clone(&buf),
                        pod.clone(),
                        pod_name,
                        lp,
                        prefix,
                    );

                    handler.await.unwrap();
                } else {
                    let handlers = container_log_stream(
                        tx.clone(),
                        pod.clone(),
                        pod_name,
                        lp,
                        Arc::clone(&buf),
                        prefix,
                    );

                    container_handler.push(handlers);
                }
            }

            container_count += containers.len();

            let containers = status.container_statuses;

            container_count += containers.len();

            for c in &containers {
                let tx = tx.clone();

                let mut lp = lp.clone();

                lp.container = Some(c.name.clone());

                let prefix = if 1 < container_count {
                    Some(format!(
                        "\x1b[{}m[{}]\x1b[39m",
                        color.next().unwrap(),
                        c.name
                    ))
                } else {
                    None
                };

                let handlers = container_log_stream(
                    tx.clone(),
                    pod.clone(),
                    pod_name,
                    lp,
                    Arc::clone(&buf),
                    prefix,
                );

                container_handler.push(handlers);
            }

            let handler = tokio::spawn(send_loop(tx, buf));

            container_handler.push(handler);
        }
        Err(err) => tx
            .send(Event::Kube(Kube::LogStreamResponse(Err(Error::Kube(err)))))
            .unwrap(),
    }

    Handlers(container_handler)
}

async fn send_loop(tx: Sender<Event>, buf: BufType) {
    let mut interval = tokio::time::interval(time::Duration::from_millis(200));

    loop {
        interval.tick().await;
        let mut buf = buf.write().await;

        if !buf.is_empty() {
            #[cfg(feature = "logging")]
            ::log::debug!("log_stream Send log stream {}", buf.len());

            tx.send(Event::Kube(Kube::LogStreamResponse(Ok(buf.clone()))))
                .unwrap();

            buf.clear();
        }
    }
}

fn container_logs(
    tx: Sender<Event>,
    buf: BufType,
    pod: Api<Pod>,
    pod_name: &str,
    lp: LogParams,
    log_prefix: Option<String>,
) -> JoinHandle<()> {
    let pod_name = pod_name.to_string();

    tokio::spawn(async move {
        let prefix = if let Some(p) = log_prefix {
            p + " "
        } else {
            "".to_string()
        };

        let logs = pod.logs(&pod_name, &lp).await.unwrap();

        for line in logs.lines() {
            let mut wbuf = buf.write().await;
            wbuf.push(format!("{}{}", prefix, line));
        }

        let mut wbuf = buf.write().await;

        tx.send(Event::Kube(Kube::LogStreamResponse(Ok(wbuf.clone()))))
            .unwrap();

        wbuf.clear();
    })
}

struct LogStreamArgs {
    pod: Api<Pod>,
    pod_name: String,
    prefix: Option<String>,
    lp: LogParams,
}

#[cfg(not(any(feature = "mock", feature = "mock-failed")))]
async fn get_log_stream(buf: BufType, args: LogStreamArgs) -> Result<()> {
    let LogStreamArgs {
        pod,
        pod_name,
        prefix,
        lp,
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
            "log_stream {}: {}",
            pod_name,
            String::from_utf8_lossy(&line)
        );
    }

    #[cfg(feature = "logging")]
    ::log::info!("log_stream finished {}", pod_name);
    Ok(())
}

#[cfg(feature = "mock")]
async fn get_log_stream(buf: BufType, _: LogStreamArgs) -> Result<()> {
    async {
        let stream = vec!["line 0", "line 1", "line 2", "line 3", "line 4"];

        for s in stream {
            let mut buf = buf.write().await;
            buf.push(s.to_string());
        }
    }
    .await;

    Err(Error::Mock("get_log_stream failed"))
}

#[cfg(feature = "mock-failed")]
async fn get_log_stream(buf: BufType, _: LogStreamArgs) -> Result<()> {
    Err(Error::Mock("get_log_stream failed"))
}

fn container_log_stream(
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
            lp,
            prefix: log_prefix,
        };

        let stream = get_log_stream(buf, args).await;

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
