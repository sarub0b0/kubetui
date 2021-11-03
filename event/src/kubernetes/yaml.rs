use k8s_openapi::apimachinery::pkg::apis::meta::v1::{ListMeta, ObjectMeta, Time};
use serde::Deserialize;

use super::{
    api_resources::{APIInfo, InnerApiDatabase},
    client::KubeClient,
};
use crate::error::{Error, Result};

#[derive(Default, Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct List {
    api_version: String,
    items: Vec<Item>,
    kind: String,
    metadata: Option<ListMeta>,
}

#[derive(Default, Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Item {
    metadata: ObjectMeta,
}

async fn fetch_resource_list_multiple_namespaces() -> Result<Vec<String>> {
    Ok(Vec::new())
}

async fn fetch_resource_list_single_namespace(
    client: &KubeClient,
    ns: &str,
    api: &APIInfo,
    request: &str,
) -> Result<Vec<String>> {
    let res = client.request(&format!("")).await?;

    Ok(Vec::new())
}

/// 選択されているリソースのリストを取得する
///
/// ネームスペースが１つのとき
///
/// ネームスペースが２つ以上のとき
pub async fn fetch_resource_list(
    client: &KubeClient,
    namespaces: &[String],
    api_database: &InnerApiDatabase,
    request: &str,
) -> Result<Vec<String>> {
    let api = api_database
        .get(request)
        .ok_or_else(|| Error::Raw(format!("Can't get {} from API Database", request)))?;

    // let ret = if namespaces.len() == 1 {
    //     fetch_resource_list_single_namespace(client, &namespaces[0], api, request).await
    // } else {
    //     fetch_resource_list_multiple_namespaces().await
    // };

    // let table = if api.api_resource.namespaced {
    //     client.request(&format!("")).await?
    // } else {
    //     client.request(&format!("")).await?
    // };

    let res: List = client
        .request(&format!(
            "{}/namespaces/{}/{}",
            api.api_url(),
            namespaces[0],
            request
        ))
        .await?;

    #[cfg(feature = "logging")]
    ::log::debug!("Fetch Resource List {:#?}", res);

    let ret: Vec<String> = res
        .items
        .into_iter()
        .filter_map(|item| item.metadata.name)
        .collect();

    Ok(ret)
}

pub async fn fetch_resource_yaml(
    client: &KubeClient,
    api_database: &InnerApiDatabase,
    kind: String,
    name: String,
    ns: String,
) -> Result<Vec<String>> {
    let api = api_database
        .get(&kind)
        .ok_or_else(|| Error::Raw(format!("Can't get {} from API Database", kind)))?;

    // json string data
    let res = client
        .request_text(&format!(
            "{}/namespaces/{}/{}/{}",
            api.api_url(),
            ns,
            kind,
            name
        ))
        .await?;

    #[cfg(feature = "logging")]
    ::log::debug!("Fetch Resource List {}", res);

    // yaml dataに変換
    let yaml_data: serde_yaml::Value = serde_json::from_str(&res)?;

    let yaml_string: Vec<String> = serde_yaml::to_string(&yaml_data)?
        .lines()
        .map(ToString::to_string)
        .collect();

    Ok(yaml_string)
}
