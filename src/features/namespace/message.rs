use crate::{
    message::Message,
    workers::{message::Kube, TargetNamespaces},
};
use anyhow::Result;

#[derive(Debug)]
pub enum NamespaceMessage {
    Request(NamespaceRequest),
    Response(NamespaceResponse),
}

#[derive(Debug)]
pub enum NamespaceRequest {
    Get,
    Set(TargetNamespaces),
}

#[derive(Debug)]
pub enum NamespaceResponse {
    Get(Result<TargetNamespaces>),
    Set(TargetNamespaces),
}

impl From<NamespaceRequest> for Message {
    fn from(n: NamespaceRequest) -> Self {
        Message::Kube(Kube::Namespace(NamespaceMessage::Request(n)))
    }
}

impl From<NamespaceResponse> for Message {
    fn from(n: NamespaceResponse) -> Self {
        Message::Kube(Kube::Namespace(NamespaceMessage::Response(n)))
    }
}
