use crossbeam::channel::Sender;
use k8s_openapi::api::core::v1::Service;

use crate::{
    error::Result,
    event::{
        kubernetes::{client::KubeClientRequest, network::NetworkMessage},
        Event,
    },
};

use super::DescriptionWorker;

pub(super) struct ServiceDescriptionWorker<'a, C>
where
    C: KubeClientRequest,
{
    client: &'a C,
    tx: &'a Sender<Event>,
    namespace: String,
    name: String,
}

#[async_trait::async_trait]
impl<'a, C> DescriptionWorker<'a, C> for ServiceDescriptionWorker<'a, C>
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

    async fn run(&self) -> Result<()> {
        let url = format!(
            "api/v1/namespaces/{}/services/{}",
            self.namespace, self.name
        );

        let res = self.client.request_text(&url).await?;

        let mut value: Service = serde_json::from_str(&res)?;

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
