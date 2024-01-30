use anyhow::Result;

use crate::{message::Message, workers::Kube};

use super::kube::LogConfig;

#[derive(Debug)]
pub enum LogMessage {
    Request(LogConfig),
    Response(Result<Vec<String>>),
}

impl From<LogMessage> for Message {
    fn from(m: LogMessage) -> Message {
        Message::Kube(Kube::Log(m))
    }
}
