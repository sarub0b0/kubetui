use k8s_openapi::apimachinery::pkg::apis::meta::v1::Condition;
use kube::ResourceExt;
use serde::{Deserialize, Serialize};

use crate::kube::apis::networking::gateway::v1beta1::{
    HTTPRoute, HTTPRouteSpec, HTTPRouteStatus, HTTPRouteStatusParents,
    HTTPRouteStatusParentsParentRef,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Description {
    httproute: DescriptionHTTPRoute,
}

impl Description {
    pub fn new(http_route: HTTPRoute) -> Self {
        Self {
            httproute: DescriptionHTTPRoute::new(http_route),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MetadataName {
    name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DescriptionHTTPRoute {
    metadata: MetadataName,

    spec: HTTPRouteSpec,

    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<HTTPRouteStatusWrapper>,
}

impl DescriptionHTTPRoute {
    fn new(http_route: HTTPRoute) -> Self {
        let name = http_route.name_any();

        let HTTPRoute {
            metadata: _,
            spec,
            status,
        } = http_route;

        let status_wrapper = status.map(HTTPRouteStatusWrapper::new);

        Self {
            metadata: MetadataName { name },
            spec,
            status: status_wrapper,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HTTPRouteStatusWrapper {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    parents: Vec<RouteParentStatusWrapper>,
}

impl HTTPRouteStatusWrapper {
    fn new(status: HTTPRouteStatus) -> Self {
        let parents = status
            .parents
            .into_iter()
            .map(RouteParentStatusWrapper::new)
            .collect();

        Self { parents }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RouteParentStatusWrapper {
    #[serde(flatten)]
    #[serde(with = "HTTPRouteStatusParentsDef")]
    status: HTTPRouteStatusParents,
}

impl RouteParentStatusWrapper {
    fn new(status: HTTPRouteStatusParents) -> Self {
        Self { status }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "HTTPRouteStatusParents")]
#[serde(rename_all = "camelCase")]
struct HTTPRouteStatusParentsDef {
    #[serde(skip)]
    conditions: Option<Vec<Condition>>,

    controller_name: String,

    parent_ref: HTTPRouteStatusParentsParentRef,
}
