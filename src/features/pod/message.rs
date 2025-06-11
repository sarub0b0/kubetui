use anyhow::Result;

use crate::{message::Message, workers::kube::message::Kube};

use super::kube::LogConfig;

#[derive(Debug)]
pub enum PodColumnsRequest {
    Get,
    Set(Vec<&'static str>),
}

impl From<PodColumnsRequest> for Message {
    fn from(req: PodColumnsRequest) -> Message {
        Message::Kube(Kube::PodColumns(PodColumnsMessage::Request(req)))
    }
}

#[derive(Debug)]
pub struct PodColumnsResponse {
    pub columns: Result<Vec<String>>,
}

impl From<PodColumnsResponse> for Message {
    fn from(res: PodColumnsResponse) -> Message {
        Message::Kube(Kube::PodColumns(PodColumnsMessage::Response(res)))
    }
}

#[derive(Debug)]
pub enum PodColumnsMessage {
    Request(PodColumnsRequest),
    Response(PodColumnsResponse),
}

impl From<PodColumnsMessage> for Message {
    fn from(m: PodColumnsMessage) -> Message {
        Message::Kube(Kube::PodColumns(m))
    }
}

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
