use crossbeam::channel::Sender;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{
    APIGroupList, APIResource, APIResourceList, APIVersions, GroupVersionForDiscovery,
};
use k8s_openapi::Resource;
use kube::Client;
use std::sync::Arc;
use std::time;

use super::{
    request::{get_request, get_table_request},
    {v1_table::*, ApiResources, KubeArgs, Namespaces},
    {Event, Kube},
};

use futures::future::join_all;
use serde_json::Value as JsonValue;

use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
struct APIInfo {
    api_version: String,
    api_group: String,
    api_group_version: String,
    api_resource: APIResource,
    preferred_version: Option<bool>,
}

#[derive(Debug)]
struct GroupVersion {
    group: String,
    version: String,
    preferred_version: Option<bool>,
}

fn is_preferred_version(
    version: &str,
    preferred_version: &Option<GroupVersionForDiscovery>,
) -> Option<bool> {
    preferred_version.as_ref().map(|gv| gv.version == version)
}

pub async fn apis_list(client: &Client, server_url: &str) -> Vec<String> {
    let api_info_list = get_all_api_info(client, server_url).await;

    let set: HashSet<String> = api_info_list
        .iter()
        .map(|api_info| {
            if api_info.api_group.is_empty() {
                api_info.api_resource.name.to_string()
            } else {
                format!("{}.{}", api_info.api_resource.name, api_info.api_group)
            }
        })
        .collect();

    let mut ret: Vec<String> = set.into_iter().collect();
    ret.sort();

    ret
}

async fn get_all_api_info(client: &Client, server_url: &str) -> Vec<APIInfo> {
    let mut group_versions = Vec::new();

    let result: Result<APIVersions, kube::Error> = client
        .request(get_request(server_url, "api").unwrap())
        .await;

    if let Ok(api_versions) = result.as_ref() {
        api_versions.versions.iter().for_each(|v| {
            group_versions.push(GroupVersion {
                group: String::default(),
                version: v.to_string(),
                preferred_version: None,
            })
        });
    }

    let result: Result<APIGroupList, kube::Error> = client
        .request(get_request(server_url, "apis").unwrap())
        .await;

    if let Ok(api_group_list) = result.as_ref() {
        api_group_list.groups.iter().for_each(|group| {
            group.versions.iter().for_each(|gv| {
                group_versions.push(GroupVersion {
                    group: group.name.to_string(),
                    version: gv.version.to_string(),
                    preferred_version: is_preferred_version(&gv.version, &group.preferred_version),
                })
            })
        });
    }

    // APIResourceListを取得
    //      /api/v1
    //      /api/v2
    //      /api/v*
    //      /apis/group/version

    let job = join_all(
        group_versions
            .iter()
            .map(|gv| api_resource_list_to_api_info_list(&client, server_url, gv)),
    )
    .await;

    job.into_iter().flatten().collect()
}

async fn api_resource_list_to_api_info_list(
    client: &Client,
    server_url: &str,
    gv: &GroupVersion,
) -> Vec<APIInfo> {
    let result = if gv.group.is_empty() {
        client
            .request::<APIResourceList>(
                get_request(server_url, &format!("api/{}", gv.version)).unwrap(),
            )
            .await
    } else {
        client
            .request::<APIResourceList>(
                get_request(server_url, &format!("apis/{}/{}", gv.group, gv.version)).unwrap(),
            )
            .await
    };

    if let Ok(list) = result {
        list.resources
            .iter()
            .filter(|resource| !resource.name.contains('/'))
            .map(|resource| APIInfo {
                api_group: gv.group.to_string(),
                api_version: APIResourceList::API_VERSION.to_string(),
                api_group_version: gv.version.to_string(),
                api_resource: resource.clone(),
                preferred_version: gv.preferred_version,
            })
            .collect()
    } else {
        Vec::new()
    }
}

fn convert_api_database(api_info_list: &[APIInfo]) -> HashMap<String, APIInfo> {
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

fn merge_tabels(fetch_data: Vec<FetchData>, insert_ns: bool) -> Table {
    if fetch_data.is_empty() {
        return Table::default();
    }

    let fetch_data = fetch_data;

    let mut base_table = fetch_data[0].table.clone();

    if insert_ns {
        let column_definitions = TableColumnDefinition {
            name: "Namespace".to_string(),
            ..Default::default()
        };

        base_table.column_definitions.insert(0, column_definitions);
    }

    fetch_data.into_iter().skip(1).for_each(|mut d| {
        if insert_ns {
            let ns = d.namespace.to_string();
            let mut rows = d
                .table
                .rows
                .into_iter()
                .map(|mut row| {
                    row.cells
                        .insert(0, Value(JsonValue::String(ns.to_string())));
                    row
                })
                .collect();

            base_table.rows.append(&mut rows);
        } else {
            base_table.rows.append(&mut d.table.rows);
        }
    });

    base_table
}

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

struct FetchData {
    namespace: String,
    table: Table,
}

async fn fetch_table_per_namespace(
    client: &Client,
    server_url: &str,
    path: String,
    ns: &str,
) -> Result<FetchData, kube::Error> {
    match client
        .request::<Table>(get_table_request(server_url, &path).unwrap())
        .await
    {
        Ok(t) => Ok(FetchData {
            namespace: ns.to_string(),
            table: t,
        }),
        Err(e) => Err(e),
    }
}

pub async fn get_api_resources(
    client: &Client,
    server_url: &str,
    namespaces: &[String],
    apis: &[String],
) -> Vec<String> {
    let api_info_list = get_all_api_info(client, server_url).await;

    let db = convert_api_database(&api_info_list);

    let mut ret = Vec::new();
    for api in apis {
        if let Some(info) = db.get(api) {
            let mut path = if info.api_group.is_empty() {
                format!("api/{}", info.api_group_version)
            } else {
                format!("apis/{}/{}", info.api_group, info.api_group_version)
            };

            let table = if info.api_resource.namespaced {
                let jobs = join_all(namespaces.iter().map(|ns| {
                    fetch_table_per_namespace(
                        client,
                        server_url,
                        format!("{}/namespaces/{}/{}", path, ns, "pods"),
                        ns,
                    )
                }))
                .await;

                let result: Vec<Result<FetchData, kube::Error>> = jobs.into_iter().collect();

                let result: Vec<FetchData> =
                    result.into_iter().flat_map(|table| table.ok()).collect();

                merge_tabels(result, insert_ns(namespaces))
            } else {
                path += &format!("/{}", info.api_resource.name);
                client
                    .request::<Table>(get_table_request(server_url, &path).unwrap())
                    .await
                    .unwrap_or_default()
            };

            if table.rows.is_empty() {
                continue;
            }

            let buf = header_by_api_info(&info) + &table.to_print();

            ret.push(buf);
            ret.push("".to_string());
        }
    }

    ret
}

pub async fn apis_loop(
    tx: Sender<Event>,
    namespace: Namespaces,
    api_resources: ApiResources,
    args: Arc<KubeArgs>,
) {
    let mut interval = tokio::time::interval(time::Duration::from_millis(1000));
    loop {
        interval.tick().await;
        let namespaces = namespace.read().await;
        let apis = api_resources.read().await;

        if apis.is_empty() {
            continue;
        }

        let result = get_api_resources(&args.client, &args.server_url, &namespaces, &apis).await;

        tx.send(Event::Kube(Kube::APIsResults(result))).unwrap();
    }
}
