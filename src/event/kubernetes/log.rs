use std::sync::Arc;

use anyhow::{bail, Result};
use async_trait::async_trait;
use chrono::{DateTime, FixedOffset, Utc};
use crossbeam::channel::Sender;
use futures::{
    future::join_all,
    stream::{repeat, repeat_with},
    AsyncBufReadExt, FutureExt, Stream, StreamExt, TryFutureExt, TryStreamExt,
};
use itertools::Itertools;
use k8s_openapi::api::core::v1::{Container, Pod};
use kube::{api::LogParams, Api};
use tokio::{sync::RwLock, task::JoinHandle, time};

use crate::event::Event;

use super::{client::KubeClient, color::Color, worker::Worker, Kube};

#[derive(Debug)]
pub struct LogHandlers {
    send_message_handler: JoinHandle<()>,
    log_stream_handler: JoinHandle<()>,
}

impl LogHandlers {
    pub fn abort(&self) {
        self.send_message_handler.abort();
        self.log_stream_handler.abort();
    }
}

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

pub struct LogWorkerBuilder {
    tx: Sender<Event>,
    client: KubeClient,
    namespace: String,
    target: String,
}

impl LogWorkerBuilder {
    pub fn new(
        tx: Sender<Event>,
        client: KubeClient,
        namespace: impl Into<String>,
        target: impl Into<String>,
    ) -> Self {
        Self {
            tx,
            client,
            namespace: namespace.into(),
            target: target.into(),
        }
    }

    pub fn build(self) -> LogWorker {
        LogWorker {
            tx: self.tx,
            client: self.client,
            namespace: self.namespace,
            message_buffer: MessageBuffer::default(),
            target: self.target,
        }
    }
}

type MessageBuffer = Arc<RwLock<Vec<String>>>;

#[derive(Clone)]
pub struct LogWorker {
    tx: Sender<Event>,
    client: KubeClient,
    namespace: String,
    message_buffer: MessageBuffer,
    target: String,
}

impl LogWorker {
    pub fn new(
        tx: Sender<Event>,
        client: KubeClient,
        namespace: impl Into<String>,
        target: impl Into<String>,
    ) -> Self {
        Self {
            tx,
            client,
            namespace: namespace.into(),
            message_buffer: MessageBuffer::default(),
            target: target.into(),
        }
    }
}

impl LogWorker {
    pub fn spawn(&self) -> LogHandlers {
        let send_message_handler =
            SendMessageWorker::new(self.tx.clone(), self.message_buffer.clone()).spawn();

        let log_stream_handler = LogStreamWorker::new(
            self.tx.clone(),
            self.client.clone(),
            self.namespace.clone(),
            self.message_buffer.clone(),
            self.target.clone(),
        )
        .spawn();

        LogHandlers {
            send_message_handler,
            log_stream_handler,
        }
    }
}

#[derive(Clone)]
struct LogStreamWorker {
    tx: Sender<Event>,
    client: KubeClient,
    namespace: String,
    message_buffer: MessageBuffer,
    target: String,
}

#[async_trait]
impl Worker for LogStreamWorker {
    type Output = ();
    async fn run(&self) -> Self::Output {
        match self.targets().await {
            Ok(targets) => {
                let target_handlers = targets.spawn_tasks().await;

                join_all(target_handlers).await;
            }
            Err(err) => {
                self.tx
                    .send(LogStreamMessage::Response(Err(err)).into())
                    .expect("Failed to send LogStreamMessage::Response");
            }
        }
    }
}

impl LogStreamWorker {
    fn new(
        tx: Sender<Event>,
        client: KubeClient,
        namespace: String,
        message_buffer: MessageBuffer,
        target: String,
    ) -> Self {
        Self {
            tx,
            client,
            namespace,
            message_buffer,
            target,
        }
    }

    async fn targets(&self) -> Result<Targets> {
        let pod_api: Api<Pod> = Api::namespaced(self.client.as_client().clone(), &self.namespace);

        let pod = pod_api.get(&self.target).await?;

        if let Some(spec) = &pod.spec {
            let targets = spec
                .clone()
                .init_containers
                .unwrap_or_default()
                .iter()
                .chain(spec.containers.iter())
                .scan(Color::new(), |color, c| {
                    Some(Target {
                        pod_name: self.target.to_string(),
                        pod_api: pod_api.clone(),
                        container: c.clone(),
                        prefix_color: color.next_color(),
                        message_buffer: self.message_buffer.clone(),
                        last_timestamp: None,
                    })
                })
                .collect();

            Ok(Targets(targets))
        } else {
            bail!("Not found pod.spec {}.", self.target);
        }
    }
}

#[derive(Clone)]
struct Targets(Vec<Target>);

impl Targets {
    async fn spawn_tasks(&self) -> Vec<JoinHandle<()>> {
        self.0.iter().map(Worker::spawn).collect()
    }
}

#[derive(Debug, Clone)]
struct Target {
    pod_name: String,
    pod_api: Api<Pod>,
    container: Container,
    prefix_color: u8,
    message_buffer: MessageBuffer,
    last_timestamp: Option<DateTime<FixedOffset>>,
}

#[async_trait]
impl Worker for Target {
    type Output = ();
    async fn run(&self) -> Self::Output {
        let mut target = self.clone();

        let mut interval = tokio::time::interval(time::Duration::from_secs(5));

        let mut prev_pod = None;

        loop {
            let Ok(pod) = self.pod_api.get(&self.pod_name).await else {
                break;
            };

            if let Err(err) = target.fetch().await {
                let mut buf = self.message_buffer.write().await;

                err.to_string()
                    .lines()
                    .for_each(|line| buf.push(format!("\x1b[31m{}\x1b[0m", line)))


            }

            prev_pod = Some(pod);

            interval.tick().await;
        }
    }
}

impl Target {
    async fn fetch(&mut self) -> Result<()> {
        let container_name = &self.container.name;

        let prefix = format!("\x1b[{}m[{}]\x1b[0m ", self.prefix_color, container_name);

        let since_seconds = self
            .last_timestamp
            .map(|last| (Utc::now().fixed_offset() - last).num_seconds());

        let lp = LogParams {
            follow: true,
            container: Some(self.container.name.to_string()),
            timestamps: true,
            since_seconds,

            ..Default::default()
        };

        let mut logs = self.pod_api.log_stream(&self.pod_name, &lp).await?.lines();

        while let Some(line) = logs.try_next().await? {
            let mut buf = self.message_buffer.write().await;

            if let Ok((dt, content)) = chrono::DateTime::parse_and_remainder(&line, "%+ ") {
                buf.push(format!("{}{}", prefix, content));

                self.last_timestamp = Some(dt);
            } else {
                buf.push(format!("{}{}", prefix, line));
            }
        }

        Ok(())
    }
}

#[derive(Clone)]
struct SendMessageWorker {
    tx: Sender<Event>,
    message_buffer: MessageBuffer,
}

impl SendMessageWorker {
    fn new(tx: Sender<Event>, message_buffer: MessageBuffer) -> Self {
        Self { tx, message_buffer }
    }
}

#[async_trait]
impl Worker for SendMessageWorker {
    type Output = ();
    async fn run(&self) -> Self::Output {
        let mut interval = tokio::time::interval(time::Duration::from_millis(200));

        loop {
            interval.tick().await;
            let mut buf = self.message_buffer.write().await;

            if !buf.is_empty() {
                self.tx
                    .send(LogStreamMessage::Response(Ok(std::mem::take(&mut buf))).into())
                    .expect("Failed to send LogStreamMessage::Response");
            }
        }
    }
}
