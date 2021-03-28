use k8s_openapi::api::core::v1::ContainerStateTerminated;

use super::{Event, Kube};
use crate::kubernetes::Handlers;

use futures::{StreamExt, TryStreamExt};
use tokio::task::JoinHandle;

use std::time;
use std::{
    sync::{Arc, RwLock},
    vec,
};

use crossbeam::channel::Sender;

use k8s_openapi::api::core::v1::Pod;

use kube::{api::LogParams, Api, Client};

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
        let logs = pod.logs(&pod_name, &lp).await.unwrap();
        for line in logs.lines() {
            let mut buf = buf.write().unwrap();
            let prefix = if let Some(ref p) = log_prefix {
                p.to_owned() + " "
            } else {
                "".to_string()
            };
            buf.push(format!("{}{}", prefix, line));
        }
        let mut buf = buf.write().unwrap();
        tx.send(Event::Kube(Kube::LogStreamResponse(buf.clone())))
            .unwrap();

        buf.clear();
    })
}

fn container_log_stream(
    tx: Sender<Event>,
    pod: Api<Pod>,
    pod_name: String,
    lp: LogParams,
    buf: Arc<RwLock<Vec<String>>>,
    log_prefix: Option<String>,
) -> Vec<JoinHandle<()>> {
    let buf_clone = Arc::clone(&buf);
    let buf_handle = tokio::spawn(async move {
        let mut logs = pod.log_stream(&pod_name, &lp).await.unwrap().boxed();
        while let Some(line) = logs.try_next().await.unwrap() {
            let mut buf = buf_clone.write().unwrap();
            let prefix = if let Some(ref p) = log_prefix {
                p.to_owned() + " "
            } else {
                "".to_string()
            }
            .to_string();
            buf.push(format!("{}{}", prefix, String::from_utf8_lossy(&line)));
        }
    });

    let buf_clone = Arc::clone(&buf);
    let send_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(time::Duration::from_millis(200));
        loop {
            interval.tick().await;
            let mut buf = buf_clone.write().unwrap();
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
