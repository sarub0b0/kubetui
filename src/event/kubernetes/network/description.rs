mod ingress;
mod network_policy;
mod pod;
mod service;

use std::sync::{atomic::AtomicBool, Arc};

use self::{
    ingress::IngressDescriptionWorker, network_policy::NetworkPolicyDescriptionWorker,
    pod::PodDescriptionWorker, service::ServiceDescriptionWorker,
};

use super::*;

const INTERVAL: u64 = 3;

#[async_trait]
trait DescriptionWorker<'a> {
    fn new(client: &'a KubeClient, tx: &'a Sender<Event>, namespace: String, name: String) -> Self;

    async fn run(&self) -> Result<()>;
}

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
            Request::Pod(data) => self.fetch_description::<PodDescriptionWorker>(data).await,
            Request::Service(data) => {
                self.fetch_description::<ServiceDescriptionWorker>(data)
                    .await
            }
            Request::Ingress(data) => {
                self.fetch_description::<IngressDescriptionWorker>(data)
                    .await
            }
            Request::NetworkPolicy(data) => {
                self.fetch_description::<NetworkPolicyDescriptionWorker>(data)
                    .await
            }
        };

        if let Err(e) = ret {
            self.tx
                .send(NetworkMessage::Response(Err(anyhow!(e))).into())?;
        }
        Ok(())
    }
}

impl NetworkDescriptionWorker {
    async fn fetch_description<'a, Worker>(&'a self, data: &RequestData) -> Result<()>
    where
        Worker: DescriptionWorker<'a>,
    {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(INTERVAL));

        let worker = Worker::new(
            &self.client,
            &self.tx,
            data.namespace.clone(),
            data.name.clone(),
        );

        while !self
            .is_terminated
            .load(std::sync::atomic::Ordering::Relaxed)
        {
            interval.tick().await;

            worker.run().await?;
        }

        Ok(())
    }
}
