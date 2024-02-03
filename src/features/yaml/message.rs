use anyhow::Result;

use crate::{features::api_resources::kube::ApiResource, message::Message, workers::message::Kube};

pub use super::kube::YamlTarget;

#[derive(Debug, Clone)]
pub struct YamlResourceListItem {
    pub kind: ApiResource,
    pub name: String,
    pub namespace: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct YamlResourceList {
    pub items: Vec<YamlResourceListItem>,
}

impl YamlResourceList {
    pub fn new(items: Vec<YamlResourceListItem>) -> Self {
        YamlResourceList { items }
    }
}

#[derive(Debug)]
pub enum YamlRequest {
    APIs,
    Resource(ApiResource),
    Yaml(YamlTarget),
}

impl From<YamlRequest> for Message {
    fn from(req: YamlRequest) -> Self {
        Message::Kube(Kube::Yaml(YamlMessage::Request(req)))
    }
}

#[derive(Debug)]
pub enum YamlResponse {
    APIs(Result<Vec<ApiResource>>),
    Resource(Result<YamlResourceList>),
    Yaml(Result<Vec<String>>),
}

impl From<YamlResponse> for Message {
    fn from(res: YamlResponse) -> Self {
        Message::Kube(Kube::Yaml(YamlMessage::Response(res)))
    }
}

#[derive(Debug)]
pub enum YamlMessage {
    Request(YamlRequest),
    Response(YamlResponse),
}

impl From<YamlMessage> for Kube {
    fn from(m: YamlMessage) -> Self {
        Self::Yaml(m)
    }
}

impl From<YamlMessage> for Message {
    fn from(m: YamlMessage) -> Self {
        Self::Kube(m.into())
    }
}
