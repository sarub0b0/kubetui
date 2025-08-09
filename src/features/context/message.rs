use crate::{message::Message, workers::kube::message::Kube};

#[derive(Debug)]
pub enum ContextMessage {
    Request(ContextRequest),
    Response(ContextResponse),
}

#[derive(Debug)]
pub enum ContextRequest {
    Get,

    /// 指定したコンテキストに切り替える
    Set {
        /// コンテキスト名
        name: String,

        /// 名前空間を維持するかどうか
        keep_namespace: bool,
    },
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
