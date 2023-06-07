use super::{
    client::KubeClientRequest,
    worker::{PollWorker, Worker},
    KubeClient, TargetApiResources, TargetNamespaces,
    {v1_table::*, SharedTargetApiResources},
    {Event, Kube},
};
use super::{metric_type::*, WorkerResult};
use crate::error::Result;

use async_trait::async_trait;
use futures::future::try_join_all;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{APIGroupList, APIResource, APIVersions};
use kube::core::TypeMeta;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::{collections::BTreeSet, fmt::Display, hash::Hash, ops::Deref, sync::Arc, time};
use tokio::{sync::RwLock, time::Instant};

#[derive(Debug)]
pub enum ApiRequest {
    Get,
    Set(Vec<ApiResource>),
}

#[derive(Debug)]
pub enum ApiResponse {
    Get(Result<Vec<ApiResource>>),
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

pub type SharedApiResources = Arc<RwLock<ApiResources>>;

#[derive(Debug, Default, Clone)]
pub struct ApiResources {
    inner: BTreeSet<ApiResource>,
}

impl ApiResources {
    pub fn to_vec(&self) -> Vec<ApiResource> {
        self.inner.clone().into_iter().collect()
    }
}

impl Deref for ApiResources {
    type Target = BTreeSet<ApiResource>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl From<BTreeSet<ApiResource>> for ApiResources {
    fn from(value: BTreeSet<ApiResource>) -> Self {
        Self { inner: value }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApiResource {
    Apis {
        name: String,
        group: String,
        version: String,
        preferred_version: bool,
        namespaced: bool,
    },
    Api {
        name: String,
        version: String,
        namespaced: bool,
    },
}

impl ApiResource {
    pub fn is_api(&self) -> bool {
        matches!(self, Self::Api { .. })
    }

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
        match self {
            Self::Api { namespaced, .. } => *namespaced,
            Self::Apis { namespaced, .. } => *namespaced,
        }
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
        self.to_string().partial_cmp(&other.to_string())
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
    type Output = WorkerResult;

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

                match fetch_api_resources(kube_client).await {
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
                kube_client,
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

trait ContainListVerb {
    fn contain_list_verb(&self) -> bool;
}

impl ContainListVerb for APIResource {
    fn contain_list_verb(&self) -> bool {
        self.verbs.contains(&"list".into())
    }
}

struct FetchApiResources<'a> {
    client: &'a KubeClient,
}

impl<'a> FetchApiResources<'a> {
    fn new(client: &'a KubeClient) -> Self {
        Self { client }
    }

    async fn fetch_all(&self) -> Result<ApiResources> {
        let mut group_versions = self.fetch_api_versions().await?;
        let api_groups = self.fetch_api_groups().await?;

        group_versions.extend(api_groups);

        // APIResourceListを取得
        //      /api/v1
        //      /api/v2
        //      /api/v*
        //      /apis/group/version
        let job =
            try_join_all(group_versions.iter().map(|gv| self.fetch_api_resources(gv))).await?;

        Ok(job.into_iter().flatten().collect::<BTreeSet<_>>().into())
    }

    async fn fetch_api_versions(&self) -> Result<Vec<GroupVersion>> {
        let api_versions: APIVersions = self.client.request("api").await?;

        let ret = api_versions
            .versions
            .iter()
            .map(|v| GroupVersion {
                group: String::default(),
                version: v.to_string(),
                preferred_version: false,
            })
            .collect();

        Ok(ret)
    }

    async fn fetch_api_groups(&self) -> Result<Vec<GroupVersion>> {
        let api_groups: APIGroupList = self.client.request("apis").await?;

        let ret = api_groups
            .groups
            .into_iter()
            .flat_map(|group| {
                group
                    .versions
                    .iter()
                    .map(|gv| {
                        if let Some(ref pv) = group.preferred_version {
                            GroupVersion {
                                group: group.name.to_string(),
                                version: gv.version.to_string(),
                                preferred_version: pv.version == gv.version,
                            }
                        } else {
                            GroupVersion {
                                group: group.name.to_string(),
                                version: gv.version.to_string(),
                                preferred_version: false,
                            }
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .collect();

        Ok(ret)
    }

    async fn fetch_api_resources(&self, gv: &GroupVersion) -> Result<Vec<ApiResource>> {
        #[derive(Debug, Deserialize)]
        #[serde(rename_all = "camelCase")]
        #[allow(dead_code)]
        struct APIResourceList {
            types: Option<TypeMeta>,
            group_version: String,
            resources: Vec<APIResource>,
        }

        let result = self
            .client
            .request::<APIResourceList>(&gv.api_url())
            .await?;

        Ok(result
            .resources
            .into_iter()
            .filter(|resource| resource.contain_list_verb())
            .map(|resource| {
                if gv.group.is_empty() {
                    ApiResource::Api {
                        name: resource.name,
                        version: gv.version.to_string(),
                        namespaced: resource.namespaced,
                    }
                } else {
                    ApiResource::Apis {
                        name: resource.name,
                        group: gv.group.to_string(),
                        version: gv.version.to_string(),
                        preferred_version: gv.preferred_version,
                        namespaced: resource.namespaced,
                    }
                }
            })
            .collect())
    }
}

pub async fn fetch_api_resources(client: &KubeClient) -> Result<ApiResources> {
    let api_resources = FetchApiResources::new(client).fetch_all().await?;

    Ok(api_resources)
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
        #[case(ApiResource::Api { name: "pods".into(), version: "v1".into(), namespaced: true }, "pods")]
        #[case(ApiResource::Apis { name: "horizontalpodautoscalers".into(), group: "autoscaling".into(), version: "v2".into(), preferred_version: true, namespaced: true }, "horizontalpodautoscalers.autoscaling (*v2)")]
        #[case(ApiResource::Apis { name: "horizontalpodautoscalers".into(), group: "autoscaling".into(), version: "v1".into(), preferred_version: false, namespaced: true }, "horizontalpodautoscalers.autoscaling (v1)")]
        #[test]
        fn to_string(#[case] key: ApiResource, #[case] expected: &str) {
            assert_eq!(key.to_string(), expected)
        }
    }
}
