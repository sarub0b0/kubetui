use std::sync::Arc;

use async_trait::async_trait;
use crossbeam::channel::Sender;
use tokio::{sync::Mutex, time};

use crate::{message::Message, send_response, workers::kube::Worker};

use super::log_content::LogContent;

pub type LogBuffer = Arc<Mutex<Vec<LogContent>>>;

#[derive(Clone)]
pub struct LogCollector {
    tx: Sender<Message>,
    buffer: LogBuffer,
    json_pretty_print: bool,
}

impl LogCollector {
    pub fn new(tx: Sender<Message>, buffer: LogBuffer, json_pretty_print: bool) -> Self {
        Self {
            tx,
            buffer,
            json_pretty_print,
        }
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

            let contents = std::mem::take(&mut *buf);

            if contents.is_empty() {
                continue;
            }

            let logs = if self.json_pretty_print {
                contents
                    .into_iter()
                    .flat_map(|content| content.try_json_pritty_print())
                    .collect()
            } else {
                contents
                    .into_iter()
                    .map(|content| content.print())
                    .collect()
            };

            send_response!(self.tx, Ok(logs));
        }
    }
}
