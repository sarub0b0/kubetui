use anyhow::Result;
use futures::future::try_join_all;
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    features::{api_resources::kube::ApiResource, yaml::message::YamlResourceListItem},
    kube::KubeClientRequest,
};

use super::single_namespace::FetchResourceListSingleNamespace;

pub(super) struct FetchResourceListMultipleNamespaces<'a, C: KubeClientRequest> {
    client: &'a C,
    namespaces: &'a [String],
    api: &'a ApiResource,
}

impl<'a, C: KubeClientRequest> FetchResourceListMultipleNamespaces<'a, C> {
    pub(super) fn new(client: &'a C, namespaces: &'a [String], api: &'a ApiResource) -> Self {
        Self {
            client,
            namespaces,
            api,
        }
    }

    pub(super) async fn fetch(&self) -> Result<Vec<YamlResourceListItem>> {
        let jobs = try_join_all(self.namespaces.iter().map(|ns| async move {
            FetchResourceListSingleNamespace::new(self.client, ns, self.api)
                .fetch()
                .await
        }))
        .await?;

        let namespace_digit = self
            .namespaces
            .iter()
            .map(|ns| ns.graphemes(true).count())
            .max()
            .unwrap_or(0);

        let list = jobs
            .into_iter()
            .flat_map(|items| {
                items
                    .into_iter()
                    .map(|mut item| {
                        item.value = format!(
                            "{:digit$}  {}",
                            item.namespace,
                            item.name,
                            digit = namespace_digit
                        );
                        item
                    })
                    .collect::<Vec<YamlResourceListItem>>()
            })
            .collect();

        Ok(list)
    }
}
