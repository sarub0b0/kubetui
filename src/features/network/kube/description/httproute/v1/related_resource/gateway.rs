use anyhow::Result;
use k8s_openapi::Resource as _;
use serde::{Deserialize, Serialize};

use crate::{
    kube::apis::networking::gateway::v1::{Gateway, HTTPRoute},
    logger,
};

pub type RelatedGateways = Vec<RelatedGateway>;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RelatedGateway {
    /// Gateway Name
    pub name: String,

    /// Gateway Namespace
    pub namespace: String,
}

struct ParentRef {
    group: String,
    kind: String,
    name: String,
    namespace: String,
}

fn extract_parent_refs(httproute: &HTTPRoute, httproute_namespace: &str) -> Option<Vec<ParentRef>> {
    httproute.spec.parent_refs.as_ref().map(|parent_refs| {
        parent_refs
            .iter()
            .map(|parent_ref| ParentRef {
                group: parent_ref
                    .group
                    .as_ref()
                    .map_or_else(|| Gateway::GROUP.to_string(), String::to_string),
                kind: parent_ref
                    .kind
                    .as_ref()
                    .map_or_else(|| Gateway::KIND.to_string(), String::to_string),
                name: parent_ref.name.clone(),
                namespace: parent_ref
                    .namespace
                    .as_ref()
                    .map_or_else(|| httproute_namespace.to_string(), String::to_string),
            })
            .collect()
    })
}

pub async fn discover_gateways(
    httproute_namespace: &str,
    httproute: &HTTPRoute,
) -> Result<Option<RelatedGateways>> {
    let Some(parent_refs) = extract_parent_refs(httproute, httproute_namespace) else {
        return Ok(None);
    };

    let mut result: Vec<_> = parent_refs.iter().flat_map(|ParentRef { group, kind, name, namespace }|{
        if group != Gateway::GROUP || kind != Gateway::KIND {
            logger!(
                warn,
                "ParentRef is not a Gateway, skipping. Group: {group}, Kind: {kind} namespace: {namespace} name: {name}",
            );

            return None;
        }

        Some(RelatedGateway {
            name: name.clone(),
            namespace: namespace.clone(),
        })
    }).collect();

    result.sort();

    if result.is_empty() {
        Ok(None)
    } else {
        Ok(Some(result))
    }
}
