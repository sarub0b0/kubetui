use std::sync::{atomic::AtomicBool, Arc};

use async_trait::async_trait;

use crossbeam::channel::Sender;
use tokio::task::{AbortHandle, JoinHandle};

use crate::message::Message;

use super::{KubeClient, SharedTargetNamespaces};

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

#[async_trait]
pub trait AbortWorker {
    async fn run(&self);

    fn spawn(&self) -> AbortHandle
    where
        Self: Clone + Send + Sync + 'static,
    {
        let worker = self.clone();
        tokio::spawn(async move { worker.run().await }).abort_handle()
    }
}

#[derive(Clone)]
pub struct PollerBase {
    pub is_terminated: Arc<AtomicBool>,
    pub tx: Sender<Message>,
    pub shared_target_namespaces: SharedTargetNamespaces,
    pub kube_client: KubeClient,
}
