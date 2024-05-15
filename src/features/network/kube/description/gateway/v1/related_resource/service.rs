use anyhow::Result;
use derivative::Derivative;
use futures::StreamExt as _;
use k8s_openapi::{api::core::v1::Service, Resource};
use kube::{Api, Client, ResourceExt};
use serde::{Deserialize, Serialize};

use crate::{
    features::network::kube::description::utils::ExtractNamespace as _,
    kube::apis::networking::gateway::v1::HTTPRouteRulesBackendRefs, logger,
};

use super::httproute::RelatedHTTPRoute;

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
    httproute_name: &'a str,
    httproute_namespace: &'a str,
    refs: Vec<&'a HTTPRouteRulesBackendRefs>,
}

impl<'a> From<&'a RelatedHTTPRoute> for Option<BackendRefs<'a>> {
    fn from(value: &'a RelatedHTTPRoute) -> Self {
        let rules = value.resource.spec.rules.as_ref();

        let refs: Vec<&HTTPRouteRulesBackendRefs> = rules.map(|rules| {
            rules
                .iter()
                .filter_map(|rule| rule.backend_refs.as_ref())
                .flat_map(|backend_refs| backend_refs.iter())
                .collect::<Vec<_>>()
        })?;

        Some(BackendRefs {
            httproute_name: value.name.as_ref(),
            httproute_namespace: value.namespace.as_ref(),
            refs,
        })
    }
}

pub async fn discover_services(
    client: Client,
    httproutes: &[RelatedHTTPRoute],
) -> Result<Option<RelatedServices>> {
    let backend_refs: Vec<BackendRefs> = httproutes.iter().filter_map(Option::from).collect();

    let fetch_service_args: Vec<_> = backend_refs
        .into_iter()
        .flat_map(|refs| {
            refs.refs.into_iter().map(move |r| {
                (
                    refs.httproute_name.to_string(),
                    refs.httproute_namespace.to_string(),
                    r.clone(),
                )
            })
        })
        .collect();

    let task = tokio::spawn(async move {
        let futures = fetch_service_args.into_iter().map(|args| {
            let client = client.clone();
            async move { fetch_service(client, args.0, args.1, args.2).await }
        });

        let stream = futures::stream::iter(futures).buffer_unordered(20);

        let result: Vec<Option<RelatedService>> = stream.collect().await;

        result.into_iter().flatten().collect::<Vec<_>>()
    });

    let mut result = task.await?;

    result.sort();

    if result.is_empty() {
        Ok(None)
    } else {
        Ok(Some(result))
    }
}

async fn fetch_service(
    client: Client,
    httproute_name: String,
    httproute_namespace: String,
    r: HTTPRouteRulesBackendRefs,
) -> Option<RelatedService> {
    if r.group.as_ref().is_some_and(|g| !g.is_empty())
        || r.kind.as_ref().is_some_and(|k| k != Service::KIND)
    {
        return None;
    }

    let namespace = if let Some(namespace) = r.namespace.as_ref() {
        namespace
    } else {
        &httproute_namespace
    };

    let api = Api::<Service>::namespaced(client, namespace);

    match api.get(&r.name).await {
        Ok(service) => Some(RelatedService {
            name: service.name_any(),
            namespace: service.extract_namespace(),
            httproute: httproute_name.to_string(),
            resource: service,
        }),

        Err(err) => {
            logger!(
                error,
                "failed to get service {namespace}/{name}: {err}",
                name = r.name
            );

            None
        }
    }
}
