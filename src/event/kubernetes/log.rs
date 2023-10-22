mod log_collector;
mod log_stream;
mod watch;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use crossbeam::channel::Sender;
use tokio::task::JoinHandle;

use crate::event::Event;

use self::{
    log_collector::{LogBuffer, LogCollector},
    watch::PodWatcher,
};

use super::{client::KubeClient, color::fg::Color, worker::Worker, Kube};

#[macro_export]
macro_rules! send_response {
    ($tx:expr, $msg:expr) => {
        use $crate::event::kubernetes::log::LogStreamMessage;

        $tx.send(LogStreamMessage::Response($msg).into())
            .expect("Failed to send LogStreamMessage::Response");
    };
}

#[derive(Debug)]
pub struct LogHandlers {
    log_collector_handler: JoinHandle<()>,
    log_stream_handler: JoinHandle<()>,
}

impl LogHandlers {
    pub fn abort(&self) {
        self.log_collector_handler.abort();
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
            message_buffer: LogBuffer::default(),
            target: self.target,
        }
    }
}

#[derive(Clone)]
pub struct LogWorker {
    tx: Sender<Event>,
    client: KubeClient,
    namespace: String,
    message_buffer: LogBuffer,
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
            message_buffer: LogBuffer::default(),
            target: target.into(),
        }
    }
}

impl LogWorker {
    pub fn spawn(&self) -> LogHandlers {
        let log_collector_handler =
            LogCollector::new(self.tx.clone(), self.message_buffer.clone()).spawn();

        let log_stream_handler = LogStreamController::new(
            self.tx.clone(),
            self.client.clone(),
            self.namespace.clone(),
            self.message_buffer.clone(),
            self.target.clone(),
        )
        .spawn();

        LogHandlers {
            log_collector_handler,
            log_stream_handler,
        }
    }
}

#[derive(Clone)]
struct LogStreamController {
    tx: Sender<Event>,
    client: KubeClient,
    namespace: String,
    message_buffer: LogBuffer,
    target: String,
}

impl LogStreamController {
    fn new(
        tx: Sender<Event>,
        client: KubeClient,
        namespace: String,
        message_buffer: LogBuffer,
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
}

#[async_trait]
impl Worker for LogStreamController {
    type Output = ();

    async fn run(&self) -> Self::Output {
        let handler = PodWatcher::new(
            self.tx.clone(),
            self.client.clone(),
            self.message_buffer.clone(),
            self.namespace.clone(),
            self.target.clone(),
        )
        .spawn();

        if let Err(err) = handler.await {
            send_response!(self.tx, Err(anyhow!(err)));
        }
    }
}
