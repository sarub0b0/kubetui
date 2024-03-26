use anyhow::{Context as _, Result};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::Condition;
use kube::{Api, ResourceExt};
use serde::{Deserialize, Serialize};

use crate::kube::{
    apis::networking::gateway::v1::{
        Gateway, GatewaySpec, GatewayStatusAddress, ListenerStatus, RouteGroupKind,
    },
    KubeClientRequest,
};

use super::{Fetch, FetchedData};

pub(super) struct GatewayDescriptionWorker<'a, C>
where
    C: KubeClientRequest,
{
    client: &'a C,
    namespace: String,
    name: String,
}

#[async_trait::async_trait]
impl<'a, C> Fetch<'a, C> for GatewayDescriptionWorker<'a, C>
where
    C: KubeClientRequest,
{
    fn new(client: &'a C, namespace: String, name: String) -> Self {
        Self {
            client,
            namespace,
            name,
        }
    }

    async fn fetch(&self) -> Result<FetchedData> {
        let api = Api::<Gateway>::namespaced(self.client.client().clone(), &self.namespace);

        let gateway = api.get(&self.name).await.context(format!(
            "Failed to fetch Gateway: namespace={}, name={}",
            self.namespace, self.name
        ))?;

        let description = Description::new(gateway.clone());

        let yaml = serde_yaml::to_string(&description)?
            .lines()
            .map(ToString::to_string)
            .collect::<Vec<String>>();

        Ok(yaml)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Description {
    gateway: DescriptionGateway,
}

impl Description {
    fn new(gateway: Gateway) -> Self {
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
    addresses: Option<Vec<GatewayStatusAddress>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    listeners: Option<Vec<ListenerStatusWrapper>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ListenerStatusWrapper {
    #[serde(flatten)]
    #[serde(with = "ListenerStatusDef")]
    status: ListenerStatus,
}

impl ListenerStatusWrapper {
    fn new(status: ListenerStatus) -> Self {
        Self { status }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "ListenerStatus")]
#[serde(rename_all = "camelCase")]
struct ListenerStatusDef {
    attached_routes: i32,

    #[serde(skip)]
    conditions: Vec<Condition>,

    name: String,

    supported_kinds: Vec<RouteGroupKind>,
}
