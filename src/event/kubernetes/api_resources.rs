use super::{
    client::KubeClientRequest,
    worker::{PollWorker, Worker},
    KubeClient, TargetApiResources,
    {v1_table::*, SharedTargetApiResources},
    {Event, Kube},
};
use super::{metric_type::*, WorkerResult};
use crate::error::Result;

use async_trait::async_trait;
use futures::future::try_join_all;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{
    APIGroup, APIGroupList, APIResource, APIVersions,
};
use kube::core::TypeMeta;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Display,
    hash::Hash,
    sync::Arc,
    time,
};
use tokio::{sync::RwLock, time::Instant};

#[derive(Debug)]
pub enum ApiRequest {
    Get,
    Set(Vec<ApiDBKey>),
}

#[derive(Debug)]
pub enum ApiResponse {
    Get(Result<Vec<ApiDBKey>>),
    Set(Vec<String>),
    Poll(Result<Vec<String>>),
}

#[derive(Debug)]
pub enum ApiMessage {
    Request(ApiRequest),
    Response(ApiResponse),
}

impl From<ApiRequest> for Event {
    fn from(f: ApiRequest) -> Self {
        Self::Kube(Kube::API(ApiMessage::Request(f)))
    }
}

impl From<ApiResponse> for Event {
    fn from(f: ApiResponse) -> Self {
        Self::Kube(Kube::API(ApiMessage::Response(f)))
    }
}

impl From<ApiMessage> for Kube {
    fn from(f: ApiMessage) -> Self {
        Self::API(f)
    }
}

impl From<ApiMessage> for Event {
    fn from(f: ApiMessage) -> Self {
        Self::Kube(f.into())
    }
}

pub type ApiResources = BTreeMap<ApiDBKey, ApiDBValue>;
pub type SharedApiResources = Arc<RwLock<ApiResources>>;

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApiDBKey {
    Apis {
        name: String,
        group: String,
        version: String,
        preferred_version: bool,
    },
    Api {
        name: String,
        version: String,
    },
}

impl ApiDBKey {
    pub fn is_api(&self) -> bool {
        matches!(self, ApiDBKey::Api { .. })
    }

    pub fn is_apis(&self) -> bool {
        matches!(self, ApiDBKey::Apis { .. })
    }

    pub fn is_preferred_version(&self) -> bool {
        match self {
            ApiDBKey::Api { .. } => false,
            ApiDBKey::Apis {
                preferred_version, ..
            } => *preferred_version,
        }
    }
}

impl Ord for ApiDBKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.to_string().cmp(&other.to_string())
    }
}

impl PartialOrd for ApiDBKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.to_string().partial_cmp(&other.to_string())
    }
}

impl Display for ApiDBKey {
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

impl From<ApiDBValue> for ApiDBKey {
    fn from(value: ApiDBValue) -> Self {
        if value.api_group.is_empty() {
            Self::Api {
                name: value.api_resource.name,
                version: value.api_group_version,
            }
        } else {
            Self::Apis {
                name: value.api_resource.name,
                group: value.api_group,
                version: value.api_group_version,
                preferred_version: value.preferred_version,
            }
        }
    }
}

#[derive(Clone)]
pub struct ApiPollWorker {
    inner: PollWorker,
    shared_target_api_resources: SharedTargetApiResources,
    shared_api_resources: SharedApiResources,
}

impl ApiPollWorker {
    pub fn new(
        inner: PollWorker,
        shared_target_api_resources: SharedTargetApiResources,
        shared_api_resources: SharedApiResources,
    ) -> Self {
        Self {
            inner,
            shared_target_api_resources,
            shared_api_resources,
        }
    }
}

#[async_trait]
impl Worker for ApiPollWorker {
    type Output = Result<WorkerResult>;

