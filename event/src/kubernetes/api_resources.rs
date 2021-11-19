use super::{
    worker::{PollWorker, Worker},
    KubeClient,
    {v1_table::*, ApiResources},
    {Event, Kube},
};

use super::{metric_type::*, WorkerResult};
use crate::error::Result;

use k8s_openapi::apimachinery::pkg::apis::meta::v1::{
    APIGroupList, APIResource, APIResourceList, APIVersions, GroupVersionForDiscovery,
};
use k8s_openapi::Resource;

use std::{sync::Arc, time};
use tokio::{sync::RwLock, time::Instant};

use futures::future::try_join_all;
use serde_json::Value as JsonValue;

use async_trait::async_trait;
use std::collections::HashMap;

pub type ApiDatabase = Arc<RwLock<InnerApiDatabase>>;
pub type InnerApiDatabase = HashMap<String, APIInfo>;

#[derive(Clone)]
pub struct ApiPollWorker {
    inner: PollWorker,
    api_resources: ApiResources,
    api_database: ApiDatabase,
}

impl ApiPollWorker {
    pub fn new(inner: PollWorker, api_resources: ApiResources, api_database: ApiDatabase) -> Self {
        Self {
            inner,
            api_resources,
            api_database,
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
                    namespaces,
                    kube_client,
                },
            api_resources,
            api_database,
        } = self;

        {
            let mut db = api_database.write().await;

            *db = fetch_api_database(kube_client).await?;
        }

        let mut interval = tokio::time::interval(time::Duration::from_millis(1000));

        let mut last_tick = Instant::now();
        let tick_rate = time::Duration::from_secs(10);

        while !is_terminated.load(std::sync::atomic::Ordering::Relaxed) {
            interval.tick().await;
            let namespaces = namespaces.read().await;
            let apis = api_resources.read().await;

            if apis.is_empty() {
                continue;
            }

            if tick_rate < last_tick.elapsed() {
                last_tick = Instant::now();

                let mut db = api_database.write().await;

                *db = fetch_api_database(kube_client).await?;
            }

            let db = api_database.read().await;
            let result = get_api_resources(kube_client, &namespaces, &apis, &db).await;

            tx.send(Event::Kube(Kube::APIsResults(result))).unwrap();
        }

        Ok(WorkerResult::Terminated)
    }
}

#[derive(Debug, Clone)]
pub struct APIInfo {
    pub api_version: String,
    pub api_group: String,
    pub api_group_version: String,
    pub api_resource: APIResource,
    pub preferred_version: Option<bool>,
}

impl APIInfo {
    pub fn api_url(&self) -> String {
        if self.api_group.is_empty() {
            format!("api/{}", self.api_group_version)
        } else {
            format!("apis/{}/{}", self.api_group, self.api_group_version)
        }
    }
}

#[derive(Debug)]
struct GroupVersion {
    group: String,
    version: String,
    preferred_version: Option<bool>,
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

fn is_preferred_version(
    version: &str,
    preferred_version: &Option<GroupVersionForDiscovery>,
) -> Option<bool> {
    preferred_version.as_ref().map(|gv| gv.version == version)
}

async fn get_all_api_info(client: &KubeClient) -> Result<Vec<APIInfo>> {
    let mut group_versions = Vec::new();

    let api_versions: APIVersions = client.request("api").await?;

    api_versions.versions.iter().for_each(|v| {
        group_versions.push(GroupVersion {
            group: String::default(),
            version: v.to_string(),
            preferred_version: None,
        })
    });

    let api_group_list: APIGroupList = client.request("apis").await?;

    api_group_list.groups.iter().for_each(|group| {
        group.versions.iter().for_each(|gv| {
            group_versions.push(GroupVersion {
                group: group.name.to_string(),
                version: gv.version.to_string(),
                preferred_version: is_preferred_version(&gv.version, &group.preferred_version),
            })
        })
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

async fn api_resource_list_to_api_info_list(
    client: &KubeClient,
    gv: &GroupVersion,
) -> Result<Vec<APIInfo>> {
    let result = client.request::<APIResourceList>(&gv.api_url()).await?;

    Ok(result
        .resources
        .iter()
        .filter(|resource| can_get_request(resource))
        .map(|resource| APIInfo {
            api_group: gv.group.to_string(),
            api_version: APIResourceList::API_VERSION.to_string(),
            api_group_version: gv.version.to_string(),
            api_resource: resource.clone(),
            preferred_version: gv.preferred_version,
        })
        .collect())
}

pub async fn fetch_api_database(client: &KubeClient) -> Result<InnerApiDatabase> {
    let api_info_list = get_all_api_info(client).await?;
    Ok(convert_api_database(&api_info_list))
}

pub fn apis_list_from_api_database(db: &InnerApiDatabase) -> Vec<String> {
    let mut ret: Vec<String> = db.iter().map(|(k, _)| k.to_string()).collect();

    ret.sort();

    ret
}

fn convert_api_database(api_info_list: &[APIInfo]) -> InnerApiDatabase {
    let mut db: HashMap<String, APIInfo> = HashMap::new();

    for info in api_info_list {
        let api_name = if info.api_group.is_empty() {
            info.api_resource.name.to_string()
        } else {
            format!("{}.{}", info.api_resource.name, info.api_group)
        };

        let mut is_insert = false;
        if db.contains_key(&api_name) {
            if let Some(pv) = info.preferred_version {
                if pv {
                    is_insert = true;
                }
            }
        } else {
            is_insert = true;
        }

        if is_insert {
            db.insert(api_name, info.clone());
        }
    }

    db
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
fn header_by_api_info(info: &APIInfo) -> String {
    if info.api_group.is_empty() {
        format!("\x1b[90m[ {} ]\x1b[0m\n", info.api_resource.name)
    } else {
        format!(
            "\x1b[90m[ {}.{} ]\x1b[0m\n",
            info.api_resource.name, info.api_group
        )
    }
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
    Ok(try_fetch_table(client, path).await?)
}

async fn get_api_resources(
    client: &KubeClient,
    namespaces: &[String],
    apis: &[String],
    db: &InnerApiDatabase,
) -> Result<Vec<String>> {
    let mut ret = Vec::new();

    for api in apis {
        if let Some(info) = db.get(api) {
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

            ret.push(data);
            ret.push("".to_string());
        }
    }

    Ok(ret)
}
