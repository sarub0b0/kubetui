use std::collections::BTreeSet;

use anyhow::{Context, Result};
use futures::future::{join_all, try_join_all};
use k8s_openapi::{
    api::core::v1::Namespace, apimachinery::pkg::apis::meta::v1::LabelSelector, Resource as _,
};
use kube::{api::ListParams, Api, Client, ResourceExt as _};
use serde::{Deserialize, Serialize};

use crate::{
    features::network::kube::description::utils::{label_selector_to_query, ExtractNamespace as _},
    kube::apis::networking::gateway::v1::{
        Gateway, GatewayListenersAllowedRoutes, GatewayListenersAllowedRoutesNamespaces,
        GatewayListenersAllowedRoutesNamespacesFrom, HTTPRoute, HTTPRouteParentRefs,
    },
    logger,
};

pub type RelatedHTTPRoutes = Vec<RelatedHTTPRoute>;

/// RelatedResourceHTTPRouteのための
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedHTTPRoute {
    pub name: String,

    pub namespace: String,

    #[serde(skip)]
    pub resource: HTTPRoute,
}

impl Eq for RelatedHTTPRoute {}

impl Ord for RelatedHTTPRoute {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // nameとnamespaceが同じであれば同じリソースとして扱う
        self.name
            .cmp(&other.name)
            .then_with(|| self.namespace.cmp(&other.namespace))
    }
}

impl PartialOrd for RelatedHTTPRoute {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for RelatedHTTPRoute {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.namespace == other.namespace
    }
}

impl RelatedHTTPRoute {
    fn new(resource: HTTPRoute) -> Self {
        Self {
            name: resource.name_any(),
            namespace: resource.extract_namespace(),
            resource,
        }
    }
}

pub async fn discover_httproutes(
    client: Client,
    gateway_name: &str,
    gateway_namespace: &str,
    gateway: &Gateway,
) -> Result<Option<RelatedHTTPRoutes>> {
    let listeners: BTreeSet<_> = gateway
        .spec
        .listeners
        .iter()
        .cloned()
        .filter_map(|listener| listener.allowed_routes)
        .collect();

    let jobs = join_all(listeners.iter().map(|listener| {
        discover_httproute_per_listener(client.clone(), gateway_name, gateway_namespace, listener)
    }))
    .await;

    let result: BTreeSet<_> = jobs
        .into_iter()
        .inspect(|res| {
            if let Err(e) = res {
                logger!(error, "Failed to discover httproute: {:?}", e);
            }
        })
        .flatten()
        .flatten()
        .flatten()
        .collect();

    if result.is_empty() {
        Ok(None)
    } else {
        Ok(Some(result.into_iter().collect()))
    }
}

async fn discover_httproute_per_listener(
    client: Client,
    gateway_name: &str,
    gateway_namespace: &str,
    allowed_routes: &GatewayListenersAllowedRoutes,
) -> Result<Option<RelatedHTTPRoutes>> {
    // NOTE: kinds, protocolでのフィルタリングは実装に依存しそうなため一旦実装しない
    let GatewayListenersAllowedRoutes {
        kinds: _,
        namespaces,
    } = allowed_routes;

    // namespacesがNoneのときは、Same（Gatewayとおなじnamespace）として扱う
    if let Some(GatewayListenersAllowedRoutesNamespaces { from, selector }) = namespaces {
        let Some(from) = from else {
            return Ok(None);
        };

        let httproutes = match from {
            GatewayListenersAllowedRoutesNamespacesFrom::All => {
                discover_httproute_for_all(client.clone(), gateway_name, gateway_namespace)
                    .await
                    .with_context(|| {
                        format!(
                            "Failed to discover httproute for all. Gateway: {}/{}",
                            gateway_namespace, gateway_name
                        )
                    })?
            }
            GatewayListenersAllowedRoutesNamespacesFrom::Selector => {
                discover_httproute_for_selector(
                    client.clone(),
                    gateway_name,
                    gateway_namespace,
                    selector.as_ref().map(|s| LabelSelector::from(s.clone())),
                )
                .await
                .with_context(|| {
                    format!(
                        "Failed to discover httproute for selector. Gateway: {}/{}",
                        gateway_namespace, gateway_name
                    )
                })?
            }
            GatewayListenersAllowedRoutesNamespacesFrom::Same => {
                discover_httproute_for_same(client.clone(), gateway_name, gateway_namespace)
                    .await
                    .with_context(|| {
                        format!(
                            "Failed to discover httproute for same. Gateway: {}/{}",
                            gateway_namespace, gateway_name
                        )
                    })?
            }
        };

        let result: Vec<_> = httproutes.into_iter().map(RelatedHTTPRoute::new).collect();

        if result.is_empty() {
            Ok(None)
        } else {
            Ok(Some(result))
        }
    } else {
        // たぶんこのブロックが実行されることはない
        let httproutes =
            discover_httproute_for_same(client.clone(), gateway_name, gateway_namespace)
                .await
                .with_context(|| {
                    format!(
                        "Failed to discover httproute for same. Gateway: {}/{}",
                        gateway_namespace, gateway_name
                    )
                })?;

        let result: Vec<_> = httproutes.into_iter().map(RelatedHTTPRoute::new).collect();

        if result.is_empty() {
            Ok(None)
        } else {
            Ok(Some(result))
        }
    }
}

