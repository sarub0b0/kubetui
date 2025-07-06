mod multiple_namespaces;
mod not_namespace;
mod single_namespace;

use anyhow::{Result, anyhow};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use serde::Deserialize;

use crate::{
    features::{
        api_resources::kube::{ApiResource, ApiResources},
        yaml::message::YamlResourceList,
    },
    kube::KubeClientRequest,
    workers::kube::TargetNamespaces,
};

use self::{
    multiple_namespaces::FetchResourceListMultipleNamespaces,
    not_namespace::FetchResourceListNotNamespaced,
    single_namespace::FetchResourceListSingleNamespace,
};

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

pub struct FetchResourceList<'a, C: KubeClientRequest> {
    client: &'a C,
    req: ApiResource,
    target_namespaces: &'a TargetNamespaces,
    api_resources: &'a ApiResources,
}

impl<'a, C: KubeClientRequest> FetchResourceList<'a, C> {
    pub fn new(
        client: &'a C,
        req: ApiResource,
        api_resources: &'a ApiResources,
        target_namespaces: &'a TargetNamespaces,
    ) -> Self {
        Self {
            client,
            req,
            api_resources,
            target_namespaces,
        }
    }

    /// 選択されているリソースのリストを取得する
    ///
    /// ネームスペースが１つのとき OR namespaced が false のとき
    ///   リソース一覧を返す
    ///
    /// ネームスペースが２つ以上のとき
    ///   ネームスペースを頭につけたリソース一覧を返す
    ///
    pub async fn fetch(&self) -> Result<YamlResourceList> {
        let kind = &self.req;

        let api = self
            .api_resources
            .iter()
            .find(|api| *api == kind)
            .ok_or_else(|| anyhow!("Can't get {} from API resource", kind))?;

        let kind = &api.name();
        let list = if api.is_namespaced() {
            if self.target_namespaces.len() == 1 {
                FetchResourceListSingleNamespace::new(
                    self.client,
                    &self.target_namespaces[0],
                    api,
                    kind,
                )
                .fetch()
                .await?
            } else {
                FetchResourceListMultipleNamespaces::new(
                    self.client,
                    self.target_namespaces,
                    api,
                    kind,
                )
                .fetch()
                .await?
            }
        } else {
            FetchResourceListNotNamespaced::new(self.client, api, kind)
                .fetch()
                .await?
        };

        Ok(YamlResourceList::new(list))
    }
}
