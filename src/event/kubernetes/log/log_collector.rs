use std::sync::Arc;

use async_trait::async_trait;
use crossbeam::channel::Sender;
use tokio::{sync::RwLock, time};

use crate::{
    event::{kubernetes::worker::Worker, Event},
    send_response,
};

pub type LogBuffer = Arc<RwLock<Vec<String>>>;

#[derive(Clone)]
pub struct LogCollector {
    tx: Sender<Event>,
    buffer: LogBuffer,
}

impl LogCollector {
    pub fn new(tx: Sender<Event>, buffer: LogBuffer) -> Self {
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

            let mut buf = self.buffer.write().await;

            if !buf.is_empty() {
                send_response!(self.tx, Ok(std::mem::take(&mut buf)));
            }
        }
    }
}
