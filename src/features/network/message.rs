use anyhow::Result;
use strum::EnumString;

use crate::{kube::table::KubeTable, message::Message, workers::kube::message::Kube};

#[derive(Copy, Clone, Default, Debug, EnumString)]
#[strum(serialize_all = "lowercase")]
pub enum GatewayVersion {
    #[default]
    V1,
    V1Beta1,
}

#[derive(Copy, Clone, Default, Debug, EnumString)]
#[strum(serialize_all = "lowercase")]
pub enum HTTPRouteVersion {
    #[default]
    V1,
    V1Beta1,
}

#[derive(Debug, Clone)]
pub struct NetworkRequestTargetParams {
    pub name: String,
    pub namespace: String,
    pub version: String,
}

#[derive(Debug, Clone)]
pub enum NetworkRequest {
    Pod(NetworkRequestTargetParams),
    Service(NetworkRequestTargetParams),
    Ingress(NetworkRequestTargetParams),
    NetworkPolicy(NetworkRequestTargetParams),
    Gateway(NetworkRequestTargetParams),
    HTTPRoute(NetworkRequestTargetParams),
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
    pub fn data(&self) -> &NetworkRequestTargetParams {
        match self {
            Self::Pod(data) => data,
            Self::Service(data) => data,
            Self::Ingress(data) => data,
            Self::NetworkPolicy(data) => data,
            Self::Gateway(data) => data,
            Self::HTTPRoute(data) => data,
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
