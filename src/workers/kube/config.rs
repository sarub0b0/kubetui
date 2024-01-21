mod data;
mod list;

pub use data::ConfigsDataWorker;
pub use list::ConfigsPollWorker;

use super::{Kube, KubeTable, Message};

use anyhow::Result;

pub type ConfigData = Vec<String>;

#[derive(Debug, Clone)]
pub struct RequestData {
    pub name: String,
    pub namespace: String,
}

#[derive(Debug)]
pub enum ConfigMessage {
    Request(ConfigRequest),
    Response(ConfigResponse),
}

#[derive(Debug, Clone)]
pub enum ConfigRequest {
    ConfigMap(RequestData),
    Secret(RequestData),
}

#[derive(Debug)]
pub enum ConfigResponse {
    Table(Result<KubeTable>),
    Data(Result<ConfigData>),
}

impl ConfigRequest {
    pub fn data(&self) -> &RequestData {
        match self {
            Self::ConfigMap(data) => data,
            Self::Secret(data) => data,
        }
    }
}

impl From<ConfigMessage> for Kube {
    fn from(m: ConfigMessage) -> Self {
        Self::Config(m)
    }
}

impl From<ConfigMessage> for Message {
    fn from(m: ConfigMessage) -> Self {
        Self::Kube(m.into())
    }
}

impl From<ConfigRequest> for Message {
    fn from(req: ConfigRequest) -> Self {
        ConfigMessage::Request(req).into()
    }
}

impl From<ConfigResponse> for Message {
    fn from(res: ConfigResponse) -> Self {
        ConfigMessage::Response(res).into()
    }
}
