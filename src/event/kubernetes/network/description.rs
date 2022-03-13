mod ingress;
mod network_policy;
mod pod;
mod service;

use std::sync::{atomic::AtomicBool, Arc};

use crate::event::kubernetes::client::KubeClientRequest;

use self::{
    ingress::IngressDescriptionWorker, network_policy::NetworkPolicyDescriptionWorker,
    pod::PodDescriptionWorker, service::ServiceDescriptionWorker,
};

use super::*;

const INTERVAL: u64 = 3;

#[async_trait]
trait DescriptionWorker<'a, C: KubeClientRequest + Clone> {
    fn new(client: &'a C, tx: &'a Sender<Event>, namespace: String, name: String) -> Self;

    async fn run(&self) -> Result<()>;
}

#[derive(Clone)]
pub struct NetworkDescriptionWorker<C>
where
    C: KubeClientRequest + Clone,
{
    is_terminated: Arc<AtomicBool>,
    tx: Sender<Event>,
    client: C,
    req: Request,
}

impl<C> NetworkDescriptionWorker<C>
where
    C: KubeClientRequest + Clone,
{
    pub fn new(is_terminated: Arc<AtomicBool>, tx: Sender<Event>, client: C, req: Request) -> Self {
        Self {
            is_terminated,
            tx,
            client,
            req,
        }
    }
}

#[async_trait]
impl<C> Worker for NetworkDescriptionWorker<C>
where
    C: KubeClientRequest + Clone,
{
    type Output = Result<()>;

    async fn run(&self) -> Self::Output {
        let ret = match &self.req {
            Request::Pod(data) => {
                self.fetch_description::<PodDescriptionWorker<C>>(data)
                    .await
            }
            Request::Service(data) => {
                self.fetch_description::<ServiceDescriptionWorker<C>>(data)
                    .await
            }
            Request::Ingress(data) => {
                self.fetch_description::<IngressDescriptionWorker<C>>(data)
                    .await
            }
            Request::NetworkPolicy(data) => {
                self.fetch_description::<NetworkPolicyDescriptionWorker<C>>(data)
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

impl<C> NetworkDescriptionWorker<C>
where
    C: KubeClientRequest + Clone,
{
    async fn fetch_description<'a, Worker>(&'a self, data: &RequestData) -> Result<()>
    where
        Worker: DescriptionWorker<'a, C>,
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

#[cfg(test)]
mod tests {
    use super::*;
    mod run {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn is_terminatedで処理を停止したときokを返す() {
            unimplemented!()
        }

        #[test]
        fn 内部でエラーがでたとき処理を停止してtxにerrを送信してokを返す() {
            unimplemented!()
        }
    }
    mod fetch_description {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn 正常系のときtxにデータを送信する() {
            unimplemented!()
        }

        #[test]
        fn is_terminatedがtrueのときループを抜けてokを返す() {
            unimplemented!()
        }

        #[test]
        fn 内部でエラーがでたときループを抜けてerrを返す() {
            unimplemented!()
        }
    }
}
