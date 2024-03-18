use anyhow::Result;

use crate::{
    features::{api_resources::kube::ApiResource, yaml::message::YamlResourceListItem},
    logger,
    workers::kube::client::KubeClientRequest,
};

use super::List;

pub(super) struct FetchResourceListSingleNamespace<'a, C: KubeClientRequest> {
    client: &'a C,
    ns: &'a str,
    api: &'a ApiResource,
    kind: &'a str,
}

impl<'a, C: KubeClientRequest> FetchResourceListSingleNamespace<'a, C> {
    pub(super) fn new(client: &'a C, ns: &'a str, api: &'a ApiResource, kind: &'a str) -> Self {
        Self {
            client,
            ns,
            api,
            kind,
        }
    }

    pub(super) async fn fetch(&self) -> Result<Vec<YamlResourceListItem>> {
        let path = format!(
            "{}/namespaces/{}/{}",
            self.api.group_version_url(),
            self.ns,
            self.kind
        );

        logger!(info, "Fetching resource [{}]", path);

        let res: List = self.client.request(&path).await?;

        logger!(info, "Fetched resource - {:?}", res);

        Ok(res
            .items
            .into_iter()
            .filter_map(|item| {
                item.metadata.name.map(|name| YamlResourceListItem {
                    namespace: self.ns.to_string(),
                    name: name.to_string(),
                    kind: self.api.clone(),
                    value: name,
                })
            })
            .collect())
    }
}
