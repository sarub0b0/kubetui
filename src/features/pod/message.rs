use anyhow::Result;

use crate::{message::Message, workers::kube::message::Kube};

use super::kube::LogConfig;

#[derive(Debug)]
pub enum LogMessage {
    Request(LogConfig),
    Response(Result<Vec<String>>),
    ToggleJsonPrettyPrint,
}

impl From<LogMessage> for Message {
    fn from(m: LogMessage) -> Message {
        Message::Kube(Kube::Log(m))
    }
}
