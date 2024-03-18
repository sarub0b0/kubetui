use anyhow::Result;

use crate::{
    features::api_resources::kube::ApiResource,
    logger,
    workers::kube::{client::KubeClientRequest, yaml::YamlResourceListItem},
};

use super::List;

pub(super) struct FetchResourceListNotNamespaced<'a, C: KubeClientRequest> {
    client: &'a C,
    api: &'a ApiResource,
    kind: &'a str,
}

impl<'a, C: KubeClientRequest> FetchResourceListNotNamespaced<'a, C> {
    pub(super) fn new(client: &'a C, api: &'a ApiResource, kind: &'a str) -> Self {
        Self { client, api, kind }
    }

    pub(super) async fn fetch(&self) -> Result<Vec<YamlResourceListItem>> {
        let path = format!("{}/{}", self.api.group_version_url(), self.kind);
        logger!(info, "Fetching resource [{}]", path);

        let res: List = self.client.request(&path).await?;

        logger!(info, "Fetched resource - {:?}", res);

        Ok(res
            .items
            .into_iter()
            .filter_map(|item| {
                item.metadata.name.map(|name| YamlResourceListItem {
                    namespace: "".to_string(),
                    name: name.to_string(),
                    kind: self.api.clone(),
                    value: name,
                })
            })
            .collect())
    }
}
