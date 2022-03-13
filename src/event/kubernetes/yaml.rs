use std::collections::HashMap;

use futures::future::try_join_all;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use serde::Deserialize;

use super::{
    api_resources::{APIInfo, InnerApiDatabase},
    client::{KubeClient, KubeClientRequest},
};
use crate::error::{Error, Result};

#[derive(Default, Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct List {
    items: Vec<Item>,
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

async fn fetch_resource_list_single_namespace_for_multiple_namespaces(
    client: &KubeClient,
    ns: &str,
    api: &APIInfo,
    kind: &str,
) -> Result<(String, Vec<String>)> {
    let item = fetch_resource_list_single_namespace(client, ns, api, kind).await?;
    Ok((ns.to_string(), item))
}

async fn fetch_resource_list_multiple_namespaces(
    client: &KubeClient,
    namespaces: &[String],
    api: &APIInfo,
    kind: &str,
) -> Result<Vec<String>> {
    let mut data = HashMap::new();

    let jobs = try_join_all(namespaces.iter().map(|ns| {
        fetch_resource_list_single_namespace_for_multiple_namespaces(client, ns, api, kind)
    }))
    .await?;

    for (ns, item) in jobs {
        data.insert(ns, item);
    }

    let result = join_namespace_and_name(&data);

    Ok(result)
}

async fn fetch_resource_list_single_namespace(
    client: &KubeClient,
    ns: &str,
    api: &APIInfo,
    kind: &str,
) -> Result<Vec<String>> {
    let path = format!("{}/namespaces/{}/{}", api.api_url(), ns, kind);

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
    kind: &str,
) -> Result<Vec<String>> {
    let path = format!("{}/{}", api.api_url(), kind);

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
pub async fn fetch_resource_list(
    client: &KubeClient,
    namespaces: &[String],
    api_database: &InnerApiDatabase,
    kind: &str,
) -> Result<Vec<String>> {
    let api = api_database
        .get(kind)
        .ok_or_else(|| Error::Raw(format!("Can't get {} from API Database", kind)))?;

    #[cfg(feature = "logging")]
    ::log::info!("[fetch_resource_list] Select APIInfo: {:#?}", api);

    let kind = &api.api_resource.name;
    let list = if api.api_resource.namespaced {
        if namespaces.len() == 1 {
            fetch_resource_list_single_namespace(client, &namespaces[0], api, kind).await
        } else {
            fetch_resource_list_multiple_namespaces(client, namespaces, api, kind).await
        }
    } else {
        fetch_resource_list_not_namespaced(client, api, kind).await
    };

    list
}

/// 選択されているリソースのyamlを取得する
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
    let kind = &api.api_resource.name;
    let path = if api.api_resource.namespaced {
        format!("{}/namespaces/{}/{}/{}", api.api_url(), ns, kind, name)
    } else {
        format!("{}/{}/{}", api.api_url(), kind, name)
    };

    let res = client.request_text(&path).await?;

    #[cfg(feature = "logging")]
    ::log::debug!("Fetch Resource yaml {}", res);

    // yaml dataに変換
    let yaml = convert_json_to_yaml_string_vec(&res)?;

    Ok(yaml)
}

fn convert_json_to_yaml_string_vec(json: &str) -> Result<Vec<String>> {
    let yaml_data: serde_yaml::Value = serde_json::from_str(json)?;

    let yaml_string = serde_yaml::to_string(&yaml_data)?
        .lines()
        .skip(1)
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

    #[test]
    fn json文字列からyaml文字列に変換() {
        let json = r#"
        {
            "apiVersion": "v1",
            "kind": "Pod",
            "metadata": {
                "name": "nginx-deployment",
                "namespace": "default"
            }
        }"#;

        let yaml = convert_json_to_yaml_string_vec(json).unwrap();

        assert_eq!(
            vec![
                "apiVersion: v1".to_string(),
                "kind: Pod".to_string(),
                "metadata:".to_string(),
                "  name: nginx-deployment".to_string(),
                "  namespace: default".to_string(),
            ],
            yaml
        )
    }
}
