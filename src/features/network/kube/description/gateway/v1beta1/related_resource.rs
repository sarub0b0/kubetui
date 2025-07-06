mod httproute;
mod pod;
mod service;

use anyhow::{Context as _, Result};
use kube::Client;
use serde::{Deserialize, Serialize};

use crate::kube::apis::networking::gateway::v1beta1::Gateway;

use self::{
    httproute::{RelatedHTTPRoutes, discover_httproutes},
    pod::{RelatedPods, discover_pods},
    service::{RelatedServices, discover_services},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayRelatedResources {
    related_resources: GatewayRelatedResourceItems,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GatewayRelatedResourceItems {
    #[serde(skip_serializing_if = "Option::is_none")]
    httproutes: Option<RelatedHTTPRoutes>,
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
    gateway_name: &str,
    gateway_namespace: &str,
    gateway: &Gateway,
) -> Result<GatewayRelatedResources> {
    let httproutes = discover_httproutes(client.clone(), gateway_name, gateway_namespace, gateway)
        .await
        .with_context(|| "discover httproutes for gateway")?;

    let services = if let Some(httproutes) = httproutes.as_ref() {
        discover_services(client.clone(), httproutes)
            .await
            .with_context(|| "discover services for gateway")?
    } else {
        None
    };

    let pods = if let Some(services) = services.as_ref() {
        discover_pods(client.clone(), services)
            .await
            .with_context(|| "discover pods for gateway")?
    } else {
        None
    };

    let related_resources = GatewayRelatedResources {
        related_resources: GatewayRelatedResourceItems {
            httproutes,
            services,
            pods,
        },
    };

    Ok(related_resources)
}
