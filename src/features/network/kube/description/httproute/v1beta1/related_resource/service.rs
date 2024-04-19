use std::collections::BTreeSet;

use anyhow::Result;
use derivative::Derivative;
use k8s_openapi::{api::core::v1::Service, Resource};
use kube::{Api, Client, ResourceExt};
use serde::{Deserialize, Serialize};

use crate::{
    features::network::kube::description::utils::ExtractNamespace as _,
    kube::apis::networking::gateway::v1beta1::{HTTPRoute, HTTPRouteRulesBackendRefs},
    logger,
};

pub type RelatedServices = Vec<RelatedService>;

#[derive(Derivative, Debug, Clone, Serialize, Deserialize)]
#[derivative(PartialEq, Eq, Ord)]
pub struct RelatedService {
    /// Service Name
    pub name: String,

    /// Service Namespace
    pub namespace: String,

    /// HTTPRoute Name
    pub httproute: String,

    #[derivative(PartialEq = "ignore", PartialOrd = "ignore", Ord = "ignore")]
    #[serde(skip)]
    pub resource: Service,
}

impl PartialOrd for RelatedService {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

struct BackendRefs<'a> {
    refs: Vec<&'a HTTPRouteRulesBackendRefs>,
}

impl<'a> From<&'a HTTPRoute> for BackendRefs<'a> {
    fn from(value: &'a HTTPRoute) -> Self {
        let rules = value.spec.rules.as_ref();

        let refs: Vec<&HTTPRouteRulesBackendRefs> = rules
            .map(|rules| {
                rules
                    .iter()
                    .filter_map(|rule| rule.backend_refs.as_ref())
                    .flat_map(|backend_refs| backend_refs.iter())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        BackendRefs { refs }
    }
}

pub async fn discover_services(
    client: Client,
    httproute_name: &str,
    httproute_namespace: &str,
    httproute: &HTTPRoute,
) -> Result<Option<RelatedServices>> {
    let BackendRefs { refs } = BackendRefs::from(httproute);

    let mut result: BTreeSet<RelatedService> = BTreeSet::new();

    for r in refs {
        if r.group.as_ref().is_some_and(|g| !g.is_empty())
            || r.kind.as_ref().is_some_and(|k| k != Service::KIND)
        {
            continue;
        }

        let namespace = if let Some(namespace) = r.namespace.as_ref() {
            namespace
        } else {
            httproute_namespace
        };

        let api = Api::<Service>::namespaced(client.clone(), namespace);

        let Ok(service) = api.get(&r.name).await else {
            logger!(error, "failed to get service {namespace}/{{r.name}}");
            continue;
        };

        result.insert(RelatedService {
            name: service.name_any(),
            namespace: service.extract_namespace(),
            httproute: httproute_name.to_string(),
            resource: service,
        });
    }

    if result.is_empty() {
        Ok(None)
    } else {
        Ok(Some(result.into_iter().collect()))
    }
}
