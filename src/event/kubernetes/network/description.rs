use std::sync::{atomic::AtomicBool, Arc};

use super::*;

#[derive(Clone)]
pub struct NetworkDescriptionWorker {
    is_terminated: Arc<AtomicBool>,
    tx: Sender<Event>,
    client: KubeClient,
    req: Request,
}

impl NetworkDescriptionWorker {
    pub fn new(
        is_terminated: Arc<AtomicBool>,
        tx: Sender<Event>,
        client: KubeClient,
        req: Request,
    ) -> Self {
        Self {
            is_terminated,
            tx,
            client,
            req,
        }
    }
}

#[async_trait]
impl Worker for NetworkDescriptionWorker {
    type Output = Result<()>;

    async fn run(&self) -> Self::Output {
        let ret = match &self.req {
            Request::Pod(data) => self.fetch_pod_description(data).await,
            Request::Service(data) => self.fetch_service_description(data).await,
            Request::Ingress(data) => self.fetch_ingress_description(data).await,
        };

        if let Err(e) = ret {
            self.tx
                .send(NetworkMessage::Response(Err(anyhow!(e))).into())?;
        }
        Ok(())
    }
}

impl NetworkDescriptionWorker {
    async fn fetch_pod_description(&self, data: &RequestData) -> Result<()> {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));

        let url = format!("api/v1/namespaces/{}/pods/{}", data.namespace, data.name);

        while !self
            .is_terminated
            .load(std::sync::atomic::Ordering::Relaxed)
        {
            interval.tick().await;

            let res = self.client.request_text(&url).await?;

            let value: serde_yaml::Value = serde_json::from_str(&res)?;

            let yaml = serde_yaml::to_string(&value)?
                .lines()
                .skip(1)
                .map(ToString::to_string)
                .collect();

            self.tx.send(NetworkMessage::Response(Ok(yaml)).into())?;
        }
        Ok(())
    }
}

impl NetworkDescriptionWorker {
    async fn fetch_service_description(&self, data: &RequestData) -> Result<()> {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));

        let url = format!(
            "api/v1/namespaces/{}/services/{}",
            data.namespace, data.name
        );

        while !self
            .is_terminated
            .load(std::sync::atomic::Ordering::Relaxed)
        {
            interval.tick().await;

            let res = self.client.request_text(&url).await?;

            let value: serde_yaml::Value = serde_json::from_str(&res)?;

            let yaml = serde_yaml::to_string(&value)?
                .lines()
                .skip(1)
                .map(ToString::to_string)
                .collect();

            self.tx.send(NetworkMessage::Response(Ok(yaml)).into())?;
        }

        Ok(())
    }
}

impl NetworkDescriptionWorker {
    async fn fetch_ingress_description(&self, data: &RequestData) -> Result<()> {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));

        let url = format!(
            "apis/networking.k8s.io/v1/namespaces/{}/ingresses/{}",
            data.namespace, data.name
        );

        while !self
            .is_terminated
            .load(std::sync::atomic::Ordering::Relaxed)
        {
            interval.tick().await;

            let res = self.client.request_text(&url).await?;

            let value: serde_yaml::Value = serde_json::from_str(&res)?;

            let yaml = serde_yaml::to_string(&value)?
                .lines()
                .skip(1)
                .map(ToString::to_string)
                .collect();

            self.tx.send(NetworkMessage::Response(Ok(yaml)).into())?;
        }

        Ok(())
    }
}
