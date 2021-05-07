use k8s_openapi::apimachinery::pkg::apis::meta::v1::{
    APIGroupList, APIResource, APIResourceList, APIVersions, GroupVersionForDiscovery,
};
use k8s_openapi::Resource;
use kube::Client;

use super::request::get_request;

use futures::future::join_all;

use std::collections::HashSet;

#[derive(Debug)]
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
    match preferred_version {
        Some(gv) => Some(gv.version == version),
        None => None,
    }
}

pub async fn apis_list(client: &Client, server_url: &str) -> Vec<String> {
    let api_info_list = get_all_api_info(client, server_url).await;

    let set: HashSet<String> = api_info_list
        .iter()
        .map(|api_info| {
            if api_info.api_group == "" {
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
    let result = if gv.group == "" {
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
            .filter(|resource| !resource.name.contains("/"))
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

pub async fn get_api_resources(
    client: &Client,
    server_url: &str,
    ns: &str,
    apis: Vec<String>,
) -> Vec<Vec<String>> {
    let api_info_list = get_all_api_info(client, server_url).await;

    apis.into_iter().map(|api| vec![api]).collect()
}
