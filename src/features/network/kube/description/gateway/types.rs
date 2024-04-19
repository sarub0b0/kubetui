use anyhow::{Context as _, Result};
use derivative::Derivative;
use k8s_openapi::NamespaceResourceScope;
use kube::Client;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

trait CommonResource:
    kube::Resource<DynamicType = (), Scope = NamespaceResourceScope>
    + DeserializeOwned
    + Clone
    + std::fmt::Debug
    + Serialize
{
}

trait GatewayResource: CommonResource {}
trait HTTPRouteResource: CommonResource {}
trait ServiceResource: CommonResource {}
trait PodResource: CommonResource {}

trait Description {
    fn new(resource: impl GatewayResource) -> Self;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayRelatedResources<H, S, P> {
    related_resources: GatewayRelatedResourceItems<H, S, P>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GatewayRelatedResourceItems<H: HTTPRouteResource, S: ServiceResource, P: PodResource> {
    #[serde(skip_serializing_if = "Option::is_none")]
    httproutes: Option<RelatedHTTPRoutes<H>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    services: Option<RelatedServices<S>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pods: Option<RelatedPods<P>>,
}

pub type RelatedHTTPRoutes<K> = Vec<RelatedHTTPRoute<K>>;

/// RelatedResourceHTTPRouteのための
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedHTTPRoute<K: HTTPRouteResource> {
    pub name: String,

    pub namespace: String,

    pub gateway_listener: String,

    #[serde(skip)]
    pub resource: K,
}

impl RelatedHTTPRoute {
    fn new(resource: impl HTTPRouteResource, gateway_listener: String) -> Self {
        Self {
            name: resource.name_any(),
            namespace: resource.extract_namespace(),
            gateway_listener,
            resource,
        }
    }
}

pub type RelatedServices<K> = Vec<RelatedService<K>>;

#[derive(Derivative, Debug, Clone, Serialize, Deserialize)]
#[derivative(PartialEq, Eq, Ord)]
pub struct RelatedService<K: ServiceResource> {
    /// Service Name
    pub name: String,

    /// Service Namespace
    pub namespace: String,

    /// HTTPRoute Name
    pub httproute: String,

    #[derivative(PartialEq = "ignore", PartialOrd = "ignore", Ord = "ignore")]
    #[serde(skip)]
    pub resource: K,
}

impl PartialOrd for RelatedService {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

pub type RelatedPods<K> = Vec<RelatedPod<K>>;

#[derive(Derivative, Debug, Clone, Serialize, Deserialize)]
#[derivative(PartialEq, Eq, Ord)]
pub struct RelatedPod<K: PodResource> {
    /// Pod Name
    pub name: String,

    /// Pod Namespace
    pub namespace: String,

    /// Service Name
    pub service: String,

    #[derivative(PartialEq = "ignore", PartialOrd = "ignore", Ord = "ignore")]
    #[serde(skip)]
    pub resource: K,
}

impl PartialOrd for RelatedPod {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
