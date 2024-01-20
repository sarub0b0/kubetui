mod description;
mod list;

pub use description::*;
pub use list::*;

use crate::{error::Result, event::Event};

use super::{Kube, KubeTable};

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

impl From<NetworkMessage> for Event {
    fn from(m: NetworkMessage) -> Self {
        Self::Kube(m.into())
    }
}

impl From<NetworkRequest> for Event {
    fn from(req: NetworkRequest) -> Self {
        NetworkMessage::Request(req).into()
    }
}

impl From<NetworkResponse> for Event {
    fn from(res: NetworkResponse) -> Self {
        NetworkMessage::Response(res).into()
    }
}