/// Gatewayを参照しているHTTPRouteリソースを取得する（全namespace）
async fn discover_httproute_for_all(
    client: Client,
    gateway_name: &str,
    gateway_namespace: &str,
) -> Result<Vec<HTTPRoute>> {
    let api = Api::<HTTPRoute>::all(client);

    let httproutes = api.list(&ListParams::default()).await?;

    let result: Vec<_> = httproutes
        .into_iter()
        .filter(|httproute| check_httproute(httproute, gateway_name, gateway_namespace))
        .collect();

    Ok(result)
}

async fn discover_httproute_for_same(
    client: Client,
    gateway_name: &str,
    gateway_namespace: &str,
) -> Result<Vec<HTTPRoute>> {
    let api = Api::<HTTPRoute>::namespaced(client, gateway_namespace);

    let httproutes = api.list(&ListParams::default()).await?;

    let result = httproutes
        .into_iter()
        .filter(|httproute| check_httproute(httproute, gateway_name, gateway_namespace))
        .collect();

    Ok(result)
}

async fn discover_httproute_for_selector(
    client: Client,
    gateway_name: &str,
    gateway_namespace: &str,
    selector: Option<LabelSelector>,
) -> Result<Vec<HTTPRoute>> {
    let api = Api::<Namespace>::all(client.clone());

    let lp = ListParams::default().labels(&label_selector_to_query(selector));

    let namespaces = api.list(&lp).await?;

    let httproutes = try_join_all(namespaces.iter().map(|ns| async {
        let api = Api::<HTTPRoute>::namespaced(client.clone(), &ns.name_any());

        let httproutes = api.list(&ListParams::default()).await?;

        let result: Vec<_> = httproutes
            .into_iter()
            .filter(|httproute| check_httproute(httproute, gateway_name, gateway_namespace))
            .collect();

        anyhow::Ok(result)
    }))
    .await?;

    Ok(httproutes.into_iter().flatten().collect())
}

// HTTPRouteのParentReferencesにGatewayリソースが含まれているかをチェックする
fn check_httproute(httproute: &HTTPRoute, gateway_name: &str, gateway_namespace: &str) -> bool {
    httproute.spec.parent_refs.as_ref().is_some_and(|refs| {
        refs.iter().any(
            |HTTPRouteParentRefs {
                 group,
                 name,
                 namespace,
                 kind,
                 ..
             }| {
                compare_parent_ref_group(group.as_ref().map(|g| g.as_str()), Gateway::GROUP)
                    && compare_parent_ref_kind(kind.as_ref().map(|k| k.as_str()), Gateway::KIND)
                    && compare_parent_ref_name(name.as_str(), gateway_name)
                    && compare_parent_ref_namespace(
                        namespace.as_ref().map(|ns| ns.as_str()),
                        httproute.extract_namespace().as_str(),
                        gateway_namespace,
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
    httproute_namespace: &str,
    gateway_namespace: &str,
) -> bool {
    if let Some(parent_ref_namespace) = parent_ref_namespace {
        parent_ref_namespace == gateway_namespace
    } else {
        httproute_namespace == gateway_namespace
    }
}
