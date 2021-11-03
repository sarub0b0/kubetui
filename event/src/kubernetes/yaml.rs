use std::collections::HashMap;

use k8s_openapi::apimachinery::pkg::apis::meta::v1::{ListMeta, ObjectMeta};
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

fn join_namespace_and_name(data: &HashMap<String, Vec<String>>) -> Vec<String> {
    let mut namespace_char_len = 0;

    for k in data.keys() {
        if namespace_char_len < k.len() {
            namespace_char_len = k.len();
        }
    }

    let mut list = data
        .iter()
        .flat_map(|(k, v)| {
            v.iter()
                .map(|v| format!("{:digit$}  {}", k, v, digit = namespace_char_len))
                .collect::<Vec<String>>()
        })
        .collect::<Vec<String>>();

    list.sort();

    list
}

async fn fetch_resource_list_multiple_namespaces(
    client: &KubeClient,
    namespaces: &[String],
    api: &APIInfo,
    request: &str,
) -> Result<Vec<String>> {
    let mut data = HashMap::new();

    for ns in namespaces {
        let item = fetch_resource_list_single_namespace(client, ns, api, request).await?;

        data.insert(ns.to_string(), item);
    }

    let result = join_namespace_and_name(&data);

    Ok(result)
}

async fn fetch_resource_list_single_namespace(
    client: &KubeClient,
    ns: &str,
    api: &APIInfo,
    request: &str,
) -> Result<Vec<String>> {
    let path = format!("{}/namespaces/{}/{}", api.api_url(), ns, request);

    let res: List = client.request(&path).await?;

    #[cfg(feature = "logging")]
    ::log::debug!("Fetch Resource List {:#?}", res);

    let list: Vec<String> = res
        .items
        .into_iter()
        .filter_map(|item| item.metadata.name)
        .collect();

    Ok(list)
}

async fn fetch_resource_list_not_namespaced(
    client: &KubeClient,
    api: &APIInfo,
    request: &str,
) -> Result<Vec<String>> {
    let path = format!("{}/{}", api.api_url(), request);

    let res: List = client.request(&path).await?;

    #[cfg(feature = "logging")]
    ::log::debug!("Fetch Resource List {:#?}", res);

    let list: Vec<String> = res
        .items
        .into_iter()
        .filter_map(|item| item.metadata.name)
        .collect();

    Ok(list)
}

/// 選択されているリソースのリストを取得する
///
/// ネームスペースが１つのとき OR namespaced が false のとき
///   リソース一覧を返す
///
/// ネームスペースが２つ以上のとき
///   ネームスペースを頭につけたリソース一覧を返す
///
///
///
pub async fn fetch_resource_list(
    client: &KubeClient,
    namespaces: &[String],
    api_database: &InnerApiDatabase,
    request: &str,
) -> Result<Vec<String>> {
    let api = api_database
        .get(request)
        .ok_or_else(|| Error::Raw(format!("Can't get {} from API Database", request)))?;

    let list = if api.api_resource.namespaced {
        if namespaces.len() == 1 {
            fetch_resource_list_single_namespace(client, &namespaces[0], api, request).await
        } else {
            fetch_resource_list_multiple_namespaces(client, namespaces, api, request).await
        }
    } else {
        fetch_resource_list_not_namespaced(client, api, request).await
    };

    list
}

/// 選択されているリソースのyamlを取得する
///
/// namespaced が false のとき
///
/// namespaced が true のとき
///
///
///
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
    let path = if api.api_resource.namespaced {
        format!("{}/namespaces/{}/{}/{}", api.api_url(), ns, kind, name)
    } else {
        format!("{}/{}/{}", api.api_url(), kind, name)
    };

    let res = client.request_text(&path).await?;

    #[cfg(feature = "logging")]
    ::log::debug!("Fetch Resource yaml {}", res);

    // yaml dataに変換
    let yaml_data: serde_yaml::Value = serde_json::from_str(&res)?;

    let yaml_string: Vec<String> = serde_yaml::to_string(&yaml_data)?
        .lines()
        .map(ToString::to_string)
        .collect();

    Ok(yaml_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn namespaceとリソース名を連結() {
        let mut map: HashMap<String, Vec<String>> = HashMap::new();

        map.insert(
            "ns-0".to_string(),
            vec![
                "item-0".to_string(),
                "item-1".to_string(),
                "item-2".to_string(),
            ],
        );

        map.insert(
            "ns-00".to_string(),
            vec![
                "item-0".to_string(),
                "item-1".to_string(),
                "item-2".to_string(),
            ],
        );
        map.insert(
            "ns-000".to_string(),
            vec![
                "item-0".to_string(),
                "item-1".to_string(),
                "item-2".to_string(),
            ],
        );

        let actual = join_namespace_and_name(&map);

        assert_eq!(
            vec![
                "ns-0    item-0".to_string(),
                "ns-0    item-1".to_string(),
                "ns-0    item-2".to_string(),
                "ns-00   item-0".to_string(),
                "ns-00   item-1".to_string(),
                "ns-00   item-2".to_string(),
                "ns-000  item-0".to_string(),
                "ns-000  item-1".to_string(),
                "ns-000  item-2".to_string(),
            ],
            actual
        )
    }
}
