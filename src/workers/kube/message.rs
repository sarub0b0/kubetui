use anyhow::Result;

use crate::{
    features::{
        api_resources::message::ApiMessage, config::message::ConfigMessage,
        context::message::ContextMessage, get::message::GetMessage,
        namespace::message::NamespaceMessage, network::message::NetworkMessage,
        pod::message::LogMessage, yaml::message::YamlMessage,
    },
    kube::table::KubeTable,
    message::Message,
};

use super::controller::{StyledTargetApiResources, TargetNamespaces};

#[derive(Debug)]
pub enum Kube {
    Context(ContextMessage),
    Api(ApiMessage),
    RestoreAPIs(StyledTargetApiResources),
    RestoreContext {
        context: String,
        namespaces: TargetNamespaces,
    },
    Event(Result<Vec<String>>),
    Namespace(NamespaceMessage),
    Pod(Result<KubeTable>),
    Log(LogMessage),
    Config(ConfigMessage),
    Network(NetworkMessage),
    Yaml(YamlMessage),
    Get(GetMessage),
}

impl From<Kube> for Message {
    fn from(k: Kube) -> Self {
        Message::Kube(k)
    }
}
