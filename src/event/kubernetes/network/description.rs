mod ingress;
mod network_policy;
mod pod;
mod service;

use std::sync::{atomic::AtomicBool, Arc};

use crate::event::{
    kubernetes::{client::KubeClientRequest, worker::Worker},
    Event,
};

use self::{
    ingress::IngressDescriptionWorker, network_policy::NetworkPolicyDescriptionWorker,
    pod::PodDescriptionWorker, service::ServiceDescriptionWorker,
};

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use crossbeam::channel::Sender;

use super::{NetworkMessage, Request, RequestData};

const INTERVAL: u64 = 3;

#[async_trait]
trait DescriptionWorker<'a, C: KubeClientRequest> {
    fn new(client: &'a C, tx: &'a Sender<Event>, namespace: String, name: String) -> Self;

    async fn run(&self) -> Result<()>;
}

#[derive(Clone)]
pub struct NetworkDescriptionWorker<C>
where
    C: KubeClientRequest,
{
    is_terminated: Arc<AtomicBool>,
    tx: Sender<Event>,
    client: C,
    req: Request,
}

impl<C> NetworkDescriptionWorker<C>
where
    C: KubeClientRequest,
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
    C: KubeClientRequest,
{
    type Output = Result<()>;

    async fn run(&self) -> Self::Output {
        let ret = match &self.req {
            Request::Pod(_) => self.fetch_description::<PodDescriptionWorker<C>>().await,
            Request::Service(_) => {
                self.fetch_description::<ServiceDescriptionWorker<C>>()
                    .await
            }
            Request::Ingress(_) => {
                self.fetch_description::<IngressDescriptionWorker<C>>()
                    .await
            }
            Request::NetworkPolicy(_) => {
                self.fetch_description::<NetworkPolicyDescriptionWorker<C>>()
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
    C: KubeClientRequest,
{
    async fn fetch_description<'a, Worker>(&'a self) -> Result<()>
    where
        Worker: DescriptionWorker<'a, C>,
    {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(INTERVAL));

        let RequestData { name, namespace } = self.req.data();

        let worker = Worker::new(
            &self.client,
            &self.tx,
            namespace.to_string(),
            name.to_string(),
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

        #[ignore]
        #[test]
        fn is_terminatedで処理を停止したときokを返す() {
            unimplemented!()
        }

        #[ignore]
        #[test]
        fn 内部でエラーがでたとき処理を停止してtxにerrを送信してokを返す() {
            unimplemented!()
        }
    }
    mod fetch_description {
        use std::collections::BTreeMap;

        use super::*;

        use crossbeam::channel::{bounded, Receiver};
        use indoc::indoc;
        use k8s_openapi::api::core::v1::{Container, Pod, PodIP, PodSpec, PodStatus};
        use mockall::predicate::eq;
        use pretty_assertions::assert_eq;

        use crate::event::kubernetes::{client::mock::MockTestKubeClient, Kube};

        use self::{pod::FetchedIngressList, pod::FetchedPod, pod::FetchedServiceList};
        use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;

        fn setup_pod() -> FetchedPod {
            FetchedPod(Pod {
                metadata: ObjectMeta {
                    name: Some("test".into()),
                    namespace: Some("default".into()),
                    labels: Some(BTreeMap::from([
                        (
                            "controller-uid".into(),
                            "30d417a8-cb1c-467b-92fe-7819601a6ef8".into(),
                        ),
                        ("job-name".into(), "kubetui-text-color".into()),
                    ])),
                    ..Default::default()
                },
                spec: Some(PodSpec {
                    containers: vec![Container {
                        name: "job".into(),
                        image: Some("alpine".into()),
                        ..Default::default()
                    }],
                    ..Default::default()
                }),
                status: Some(PodStatus {
                    phase: Some("Succeeded".into()),
                    host_ip: Some("192.168.65.4".into()),
                    pod_ip: Some("10.1.0.21".into()),
                    pod_ips: Some(vec![PodIP {
                        ip: Some("10.1.0.21".into()),
                    }]),
                    ..Default::default()
                }),
            })
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn 正常系のときtxにデータを送信する() {
            let is_terminated = Arc::new(AtomicBool::new(false));
            let (tx, rx): (Sender<Event>, Receiver<Event>) = bounded(3);
            let mut client = MockTestKubeClient::new();
            client
                .expect_request::<FetchedPod>()
                .with(eq("api/v1/namespaces/default/pods/test"))
                .returning(|_| Ok(setup_pod()));
            client
                .expect_request::<FetchedServiceList>()
                .with(eq("api/v1/namespaces/default/services"))
                .returning(|_| Ok(FetchedServiceList::default()));
            client
                .expect_request::<FetchedIngressList>()
                .with(eq("apis/networking.k8s.io/v1/namespaces/default/ingresses"))
                .returning(|_| Ok(FetchedIngressList::default()));

            let req_data = RequestData {
                namespace: "default".to_string(),
                name: "test".to_string(),
            };
            let req = Request::Pod(req_data);

            let worker = NetworkDescriptionWorker::new(is_terminated.clone(), tx, client, req);

            let handle = tokio::spawn(async move {
                worker
                    .fetch_description::<PodDescriptionWorker<MockTestKubeClient>>()
                    .await
            });

            let event = rx.recv().unwrap();

            is_terminated.store(true, std::sync::atomic::Ordering::Relaxed);

            let _ret = handle.await;

            let expected: Vec<String> = indoc!(
                "
                pod:
                  labels:
                    controller-uid: 30d417a8-cb1c-467b-92fe-7819601a6ef8
                    job-name: kubetui-text-color
                  containers:
                    - name: job
                      image: alpine
                  hostIP: 192.168.65.4
                  podIP: 10.1.0.21
                  podIPs: 10.1.0.21
                  phase: Succeeded
                "
            )
            .lines()
            .map(ToString::to_string)
            .collect();

            if let Event::Kube(Kube::Network(NetworkMessage::Response(Ok(actual)))) = event {
                assert_eq!(actual, expected)
            } else {
                unreachable!()
            }
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn is_terminatedがtrueのときループを抜けてokを返す() {
            let (tx, _rx): (Sender<Event>, Receiver<Event>) = bounded(3);
            let client = MockTestKubeClient::new();

            let req_data = RequestData {
                namespace: "default".to_string(),
                name: "test".to_string(),
            };
            let req = Request::Pod(req_data);

            let is_terminated = Arc::new(AtomicBool::new(true));
            let worker = NetworkDescriptionWorker::new(is_terminated, tx, client, req);

            let handle = tokio::spawn(async move {
                worker
                    .fetch_description::<PodDescriptionWorker<MockTestKubeClient>>()
                    .await
            });

            let ret = handle.await.unwrap();

            assert!(ret.is_ok())
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn 内部でエラーがでたときループを抜けてerrを返す() {
            let (tx, _rx): (Sender<Event>, Receiver<Event>) = bounded(3);
            let mut client = MockTestKubeClient::new();
            client
                .expect_request::<FetchedPod>()
                .with(eq("api/v1/namespaces/default/pods/test"))
                .returning(|_| Err(anyhow!("error")));
            client
                .expect_request::<FetchedServiceList>()
                .with(eq("api/v1/namespaces/default/services"))
                .returning(|_| Err(anyhow!("error")));
            client
                .expect_request::<FetchedIngressList>()
                .with(eq("apis/networking.k8s.io/v1/namespaces/default/ingresses"))
                .returning(|_| Err(anyhow!("error")));

            let req_data = RequestData {
                namespace: "default".to_string(),
                name: "test".to_string(),
            };
            let req = Request::Pod(req_data);

            let is_terminated = Arc::new(AtomicBool::new(false));
            let worker = NetworkDescriptionWorker::new(is_terminated.clone(), tx, client, req);

            let handle = tokio::spawn(async move {
                worker
                    .fetch_description::<PodDescriptionWorker<MockTestKubeClient>>()
                    .await
            });

            let ret = handle.await.unwrap();

            assert_eq!(ret.is_err(), true)
        }
    }
}
