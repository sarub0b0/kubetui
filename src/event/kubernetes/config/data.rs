mod configmap;
mod secret;

use std::sync::{atomic::AtomicBool, Arc};

use async_trait::async_trait;
use crossbeam::channel::Sender;

use crate::{
    error::Result,
    event::{
        kubernetes::{client::KubeClient, worker::Worker},
        Event,
    },
};

use self::{configmap::ConfigMapDataWorker, secret::SecretDataWorker};

use super::{ConfigData, ConfigRequest, ConfigResponse, RequestData};

#[derive(Clone)]
pub struct ConfigsDataWorker {
    is_terminated: Arc<AtomicBool>,
    tx: Sender<Event>,
    client: KubeClient,
    req: ConfigRequest,
}

impl ConfigsDataWorker {
    pub fn new(
        is_terminated: Arc<AtomicBool>,
        tx: Sender<Event>,
        client: KubeClient,
        req: ConfigRequest,
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
impl Worker for ConfigsDataWorker {
    type Output = Result<()>;

    async fn run(&self) -> Self::Output {
        let ret = match &self.req {
            ConfigRequest::ConfigMap(_) => self.fetch_description::<ConfigMapDataWorker>().await,
            ConfigRequest::Secret(_) => self.fetch_description::<SecretDataWorker>().await,
        };

        if let Err(e) = ret {
            self.tx.send(ConfigResponse::Data(Err(e)).into())?;
        }

        Ok(())
    }
}

#[async_trait]
trait Fetch<'a> {
    fn new(client: &'a KubeClient, namespace: String, name: String) -> Self;

    async fn fetch(&self) -> Result<ConfigData>;
}

const INTERVAL: u64 = 3;

impl ConfigsDataWorker {
    async fn fetch_description<'a, Worker>(&'a self) -> Result<()>
    where
        Worker: Fetch<'a>,
    {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(INTERVAL));

        let RequestData { name, namespace } = self.req.data();

        let worker = Worker::new(&self.client, namespace.to_string(), name.to_string());

        while !self
            .is_terminated
            .load(std::sync::atomic::Ordering::Relaxed)
        {
            interval.tick().await;

            let fetched_data = worker.fetch().await;

            self.tx.send(ConfigResponse::Data(fetched_data).into())?;
        }

        Ok(())
    }
}
