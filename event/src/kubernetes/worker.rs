use std::sync::{atomic::AtomicBool, Arc};

use async_trait::async_trait;

use crossbeam::channel::Sender;
use tokio::task::JoinHandle;

use crate::Event;

use super::{KubeClient, Namespaces};

#[async_trait]
pub trait Worker {
    type Output;

    async fn run(&self) -> Self::Output;

    fn spawn(&self) -> JoinHandle<Self::Output>
    where
        Self: Clone + Send + Sync + 'static,
        Self::Output: Send,
    {
        let worker = self.clone();
        tokio::spawn(async move { worker.run().await })
    }
}

#[derive(Clone)]
pub struct PollWorker {
    pub is_terminated: Arc<AtomicBool>,
    pub tx: Sender<Event>,
    pub namespaces: Namespaces,
    pub kube_client: KubeClient,
}
