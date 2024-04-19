use anyhow::Result;
use derivative::Derivative;
use k8s_openapi::api::core::v1::Pod;
use kube::{api::ListParams, Api, Client, ResourceExt as _};
use serde::{Deserialize, Serialize};

use crate::features::network::kube::description::utils::ExtractNamespace as _;

use super::service::RelatedService;

pub type RelatedPods = Vec<RelatedPod>;

#[derive(Derivative, Debug, Clone, Serialize, Deserialize)]
#[derivative(PartialEq, Eq, Ord)]
pub struct RelatedPod {
    /// Pod Name
    pub name: String,

    /// Pod Namespace
    pub namespace: String,

    /// Service Name
    pub service: String,

    #[derivative(PartialEq = "ignore", PartialOrd = "ignore", Ord = "ignore")]
    #[serde(skip)]
    pub resource: Pod,
}

impl PartialOrd for RelatedPod {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

pub async fn discover_pods(
    client: Client,
    services: &[RelatedService],
) -> Result<Option<RelatedPods>> {
    let mut result = Vec::new();

    for svc in services {
        let Some(spec) = svc.resource.spec.as_ref() else {
            continue;
        };

        let Some(selector) = spec.selector.as_ref() else {
            continue;
        };

        let label_selector = selector_to_query(selector);

        let lp = ListParams::default().labels(&label_selector);

        let api = Api::<Pod>::namespaced(client.clone(), &svc.namespace);

        let pods = api.list(&lp).await?;

        for pod in pods {
            result.push(RelatedPod {
                name: pod.name_any(),
                namespace: pod.extract_namespace(),
                service: svc.name.clone(),
                resource: pod,
            });
        }
    }

    if result.is_empty() {
        Ok(None)
    } else {
        Ok(Some(result))
    }
}

fn selector_to_query(selector: &std::collections::BTreeMap<String, String>) -> String {
    selector
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<String>>()
        .join(",")
}
