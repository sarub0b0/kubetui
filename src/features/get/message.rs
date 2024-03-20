use anyhow::Result;

use crate::{message::Message, workers::kube::message::Kube};

pub use super::kube::yaml::GetYamlKind;

#[derive(Debug)]
pub enum GetMessage {
    Request(GetRequest),
    Response(GetResponse),
}

#[derive(Debug, Clone)]
pub struct GetRequest {
    pub name: String,
    pub namespace: String,
    pub kind: GetYamlKind,
}

#[derive(Debug)]
pub struct GetResponse {
    pub kind: String,
    pub name: String,
    pub yaml: Result<Vec<String>>,
}

impl From<GetRequest> for Message {
    fn from(req: GetRequest) -> Self {
        Self::Kube(Kube::Get(GetMessage::Request(req)))
    }
}

impl From<GetResponse> for Message {
    fn from(res: GetResponse) -> Self {
        Self::Kube(Kube::Get(GetMessage::Response(res)))
    }
}
