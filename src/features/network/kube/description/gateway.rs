use anyhow::{Context as _, Result};
use k8s_openapi::{apimachinery::pkg::apis::meta::v1::Condition, Resource as _};
use kube::{Api, Client, ResourceExt};
use serde::{Deserialize, Serialize};

use crate::kube::{
    apis::networking::gateway::v1::{
        AllowedRoutes, FromNamespaces, Gateway, GatewaySpec, GatewayStatusAddress, HTTPRoute,
        ListenerStatus, ParentReference, RouteGroupKind, RouteNamespaces,
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

// #[async_trait::async_trait]
// impl<'a, C> Fetch<'a, C> for GatewayDescriptionWorker<'a, C>
// where
//     C: KubeClientRequest,
// {
//     fn new(client: &'a C, namespace: String, name: String) -> Self {
//         Self {
//             client,
//             namespace,
//             name,
//         }
//     }
//
//     async fn fetch(&self) -> Result<FetchedData> {
//         let client = self.client.client().clone();
//
//         let (gateway, description) =
//             fetch_gateway(client.clone(), &self.namespace, &self.name).await?;
//
//         let related_resources =
//             fetch_releated_resources(client, &gateway, &self.name, &self.namespace).await?;
//
//         let result = [description, vec!["".to_string()], related_resources]
//             .into_iter()
//             .flatten()
//             .collect();
//
//         Ok(result)
//     }
// }
//
// async fn fetch_gateway(
//     client: Client,
//     namespace: &str,
//     name: &str,
// ) -> Result<(Gateway, Vec<String>)> {
//     let api = Api::<Gateway>::namespaced(client, namespace);
//
//     let gateway = api.get(name).await.context(format!(
//         "Failed to fetch Gateway: namespace={}, name={}",
//         namespace, name
//     ))?;
//
//     let description = GatewayDescription::new(gateway.clone());
//
//     let yaml = serde_yaml::to_string(&description)?
//         .lines()
//         .map(ToString::to_string)
//         .collect::<Vec<String>>();
//
//     Ok((gateway, yaml))
// }
//
async fn discover_releated_resources(
    client: Client,
    gateway_name: &str,
    namespace: &str,
    gateway: &Gateway,
) -> Result<Vec<String>> {
    // let httproutes = discover_http_routes(client.clone(), gateway_name, namespace, gateway).await?;
    //
    // let related_resources = GatewayRelatedResources {
    //     related_resources: GatewayRelatedResourceItems {
    //         httproutes,
    //         services: None,
    //         pods: None,
    //     },
    // };
    //
    // let yaml = serde_yaml::to_string(&related_resources)?
    //     .lines()
    //     .map(ToString::to_string)
    //     .collect::<Vec<String>>();

    let yaml = vec![];

    Ok(yaml)
}

async fn discover_http_routes(
    client: Client,
    gateway_name: &str,
    namespace: &str,
    gateway: &Gateway,
) -> Result<Option<RelatedResources>> {
    let mut result = Vec::new();

    for listener in &gateway.spec.listeners {
        let Some(AllowedRoutes { kinds, namespaces }) = listener.allowed_routes.as_ref() else {
            continue;
        };

        if let Some(RouteNamespaces { from, selector }) = namespaces {
            if let Some(FromNamespaces(from)) = from {
                match from.as_str() {
                    "All" => {
                        let http_routes = http_route_all(client.clone(), gateway_name).await?;

                        result.extend(http_routes);
                    }
                    "Selector" => {}
                    "Same" => {}
                    _ => {
                        unreachable!()
                    }
                }
            }

            if let Some(selector) = selector {}
        }
    }

    if result.is_empty() {
        Ok(None)
    } else {
        Ok(Some(result))
    }
}

async fn http_route_all(client: Client, parent_refs: &str) -> Result<RelatedResources> {
    let api = Api::<HTTPRoute>::all(client);

    let http_routes = api.list(&Default::default()).await?;

    todo!()

    // let result: RelatedResources = http_routes
    //     .iter()
    //     .filter(|http_route| check_http_route(http_route, parent_refs))
    //     .map(|http_route| RelatedResource {
    //         name: http_route.name_any(),
    //         namespace: http_route.namespace().unwrap_or("unknown".to_string()),
    //     })
    //     .collect();
    //
    // Ok(result)
}

// HTTPRouteのParentReferencesにGatewayリソースが含まれているかをチェックする
fn check_http_route(http_route: &HTTPRoute, gateway: &Gateway) -> bool {
    http_route.spec.parent_refs.as_ref().is_some_and(|refs| {
        refs.iter().any(
            |ParentReference {
                 group,
                 name,
                 namespace,
                 kind,
                 section_name,
             }| {
                compare_parent_ref_group(group.as_ref().map(|g| g.as_str()), Gateway::GROUP)
                    && compare_parent_ref_kind(kind.as_ref().map(|k| k.as_str()), Gateway::KIND)
                    && compare_parent_ref_name(name, &gateway.name_any())
                    && compare_parent_ref_namespace(
                        namespace.as_ref().map(|ns| ns.as_str()),
                        http_route.namespace().as_ref().map(|ns| ns.as_str()),
                        gateway.namespace().as_ref().map(|ns| ns.as_str()),
                    )
            },
        )
    })
}

fn compare_parent_ref_group(group: Option<&str>, target_group: &str) -> bool {
    group.unwrap_or(Gateway::GROUP) == target_group
}

fn compare_parent_ref_kind(kind: Option<&str>, target_kind: &str) -> bool {
    kind.unwrap_or(Gateway::KIND) == target_kind
}

fn compare_parent_ref_name(name: &str, target_name: &str) -> bool {
    name == target_name
}

/// ParentReferenceが指定しているnamespaceとGatewayのnamespaceが一致しているかをチェックする。
/// ParentRefereceのnamespaceが指定されていない場合は、HTTPRouteのnamespaceとGatewayのnamespaceが一致しているかをチェックする。
fn compare_parent_ref_namespace(
    parent_ref_namespace: Option<&str>,
    http_route_namespace: Option<&str>,
    gateway_namespace: Option<&str>,
) -> bool {
    if let Some(parent_ref_namespace) = parent_ref_namespace {
        parent_ref_namespace == gateway_namespace.expect("Gateway namespace is not found")
    } else {
        http_route_namespace == gateway_namespace
    }
}

fn compare_parent_ref_section_name(section_name: Option<&str>, gateway: &Gateway) -> bool {
    section_name.unwrap_or(gateway.name_any().as_str()) == gateway.name_any()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GatewayRelatedResources {
    related_resources: GatewayRelatedResourceItems,
}

type RelatedResources = Vec<RelatedResource>;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GatewayRelatedResourceItems {
    #[serde(skip_serializing_if = "Option::is_none")]
    httproutes: Option<RelatedResources>,
    #[serde(skip_serializing_if = "Option::is_none")]
    services: Option<RelatedResources>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pods: Option<RelatedResources>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RelatedResource {
    name: String,
    namespace: String,
}
