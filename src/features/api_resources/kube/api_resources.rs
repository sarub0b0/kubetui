use std::{fmt::Display, hash::Hash, ops::Deref, sync::Arc, time};

use anyhow::Result;
use async_trait::async_trait;
use futures::future::try_join_all;
use kube::{
    discovery::{verbs, ApiGroup, Scope},
    Discovery,
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tokio::{sync::RwLock, time::Instant};

use crate::{
    features::api_resources::message::ApiResponse,
    kube::{
        apis::{
            metrics::{NodeMetricsList, PodMetricsList},
            v1_table::{Table, TableColumnDefinition, Value},
        },
        table::insert_ns,
        KubeClient, KubeClientRequest as _,
    },
    workers::kube::{
        PollerBase, SharedTargetApiResources, TargetApiResources, TargetNamespaces, Worker,
        WorkerResult,
    },
};

pub type SharedApiResources = Arc<RwLock<ApiResources>>;

/// kubectl api-resources の結果を保持
/// Network一覧機能のために順番が重要なためVecで保持
#[derive(Debug, Default, Clone)]
pub struct ApiResources {
    inner: Vec<ApiResource>,
}

impl ApiResources {
    pub fn to_vec(&self) -> Vec<ApiResource> {
        self.inner.clone()
    }

    /// SharedApiResourcesを生成
    pub fn shared() -> SharedApiResources {
        Arc::new(RwLock::new(Default::default()))
    }
}

impl Deref for ApiResources {
    type Target = Vec<ApiResource>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: Into<Vec<ApiResource>>> From<T> for ApiResources {
    fn from(value: T) -> Self {
        Self {
            inner: value.into(),
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApiResource {
    Apis {
        name: String,
        group: String,
        version: String,
        preferred_version: bool,
        #[serde(with = "scope_format")]
        scope: Scope,
    },
    Api {
        name: String,
        version: String,
        #[serde(with = "scope_format")]
        scope: Scope,
    },
}

mod scope_format {
    use kube::discovery::Scope;
    use serde::Deserialize as _;

    const NAMESPACED: &str = "Namespaced";
    const CLUSTER: &str = "Cluster";

    pub fn serialize<S>(scope: &Scope, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let value = match scope {
            Scope::Namespaced => NAMESPACED,
            Scope::Cluster => CLUSTER,
        };

        serializer.serialize_str(value)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Scope, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;

        match value.as_str() {
            NAMESPACED => Ok(Scope::Namespaced),
            CLUSTER => Ok(Scope::Cluster),
            _ => Err(serde::de::Error::custom("Invalid scope")),
        }
    }
}

impl ApiResource {
    pub fn is_api(&self) -> bool {
        matches!(self, Self::Api { .. })
    }

    #[allow(dead_code)]
    pub fn is_apis(&self) -> bool {
        matches!(self, Self::Apis { .. })
    }

    pub fn is_preferred_version(&self) -> bool {
        match self {
            Self::Api { .. } => false,
            Self::Apis {
                preferred_version, ..
            } => *preferred_version,
        }
    }

    pub fn is_namespaced(&self) -> bool {
        self.scope() == &Scope::Namespaced
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Api { name, .. } => name,
            Self::Apis { name, .. } => name,
        }
    }

    pub fn group_version_url(&self) -> String {
        match self {
            Self::Apis { group, version, .. } => format!("apis/{}/{}", group, version),
            Self::Api { version, .. } => format!("api/{}", version),
        }
    }

    pub fn group(&self) -> &str {
        match self {
            Self::Apis { group, .. } => group,
            Self::Api { .. } => "",
        }
    }

    pub fn version(&self) -> &str {
        match self {
            Self::Apis { version, .. } => version,
            Self::Api { version, .. } => version,
        }
    }

    pub fn api_url_with_namespace(&self, ns: &str) -> String {
        format!(
            "{}/namespaces/{}/{}",
            self.group_version_url(),
            ns,
            self.name()
        )
    }

    pub fn api_url(&self) -> String {
        format!("{}/{}", self.group_version_url(), self.name())
    }

    fn scope(&self) -> &Scope {
        match self {
            Self::Api { scope, .. } => scope,
            Self::Apis { scope, .. } => scope,
        }
    }

    fn to_table_header(&self) -> String {
        format!("\x1b[90m[ {} ]\x1b[0m\n", self)
    }
}

impl Ord for ApiResource {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.to_string().cmp(&other.to_string())
    }
}

impl PartialOrd for ApiResource {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Display for ApiResource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Api { name, .. } => {
                write!(f, "{}", name)
            }
            Self::Apis {
                name,
                group,
                version,
                preferred_version,
                ..
            } => {
                if *preferred_version {
                    write!(f, "{}.{} (*{})", name, group, version)
                } else {
                    write!(f, "{}.{} ({})", name, group, version)
                }
            }
        }
    }
}

#[derive(Clone)]
pub struct ApiPoller {
    base: PollerBase,
    shared_target_api_resources: SharedTargetApiResources,
    shared_api_resources: SharedApiResources,
}

impl ApiPoller {
    pub fn new(
        base: PollerBase,
        shared_target_api_resources: SharedTargetApiResources,
        shared_api_resources: SharedApiResources,
    ) -> Self {
        Self {
            base,
            shared_target_api_resources,
            shared_api_resources,
        }
    }
}

#[async_trait]
impl Worker for ApiPoller {
    type Output = WorkerResult;

    async fn run(&self) -> Self::Output {
        let Self {
            base:
                PollerBase {
                    is_terminated,
                    tx,
                    shared_target_namespaces,
                    kube_client,
                },
            shared_target_api_resources,
            shared_api_resources,
        } = self;

        match fetch_api_resources(kube_client).await {
            Ok(fetched) => {
                let mut api_resources = shared_api_resources.write().await;
                *api_resources = fetched;
            }
            Err(err) => {
                tx.send(ApiResponse::Poll(Err(err)).into())
                    .expect("Failed to send ApiResponse::Poll");
            }
        }

        let mut interval = tokio::time::interval(time::Duration::from_millis(1000));

        let mut last_tick = Instant::now();
        let tick_rate = time::Duration::from_secs(10);

        let mut is_error = false;

        while !is_terminated.load(std::sync::atomic::Ordering::Relaxed) {
            interval.tick().await;

            if tick_rate < last_tick.elapsed() {
                last_tick = Instant::now();

                match fetch_api_resources(&kube_client).await {
                    Ok(fetched) => {
                        let mut api_resources = shared_api_resources.write().await;
                        *api_resources = fetched;

                        // Clear error
                        if is_error {
                            is_error = false;
                            tx.send(ApiResponse::Poll(Ok(Default::default())).into())
                                .expect("Failed to send ApiResponse::Poll");
                        }
                    }
                    Err(err) => {
                        tx.send(ApiResponse::Poll(Err(err)).into())
                            .expect("Failed to send ApiResponse::Poll");
                        is_error = true;
                        continue;
                    }
                }
            }

            let target_namespaces = shared_target_namespaces.read().await;
            let target_api_resources = shared_target_api_resources.read().await;

            if target_api_resources.is_empty() {
                continue;
            }

            let result = FetchTargetApiResources::new(
                &kube_client,
                &target_api_resources,
                &target_namespaces,
            )
            .fetch_table()
            .await;

            tx.send(ApiResponse::Poll(result).into())
                .expect("Failed to send ApiResponse::Poll");
        }

        WorkerResult::Terminated
    }
}

pub async fn fetch_api_resources(client: &KubeClient) -> Result<ApiResources> {
    let discovery = Discovery::new(client.to_client()).run().await?;

    let ret = discovery
        .groups()
        .flat_map(|group| {
            let preferred_version = group.preferred_version_or_latest();

            group
                .versions()
                .flat_map(|version| {
                    let is_preferred_version = preferred_version == version;

                    group
                        .versioned_resources(version)
                        .iter()
                        .filter_map(|(ar, caps)| {
                            if !caps.supports_operation(verbs::LIST) {
                                return None;
                            }

                            if group.name() == ApiGroup::CORE_GROUP {
                                Some(ApiResource::Api {
                                    name: ar.plural.to_string(),
                                    version: ar.version.to_string(),
                                    scope: caps.scope.clone(),
                                })
                            } else {
                                Some(ApiResource::Apis {
                                    name: ar.plural.to_string(),
                                    group: ar.group.to_string(),
                                    version: ar.version.to_string(),
                                    preferred_version: is_preferred_version,
                                    scope: caps.scope.clone(),
                                })
                            }
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    Ok(ret.into())
}

fn merge_tables(fetch_data: Vec<FetchData>, insert_ns: bool) -> Table {
    if fetch_data.is_empty() {
        return Table::default();
    }

    let mut base_table = fetch_data[0].table.clone();
    let base_ns = &fetch_data[0].namespace;

    if insert_ns {
        let column_definitions = TableColumnDefinition {
            name: "Namespace".to_string(),
            ..Default::default()
        };

        base_table.column_definitions.insert(0, column_definitions);

        base_table.rows.iter_mut().for_each(|row| {
            row.cells
                .insert(0, Value(JsonValue::String(base_ns.to_string())))
        });
    }

    fetch_data.into_iter().skip(1).for_each(|mut d| {
        if insert_ns {
            let ns = d.namespace.to_string();
            d.table.rows.iter_mut().for_each(|row| {
                row.cells
                    .insert(0, Value(JsonValue::String(ns.to_string())));
            });
        }

        base_table.rows.append(&mut d.table.rows);
    });

    base_table
}

async fn try_fetch_table(client: &KubeClient, path: &str) -> Result<Table> {
    let table = client.table_request::<Table>(path).await;

    if let Ok(t) = table {
        return Ok(t);
    }

    let table = client.table_request::<NodeMetricsList>(path).await;

    if let Ok(t) = table {
        return Ok(t.into());
    }

    let table = client.table_request::<PodMetricsList>(path).await?;

    Ok(table.into())
}

struct FetchData {
    namespace: String,
    table: Table,
}

async fn fetch_table_per_namespace(
    client: &KubeClient,
    path: String,
    ns: &str,
) -> Result<FetchData> {
    let table = try_fetch_table(client, &path).await?;

    Ok(FetchData {
        namespace: ns.to_string(),
        table,
    })
}

#[inline]
async fn get_table_namespaced_resource(
    client: &KubeClient,
    api_resource: &ApiResource,
    namespaces: &[String],
) -> Result<Table> {
    let jobs =
        try_join_all(namespaces.iter().map(|ns| {
            fetch_table_per_namespace(client, api_resource.api_url_with_namespace(ns), ns)
        }))
        .await?;

    let result: Vec<FetchData> = jobs.into_iter().collect();

    Ok(merge_tables(result, insert_ns(namespaces)))
}

#[inline]
async fn get_table_cluster_resource(client: &KubeClient, path: &str) -> Result<Table> {
    try_fetch_table(client, path).await
}

struct FetchTargetApiResources<'a> {
    client: &'a KubeClient,
    target_api_resources: &'a TargetApiResources,
    target_namespace: &'a TargetNamespaces,
}

impl<'a> FetchTargetApiResources<'a> {
    fn new(
        client: &'a KubeClient,
        target_api_resources: &'a TargetApiResources,
        target_namespace: &'a TargetNamespaces,
    ) -> Self {
        Self {
            client,
            target_api_resources,
            target_namespace,
        }
    }

    async fn fetch_table(&self) -> Result<Vec<String>> {
        let mut ret = Vec::new();
        for api_resource in self.target_api_resources {
            let table = if api_resource.is_namespaced() {
                get_table_namespaced_resource(self.client, api_resource, self.target_namespace)
                    .await
            } else {
                get_table_cluster_resource(self.client, &api_resource.api_url()).await
            }?;

            let data = if table.rows.is_empty() {
                api_resource.to_table_header()
            } else {
                api_resource.to_table_header() + &table.to_print()
            };

            ret.extend(data.lines().map(ToString::to_string).collect::<Vec<_>>());
            ret.push("".to_string());
        }

        Ok(ret)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod api_resource {
        use super::*;
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        #[rstest]
        #[case(ApiResource::Api { name: "pods".into(), version: "v1".into(), scope: Scope::Namespaced }, "pods")]
        #[case(ApiResource::Apis { name: "horizontalpodautoscalers".into(), group: "autoscaling".into(), version: "v2".into(), preferred_version: true, scope: Scope::Namespaced }, "horizontalpodautoscalers.autoscaling (*v2)")]
        #[case(ApiResource::Apis { name: "horizontalpodautoscalers".into(), group: "autoscaling".into(), version: "v1".into(), preferred_version: false, scope: Scope::Namespaced }, "horizontalpodautoscalers.autoscaling (v1)")]
        #[test]
        fn to_string(#[case] key: ApiResource, #[case] expected: &str) {
            assert_eq!(key.to_string(), expected)
        }
    }
}
