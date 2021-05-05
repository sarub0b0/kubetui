use k8s_openapi::apimachinery::pkg::apis::meta::v1::{
    APIGroup, APIGroupList, APIResource, APIResourceList, APIVersions, GroupVersionForDiscovery,
};
use kube::Client;

use super::request::get_request;

use futures::future::join_all;
use futures::Future;

use std::collections::HashSet;

pub async fn apis_list(client: Client, server_url: &str) -> Vec<String> {
    let mut ret = api_group_version_list(&client, server_url).await;

    ret.append(&mut api_resource_list(&client, server_url).await);

    ret
}

async fn api_resource_list(client: &Client, server_url: &str) -> Vec<String> {
    let res: Result<APIVersions, kube::Error> = client
        .request(get_request(server_url, "api").unwrap())
        .await;

    if let Ok(api_versions) = res {
        let versions = api_versions.versions;

        let job = join_all(versions.iter().map(|v| {
            client
                .request::<APIResourceList>(get_request(server_url, &format!("api/{}", v)).unwrap())
        }))
        .await;

        job.iter()
            .flat_map(|v| v.as_ref().unwrap().clone().resources)
            .filter_map(|r| {
                if !r.name.contains(&String::from("/")) {
                    Some(r)
                } else {
                    None
                }
            })
            .map(|r| r.name.clone())
            .collect()
    } else {
        vec![]
    }
}

struct APIData<'a> {
    pub name: &'a str,
    pub version: &'a GroupVersionForDiscovery,
}

async fn api_group_version_list(client: &Client, server_url: &str) -> Vec<String> {
    let res: Result<APIGroupList, kube::Error> = client
        .request(get_request(server_url, "apis").unwrap())
        .await;

    if let Ok(list) = res {
        let preferred_list: Vec<APIData> = list
            .groups
            .iter()
            .flat_map(|group| {
                group.versions.iter().map(move |v| APIData {
                    name: group.name.as_str(),
                    version: v,
                })
            })
            .collect();

        let job = join_all(preferred_list.iter().map(|api| async move {
            (
                api.name,
                client
                    .request::<APIResourceList>(
                        get_request(server_url, &format!("apis/{}", api.version.group_version))
                            .unwrap(),
                    )
                    .await,
            )
        }))
        .await;

        let set: HashSet<String> = job
            .iter()
            .flat_map(|(group_name, response)| {
                response
                    .as_ref()
                    .unwrap()
                    .resources
                    .iter()
                    .filter_map(|r| {
                        if !r.name.contains(&String::from("/")) {
                            Some(r)
                        } else {
                            None
                        }
                    })
                    .map(move |r| format!("{}.{}", r.name, group_name))
            })
            .collect();

        set.into_iter().collect()
    } else {
        Vec::new()
    }
}
