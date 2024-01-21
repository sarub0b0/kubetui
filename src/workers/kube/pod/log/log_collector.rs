use std::sync::Arc;

use async_trait::async_trait;
use crossbeam::channel::Sender;
use tokio::{sync::Mutex, time};

use crate::{message::Message, send_response, workers::kube::worker::Worker};

pub type LogBuffer = Arc<Mutex<Vec<String>>>;

#[derive(Clone)]
pub struct LogCollector {
    tx: Sender<Message>,
    buffer: LogBuffer,
}

impl LogCollector {
    pub fn new(tx: Sender<Message>, buffer: LogBuffer) -> Self {
        Self { tx, buffer }
    }
}

/// 将来的にはチャネルにしたい
#[async_trait]
impl Worker for LogCollector {
    type Output = ();
    async fn run(&self) -> Self::Output {
        let mut interval = tokio::time::interval(time::Duration::from_millis(200));

        loop {
            interval.tick().await;

            let mut buf = self.buffer.lock().await;

            if !buf.is_empty() {
                send_response!(self.tx, Ok(std::mem::take(&mut buf)));
            }
        }
    }
}
