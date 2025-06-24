use anyhow::Result;

use crate::{kube::table::KubeTable, message::Message, workers::kube::message::Kube};

use super::{kube::LogConfig, PodColumns};

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

#[derive(Debug)]
pub enum PodMessage {
    Request(PodColumns),
    Poll(Result<KubeTable>),
}

impl From<PodMessage> for Message {
    fn from(m: PodMessage) -> Message {
        Message::Kube(Kube::Pod(m))
    }
}
