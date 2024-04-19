mod gateway;
mod pod;
mod service;

use anyhow::{Context as _, Result};
use kube::Client;
use serde::{Deserialize, Serialize};

use crate::kube::apis::networking::gateway::v1::HTTPRoute;

use self::{
    gateway::{discover_gateways, RelatedGateways},
    pod::{discover_pods, RelatedPods},
    service::{discover_services, RelatedServices},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HTTPRouteRelatedResources {
    related_resources: HTTPRouteRelatedResourceItems,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HTTPRouteRelatedResourceItems {
    #[serde(skip_serializing_if = "Option::is_none")]
    gateways: Option<RelatedGateways>,
    #[serde(skip_serializing_if = "Option::is_none")]
    services: Option<RelatedServices>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pods: Option<RelatedPods>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RelatedResource {
    name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    namespace: Option<String>,
}

pub async fn discover_releated_resources(
    client: Client,
    httproute_name: &str,
    httproute_namespace: &str,
    httproute: &HTTPRoute,
) -> Result<HTTPRouteRelatedResources> {
    let gateways = discover_gateways(
        client.clone(),
        httproute_name,
        httproute_namespace,
        httproute,
    )
    .await
    .with_context(|| "discover gateways for httproute")?;

    let services = discover_services(
        client.clone(),
        httproute_name,
        httproute_namespace,
        httproute,
    )
    .await
    .with_context(|| "discover services for httproute")?;

    let pods = if let Some(services) = services.as_ref() {
        discover_pods(client.clone(), services)
            .await
            .with_context(|| "discover pods for httproute")?
    } else {
        None
    };

    let related_resources = HTTPRouteRelatedResources {
        related_resources: HTTPRouteRelatedResourceItems {
            gateways,
            services,
            pods,
        },
    };

    Ok(related_resources)
}
