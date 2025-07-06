use anyhow::Result;
use futures::StreamExt;
use k8s_openapi::api::core::v1::{Pod, Service};
use kube::{api::ListParams, Api, Client, ResourceExt as _};
use serde::{Deserialize, Serialize};

use crate::features::network::kube::description::utils::ExtractNamespace as _;

use super::service::RelatedService;

pub type RelatedPods = Vec<RelatedPod>;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RelatedPod {
    /// Pod Name
    pub name: String,

    /// Pod Namespace
    pub namespace: String,

    /// Service Name
    pub service: String,
}

pub async fn discover_pods(
    client: Client,
    services: &[RelatedService],
) -> Result<Option<RelatedPods>> {
    let services = services.to_vec();

    let task = tokio::spawn(async move {
        let futures = futures::stream::iter(services.into_iter().map(|svc| {
            let client = client.clone();
            let svc_name = svc.name.clone();
            let svc_namespace = svc.namespace.clone();
            let svc = svc.resource.clone();

            async move { fetch_pods(client, svc_name, svc_namespace, svc).await }
        }))
        .buffer_unordered(20);

        let result: Vec<Option<Vec<RelatedPod>>> = futures.collect::<Vec<_>>().await;

        result.into_iter().flatten().flatten().collect::<Vec<_>>()
    });

    let mut result = task.await?;

    result.sort();

    if result.is_empty() {
        Ok(None)
    } else {
        Ok(Some(result))
    }
}

async fn fetch_pods(
    client: Client,
    svc_name: String,
    svc_namespace: String,
    svc: Service,
) -> Option<Vec<RelatedPod>> {
    let spec = svc.spec.as_ref()?;

    let selector = spec.selector.as_ref()?;

    let label_selector = selector_to_query(selector);

    let lp = ListParams::default().labels(&label_selector);

    let api = Api::<Pod>::namespaced(client.clone(), &svc_namespace);

    match api.list(&lp).await {
        Ok(pods) => Some(
            pods.into_iter()
                .map(|pod| RelatedPod {
                    name: pod.name_any(),
                    namespace: pod.extract_namespace(),
                    service: svc_name.clone(),
                })
                .collect::<Vec<_>>(),
        ),
        Err(_) => None,
    }
}

fn selector_to_query(selector: &std::collections::BTreeMap<String, String>) -> String {
    selector
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<String>>()
        .join(",")
}
