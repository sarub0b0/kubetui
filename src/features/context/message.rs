use crate::{message::Message, workers::message::Kube};

#[derive(Debug)]
pub enum ContextMessage {
    Request(ContextRequest),
    Response(ContextResponse),
}

#[derive(Debug)]
pub enum ContextRequest {
    Get,
    Set(String),
}

#[derive(Debug)]
pub enum ContextResponse {
    Get(Vec<String>),
}

impl From<ContextMessage> for Message {
    fn from(m: ContextMessage) -> Self {
        Message::Kube(Kube::Context(m))
    }
}

impl From<ContextRequest> for Message {
    fn from(m: ContextRequest) -> Self {
        Message::Kube(Kube::Context(ContextMessage::Request(m)))
    }
}

impl From<ContextResponse> for Message {
    fn from(m: ContextResponse) -> Self {
        Message::Kube(Kube::Context(ContextMessage::Response(m)))
    }
}
