use k8s_openapi::api::networking::v1::NetworkPolicy;

use crate::{error::Result, event::kubernetes::client::KubeClientRequest};

use super::{Fetch, FetchedData};

pub(super) struct NetworkPolicyDescriptionWorker<'a, C>
where
    C: KubeClientRequest,
{
    client: &'a C,
    namespace: String,
    name: String,
}

#[async_trait::async_trait]
impl<'a, C> Fetch<'a, C> for NetworkPolicyDescriptionWorker<'a, C>
where
    C: KubeClientRequest,
{
    fn new(client: &'a C, namespace: String, name: String) -> Self {
        Self {
            client,
            namespace,
            name,
        }
    }

    async fn fetch(&self) -> Result<FetchedData> {
        let url = format!(
            "apis/networking.k8s.io/v1/namespaces/{}/networkpolicies/{}",
            self.namespace, self.name
        );

        let res = self.client.request_text(&url).await?;

        let mut value: NetworkPolicy = serde_json::from_str(&res)?;

        value.metadata.managed_fields = None;

        let value = serde_yaml::to_string(&value)?
            .lines()
            .skip(1)
            .map(ToString::to_string)
            .collect();

        Ok(value)
    }
}
