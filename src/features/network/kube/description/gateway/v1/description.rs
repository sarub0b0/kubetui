use k8s_openapi::apimachinery::pkg::apis::meta::v1::Condition;
use kube::ResourceExt;
use serde::{Deserialize, Serialize};

use crate::kube::apis::networking::gateway::v1::{
    Gateway, GatewaySpec, GatewayStatusAddresses, GatewayStatusListeners,
    GatewayStatusListenersSupportedKinds,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Description {
    gateway: DescriptionGateway,
}

impl Description {
    pub fn new(gateway: Gateway) -> Self {
        Self {
            gateway: DescriptionGateway::new(gateway),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MetadataName {
    name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DescriptionGateway {
    metadata: MetadataName,

    spec: GatewaySpec,

    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<GatewayStatusWrapper>,
}

impl DescriptionGateway {
    fn new(gateway: Gateway) -> Self {
        let name = gateway.name_any();

        let Gateway {
            metadata: _,
            spec,
            status,
        } = gateway;

        let status_wrapper = status.map(|status| GatewayStatusWrapper {
            addresses: status.addresses,
            listeners: status.listeners.map(|listeners| {
                listeners
                    .into_iter()
                    .map(ListenerStatusWrapper::new)
                    .collect()
            }),
        });

        Self {
            metadata: MetadataName { name },
            spec,
            status: status_wrapper,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GatewayStatusWrapper {
    #[serde(skip_serializing_if = "Option::is_none")]
    addresses: Option<Vec<GatewayStatusAddresses>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    listeners: Option<Vec<ListenerStatusWrapper>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ListenerStatusWrapper {
    #[serde(flatten)]
    #[serde(with = "ListenerStatusDef")]
    status: GatewayStatusListeners,
}

impl ListenerStatusWrapper {
    fn new(status: GatewayStatusListeners) -> Self {
        Self { status }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "GatewayStatusListeners")]
#[serde(rename_all = "camelCase")]
struct ListenerStatusDef {
    attached_routes: i32,

    #[serde(skip)]
    conditions: Vec<Condition>,

    name: String,

    supported_kinds: Vec<GatewayStatusListenersSupportedKinds>,
}

