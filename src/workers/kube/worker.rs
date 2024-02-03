use async_trait::async_trait;
use tokio::task::{AbortHandle, JoinHandle};

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
