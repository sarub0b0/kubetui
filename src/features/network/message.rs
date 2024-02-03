use anyhow::Result;

use crate::{kube::table::KubeTable, message::Message, workers::message::Kube};

#[derive(Debug, Clone)]
pub struct RequestData {
    pub name: String,
    pub namespace: String,
}

#[derive(Debug, Clone)]
pub enum NetworkRequest {
    Pod(RequestData),
    Service(RequestData),
    Ingress(RequestData),
    NetworkPolicy(RequestData),
}

#[derive(Debug)]
pub enum NetworkResponse {
    List(Result<KubeTable>),
    Yaml(Result<Vec<String>>),
}

#[derive(Debug)]
pub enum NetworkMessage {
    Request(NetworkRequest),
    Response(NetworkResponse),
}

impl NetworkRequest {
    pub fn data(&self) -> &RequestData {
        match self {
            Self::Pod(data) => data,
            Self::Service(data) => data,
            Self::Ingress(data) => data,
            Self::NetworkPolicy(data) => data,
        }
    }
}

impl From<NetworkMessage> for Kube {
    fn from(m: NetworkMessage) -> Self {
        Self::Network(m)
    }
}

impl From<NetworkMessage> for Message {
    fn from(m: NetworkMessage) -> Self {
        Self::Kube(m.into())
    }
}

impl From<NetworkRequest> for Message {
    fn from(req: NetworkRequest) -> Self {
        NetworkMessage::Request(req).into()
    }
}

impl From<NetworkResponse> for Message {
    fn from(res: NetworkResponse) -> Self {
        NetworkMessage::Response(res).into()
    }
}