    async fn run(&self) -> Self::Output {
        let Self {
            inner:
                PollWorker {
                    is_terminated,
                    tx,
                    shared_target_namespaces,
                    kube_client,
                },
            shared_target_api_resources,
            shared_api_resources,
        } = self;

        {
            let mut api_resources = shared_api_resources.write().await;

            *api_resources = fetch_api_resources(kube_client).await?;
        }

        let mut interval = tokio::time::interval(time::Duration::from_millis(1000));

        let mut last_tick = Instant::now();
        let tick_rate = time::Duration::from_secs(10);

        while !is_terminated.load(std::sync::atomic::Ordering::Relaxed) {
            interval.tick().await;
            let target_namespaces = shared_target_namespaces.read().await;
            let target_api_resources = shared_target_api_resources.read().await;

            if target_api_resources.is_empty() {
                continue;
            }

            if tick_rate < last_tick.elapsed() {
                last_tick = Instant::now();

                match fetch_api_resources(kube_client).await {
                    Ok(fetched) => {
                        let mut api_resources = shared_api_resources.write().await;
                        *api_resources = fetched;
                    }
                    Err(err) => {
                        tx.send(ApiResponse::Poll(Err(err)).into()).unwrap();
                        continue;
                    }
                }
            }

            let api_resources = shared_api_resources.read().await;
            let result = get_api_resources(
                kube_client,
                &target_namespaces,
                &target_api_resources,
                &api_resources,
            )
            .await;

            tx.send(ApiResponse::Poll(result).into()).unwrap();
        }

        Ok(WorkerResult::Terminated)
    }
}

#[derive(Debug, Clone)]
pub struct ApiDBValue {
    pub api_version: String,
    pub api_group: String,
    pub api_group_version: String,
    pub api_resource: APIResource,
    pub preferred_version: bool,
}

impl ApiDBValue {
    pub fn api_url(&self) -> String {
        if self.api_group.is_empty() {
            format!("api/{}", self.api_group_version)
        } else {
            format!("apis/{}/{}", self.api_group, self.api_group_version)
        }
    }

    pub fn resource_full_name(&self) -> String {
        if self.api_group.is_empty() {
            self.api_resource.name.to_string()
        } else {
            format!("{}.{}", self.api_resource.name, self.api_group)
        }
    }
}

impl Display for ApiDBValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.api_group.is_empty() {
            write!(f, "{}", self.api_resource.name)
        } else if self.preferred_version {
            write!(
                f,
                "{}.{} (*{})",
                self.api_resource.name, self.api_group, self.api_group_version
            )
        } else {
            write!(
                f,
                "{}.{} ({})",
                self.api_resource.name, self.api_group, self.api_group_version
            )
        }
    }
}

#[derive(Debug)]
struct GroupVersion {
    group: String,
    version: String,
    preferred_version: bool,
}

impl GroupVersion {
    fn api_url(&self) -> String {
        if self.group.is_empty() {
            format!("api/{}", self.version)
        } else {
            format!("apis/{}/{}", self.group, self.version)
        }
    }
}

async fn get_all_api_info(client: &KubeClient) -> Result<Vec<ApiDBValue>> {
    let mut group_versions = Vec::new();

    let api_versions: APIVersions = client.request("api").await?;

    api_versions.versions.iter().for_each(|v| {
        group_versions.push(GroupVersion {
            group: String::default(),
            version: v.to_string(),
            preferred_version: false,
        })
    });

    let api_groups: APIGroupList = client.request("apis").await?;

    api_groups.groups.iter().for_each(|group| {
        for gv in &group.versions {
            if let Some(ref pv) = group.preferred_version {
                group_versions.push(GroupVersion {
                    group: group.name.to_string(),
                    version: gv.version.to_string(),
                    preferred_version: pv.version == gv.version,
                });
            } else {
                group_versions.push(GroupVersion {
                    group: group.name.to_string(),
                    version: gv.version.to_string(),
                    preferred_version: false,
                });
            }
        }
    });

    // APIResourceListを取得
    //      /api/v1
    //      /api/v2
    //      /api/v*
    //      /apis/group/version

    let job = try_join_all(
        group_versions
            .iter()
            .map(|gv| api_resource_list_to_api_info_list(client, gv)),
    )
    .await?;

    Ok(job.into_iter().flatten().collect())
}

