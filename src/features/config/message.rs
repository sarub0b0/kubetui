use anyhow::Result;

use crate::{
    features::config::ConfigColumns,
    kube::table::KubeTable,
    message::Message,
    workers::kube::message::Kube,
};

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
    /// Replace the active labelSelector value. `None` clears it (the poller
    /// stops sending `?labelSelector=` in its sub-fetch URLs).
    Filter(Option<String>),
    /// Replace the active column composition (sent from the column dialog).
    /// The poller will use the new columns on the next poll.
    ColumnsRequest(ConfigColumns),
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

impl From<ConfigMessage> for Message {
    fn from(m: ConfigMessage) -> Self {
        Self::Kube(Kube::Config(m))
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
