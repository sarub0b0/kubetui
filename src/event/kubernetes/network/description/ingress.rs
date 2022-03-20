use crossbeam::channel::Sender;
use k8s_openapi::api::networking::v1::Ingress;

use crate::{
    error::Result,
    event::{
        kubernetes::{client::KubeClientRequest, network::NetworkMessage},
        Event,
    },
};

use super::Fetch;

pub(super) struct IngressDescriptionWorker<'a, C>
where
    C: KubeClientRequest,
{
    client: &'a C,
    tx: &'a Sender<Event>,
    namespace: String,
    name: String,
}

#[async_trait::async_trait]
impl<'a, C> Fetch<'a, C> for IngressDescriptionWorker<'a, C>
where
    C: KubeClientRequest,
{
    fn new(client: &'a C, tx: &'a Sender<Event>, namespace: String, name: String) -> Self {
        Self {
            client,
            tx,
            namespace,
            name,
        }
    }

    async fn fetch(&self) -> Result<()> {
        let url = format!(
            "apis/networking.k8s.io/v1/namespaces/{}/ingresses/{}",
            self.namespace, self.name
        );

        let res = self.client.request_text(&url).await?;

        let mut value: Ingress = serde_json::from_str(&res)?;

        value.metadata.managed_fields = None;

        let value = serde_yaml::to_string(&value)?
            .lines()
            .skip(1)
            .map(ToString::to_string)
            .collect();

        self.tx.send(NetworkMessage::Response(Ok(value)).into())?;

        Ok(())
    }
}