fn can_get_request(api: &APIResource) -> bool {
    api.verbs.contains(&"list".to_string())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct APIResourceList {
    types: Option<TypeMeta>,
    group_version: String,
    resources: Vec<APIResource>,
}

async fn api_resource_list_to_api_info_list(
    client: &KubeClient,
    gv: &GroupVersion,
) -> Result<Vec<ApiDBValue>> {
    let result = client.request::<APIResourceList>(&gv.api_url()).await?;

    Ok(result
        .resources
        .iter()
        .filter(|resource| can_get_request(resource))
        .map(|resource| ApiDBValue {
            api_group: gv.group.to_string(),
            api_version: resource.version.clone().unwrap_or_default(),
            api_group_version: gv.version.to_string(),
            api_resource: resource.clone(),
            preferred_version: gv.preferred_version,
        })
        .collect())
}

pub async fn fetch_api_resources(client: &KubeClient) -> Result<ApiResources> {
    let api_info_list = get_all_api_info(client).await?;
    Ok(convert_api_resources(&api_info_list))
}

pub fn api_resources_to_vec(api_resources: &ApiResources) -> Vec<ApiDBKey> {
    api_resources.iter().map(|(k, _)| k.clone()).collect()
}

fn convert_api_resources(api_info_list: &[ApiDBValue]) -> ApiResources {
    let mut api_resources: ApiResources = ApiResources::new();

    for info in api_info_list {
        let key = ApiDBKey::from(info.clone());

        api_resources.entry(key).or_insert_with(|| info.clone());
    }

    api_resources
}

fn merge_tables(fetch_data: Vec<FetchData>, insert_ns: bool) -> Table {
    if fetch_data.is_empty() {
        return Table::default();
    }

    let fetch_data = fetch_data;

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

#[inline]
fn header_by_api_info(info: &ApiDBValue) -> String {
    format!("\x1b[90m[ {} ]\x1b[0m\n", info)
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
    path: String,
    kind: &str,
    namespaces: &[String],
) -> Result<Table> {
    let jobs = try_join_all(namespaces.iter().map(|ns| {
        let path = format!("{}/namespaces/{}/{}", path, ns, kind);
        fetch_table_per_namespace(client, path, ns)
    }))
    .await?;

    let result: Vec<FetchData> = jobs.into_iter().collect();

    Ok(merge_tables(result, insert_ns(namespaces)))
}

#[inline]
async fn get_table_cluster_resource(client: &KubeClient, path: &str) -> Result<Table> {
    try_fetch_table(client, path).await
}

async fn get_api_resources(
    client: &KubeClient,
    namespaces: &[String],
    target_api_resources: &TargetApiResources,
    api_resources: &ApiResources,
) -> Result<Vec<String>> {
    let mut ret = Vec::new();

    for api in target_api_resources {
        if let Some(info) = api_resources.get(api) {
            let table = if info.api_resource.namespaced {
                get_table_namespaced_resource(
                    client,
                    info.api_url(),
                    &info.api_resource.name,
                    namespaces,
                )
                .await
            } else {
                get_table_cluster_resource(
                    client,
                    &format!("{}/{}", info.api_url(), info.api_resource.name),
                )
                .await
            }?;

            let data = if table.rows.is_empty() {
                header_by_api_info(info)
            } else {
                header_by_api_info(info) + &table.to_print()
            };

            ret.extend(data.lines().map(ToString::to_string).collect::<Vec<_>>());
            ret.push("".to_string());
        }
    }

    Ok(ret)
}

#[cfg(test)]
mod tests {
    use super::*;

    mod database {
        use super::*;
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        #[rstest]
        #[case(ApiDBKey::Api { name: "pods".into(), version: "v1".into() }, "pods")]
        #[case(ApiDBKey::Apis { name: "horizontalpodautoscalers".into(), group: "autoscaling".into(), version: "v2".into(), preferred_version: true }, "horizontalpodautoscalers.autoscaling (*v2)")]
        #[case(ApiDBKey::Apis { name: "horizontalpodautoscalers".into(), group: "autoscaling".into(), version: "v1".into(), preferred_version: false }, "horizontalpodautoscalers.autoscaling (v1)")]
        #[test]
        fn feature(#[case] key: ApiDBKey, #[case] expected: &str) {
            println!("{}", key);
            assert_eq!(key.to_string(), expected)
        }
    }
}
