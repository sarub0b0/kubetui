mod gateway;
mod httproute;
mod ingress;
mod network_policy;
mod pod;
mod service;
mod utils;

#[allow(dead_code)]
mod related_resources;

use crate::{
    features::{
        api_resources::kube::SharedApiResources,
        network::message::{NetworkRequest, NetworkRequestTargetParams, NetworkResponse},
    },
    kube::KubeClientRequest,
    logger,
    message::Message,
    workers::kube::AbortWorker,
};

use self::{
    gateway::GatewayDescriptionWorker, httproute::HTTPRouteDescriptionWorker,
    ingress::IngressDescriptionWorker, network_policy::NetworkPolicyDescriptionWorker,
    pod::PodDescriptionWorker, service::ServiceDescriptionWorker,
};

use anyhow::Result;
use async_trait::async_trait;
use crossbeam::channel::Sender;

const INTERVAL: u64 = 3;

type FetchedData = Vec<String>;

#[async_trait]
trait Fetch<'a, C: KubeClientRequest> {
    // TODO: 将来このメソッドはtraitから削除する
    fn new(
        client: &'a C,
        params: NetworkRequestTargetParams,
        api_resources: SharedApiResources,
    ) -> Self;

    async fn fetch(&self) -> Result<FetchedData>;
}

#[derive(Clone)]
pub struct NetworkDescriptionWorker<C>
where
    C: KubeClientRequest,
{
    tx: Sender<Message>,
    client: C,
    req: NetworkRequest,
    api_resources: SharedApiResources,
}

impl<C> NetworkDescriptionWorker<C>
where
    C: KubeClientRequest,
{
    pub fn new(
        tx: Sender<Message>,
        client: C,
        req: NetworkRequest,
        api_resources: SharedApiResources,
    ) -> Self {
        Self {
            tx,
            client,
            req,
            api_resources,
        }
    }
}

#[async_trait]
impl<C> AbortWorker for NetworkDescriptionWorker<C>
where
    C: KubeClientRequest,
{
    async fn run(&self) {
        let ret = match &self.req {
            NetworkRequest::Pod(_) => self.fetch_description::<PodDescriptionWorker<C>>().await,
            NetworkRequest::Service(_) => {
                self.fetch_description::<ServiceDescriptionWorker<C>>()
                    .await
            }
            NetworkRequest::Ingress(_) => {
                self.fetch_description::<IngressDescriptionWorker<C>>()
                    .await
            }
            NetworkRequest::NetworkPolicy(_) => {
                self.fetch_description::<NetworkPolicyDescriptionWorker<C>>()
                    .await
            }
            NetworkRequest::Gateway(_) => {
                self.fetch_description::<GatewayDescriptionWorker<C>>()
                    .await
            }
            NetworkRequest::HTTPRoute(_) => {
                self.fetch_description::<HTTPRouteDescriptionWorker<C>>()
                    .await
            }
        };

        if let Err(e) = ret {
            logger!(error, "Failed to fetch description: {:?}", e);

            self.tx
                .send(NetworkResponse::Yaml(Err(e)).into())
                .expect("Failed to send NetworkResponse::Yaml");
        }
    }
}

impl<C> NetworkDescriptionWorker<C>
where
    C: KubeClientRequest,
{
    async fn fetch_description<'a, Worker>(&'a self) -> Result<()>
    where
        Worker: Fetch<'a, C>,
    {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(INTERVAL));

        let worker = Worker::new(
            &self.client,
            self.req.data().to_owned(),
            self.api_resources.clone(),
        );

        loop {
            interval.tick().await;

            let fetched_data = worker.fetch().await;

            self.tx.send(NetworkResponse::Yaml(fetched_data).into())?;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use indoc::indoc;
    use k8s_openapi::api::core::v1::Pod;

    fn pod() -> Pod {
        let yaml = indoc! {
            "
            metadata:
              name: test
              namespace: default
              labels:
                controller-uid: 30d417a8-cb1c-467b-92fe-7819601a6ef8
                job-name: kubetui-text-color
            spec:
              containers:
              - name: job
                image: alpine
            status:
              phase: Succeeded
              hostIP: 192.168.65.4
              podIP: 10.1.0.21
              podIPs:
              - ip: 10.1.0.21
            "
        };

        serde_yaml::from_str(yaml).unwrap()
    }

    mod run {

        use crate::{
            features::{api_resources::kube::ApiResources, network::message::NetworkMessage},
            kube::mock::MockTestKubeClient,
            mock_expect,
            workers::kube::message::Kube,
        };

        use super::*;
        use anyhow::bail;
        use crossbeam::channel::{bounded, Receiver};
        use k8s_openapi::{
            api::{core::v1::Service, networking::v1::Ingress},
            List,
        };
        use mockall::predicate::eq;

        #[tokio::test(flavor = "multi_thread")]
        async fn 内部でエラーがでたときtxにerrを送信する() {
            let (tx, rx): (Sender<Message>, Receiver<Message>) = bounded(3);

            let mut client = MockTestKubeClient::new();
            mock_expect!(
                client,
                request,
                [
                    (
                        Pod,
                        eq("/api/v1/namespaces/default/pods/test"),
                        bail!("error")
                    ),
                    (
                        List<Service>,
                        eq("/api/v1/namespaces/default/services"),
                        bail!("error")
                    ),
                    (
                        List<Ingress>,
                        eq("/apis/networking.k8s.io/v1/namespaces/default/ingresses"),
                        bail!("error")
                    )
                ]
            );

            let req_data = NetworkRequestTargetParams {
                namespace: "default".to_string(),
                name: "test".to_string(),
                version: "v1".to_string(),
            };
            let req = NetworkRequest::Pod(req_data);

            let worker = NetworkDescriptionWorker::new(tx, client, req, ApiResources::shared());

            tokio::spawn(async move { worker.run().await });

            if let Message::Kube(Kube::Network(NetworkMessage::Response(NetworkResponse::Yaml(
                msg,
            )))) = rx.recv().unwrap()
            {
                assert!(msg.is_err())
            } else {
                unreachable!()
            }
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn txがエラーのときエラーを返す() {
            let (tx, rx): (Sender<Message>, Receiver<Message>) = bounded(0);
            let mut client = MockTestKubeClient::new();

            mock_expect!(
                client,
                request,
                [
                    (
                        Pod,
                        eq("/api/v1/namespaces/default/pods/test"),
                        bail!("error")
                    ),
                    (
                        List<Service>,
                        eq("/api/v1/namespaces/default/services"),
                        bail!("error")
                    ),
                    (
                        List<Ingress>,
                        eq("/apis/networking.k8s.io/v1/namespaces/default/ingresses"),
                        bail!("error")
                    )
                ]
            );

            let req_data = NetworkRequestTargetParams {
                namespace: "default".to_string(),
                name: "test".to_string(),
                version: "v1".to_string(),
            };
            let req = NetworkRequest::Pod(req_data);

            let worker = NetworkDescriptionWorker::new(tx, client, req, ApiResources::shared());

            drop(rx);

            let ret = tokio::spawn(async move { worker.run().await }).await;

            assert!(ret.is_err())
        }
    }

    mod fetch_description {

        use super::*;

        use anyhow::bail;
        use crossbeam::channel::{bounded, Receiver};
        use indoc::indoc;
        use k8s_openapi::{
            api::{
                core::v1::Service,
                networking::v1::{Ingress, NetworkPolicy},
            },
            List,
        };
        use mockall::predicate::eq;
        use pretty_assertions::assert_eq;

        use crate::{
            features::{api_resources::kube::ApiResources, network::message::NetworkMessage},
            kube::mock::MockTestKubeClient,
            mock_expect,
            workers::kube::message::Kube,
        };

        #[tokio::test(flavor = "multi_thread")]
        async fn 正常系のときtxにデータを送信する() {
            let (tx, rx): (Sender<Message>, Receiver<Message>) = bounded(3);
            let mut client = MockTestKubeClient::new();

            mock_expect!(
                client,
                request,
                [
                    (
                        Pod,
                        eq("/api/v1/namespaces/default/pods/test"),
                        Ok(pod())
                    ),
                    (
                        List<Service>,
                        eq("/api/v1/namespaces/default/services"),
                        Ok(Default::default())
                    ),
                    (
                        List<Ingress>,
                        eq("/apis/networking.k8s.io/v1/namespaces/default/ingresses"),
                        Ok(Default::default())
                    ),
                    (
                        List<NetworkPolicy>,
                        eq("/apis/networking.k8s.io/v1/namespaces/default/networkpolicies"),
                        Ok(Default::default())
                    )
                ]
            );

            let req_data = NetworkRequestTargetParams {
                namespace: "default".to_string(),
                name: "test".to_string(),
                version: "v1".to_string(),
            };
            let req = NetworkRequest::Pod(req_data);

            let worker = NetworkDescriptionWorker::new(tx, client, req, ApiResources::shared());

            tokio::spawn(async move {
                worker
                    .fetch_description::<PodDescriptionWorker<MockTestKubeClient>>()
                    .await
            });

            let event = rx.recv().unwrap();

            let expected: Vec<String> = indoc!(
                "
                pod:
                  metadata:
                    labels:
                      controller-uid: 30d417a8-cb1c-467b-92fe-7819601a6ef8
                      job-name: kubetui-text-color
                    name: test
                  spec:
                    containers:
                    - image: alpine
                      name: job
                  status:
                    hostIP: 192.168.65.4
                    phase: Succeeded
                    podIP: 10.1.0.21
                    podIPs:
                    - ip: 10.1.0.21
                "
            )
            .lines()
            .map(ToString::to_string)
            .collect();

            if let Message::Kube(Kube::Network(NetworkMessage::Response(NetworkResponse::Yaml(
                Ok(actual),
            )))) = event
            {
                assert_eq!(actual, expected)
            } else {
                unreachable!()
            }
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn 内部でエラーがでたときtxにerrを送信する() {
            let (tx, rx): (Sender<Message>, Receiver<Message>) = bounded(3);
            let mut client = MockTestKubeClient::new();
            mock_expect!(
                client,
                request,
                [
                    (
                        Pod,
                        eq("/api/v1/namespaces/default/pods/test"),
                        bail!("error")
                    ),
                    (
                        List<Service>,
                        eq("/api/v1/namespaces/default/services"),
                        bail!("error")
                    ),
                    (
                        List<Ingress>,
                        eq("/apis/networking.k8s.io/v1/namespaces/default/ingresses"),
                        bail!("error")
                    ),
                    (
                        List<NetworkPolicy>,
                        eq("/apis/networking.k8s.io/v1/namespaces/default/networkpolicies"),
                        bail!("error")
                    )
                ]
            );

            let req_data = NetworkRequestTargetParams {
                namespace: "default".to_string(),
                name: "test".to_string(),
                version: "v1".to_string(),
            };
            let req = NetworkRequest::Pod(req_data);

            let worker = NetworkDescriptionWorker::new(tx, client, req, ApiResources::shared());

            tokio::spawn(async move {
                worker
                    .fetch_description::<PodDescriptionWorker<MockTestKubeClient>>()
                    .await
            });

            if let Message::Kube(Kube::Network(NetworkMessage::Response(NetworkResponse::Yaml(
                msg,
            )))) = rx.recv().unwrap()
            {
                assert!(msg.is_err())
            } else {
                unreachable!()
            }
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn txがエラーのときエラーを返す() {
            let (tx, rx): (Sender<Message>, Receiver<Message>) = bounded(0);
            let mut client = MockTestKubeClient::new();

            mock_expect!(
                client,
                request,
                [
                    (
                        Pod,
                        eq("/api/v1/namespaces/default/pods/test"),
                        bail!("error")
                    ),
                    (
                        List<Service>,
                        eq("/api/v1/namespaces/default/services"),
                        bail!("error")
                    ),
                    (
                        List<Ingress>,
                        eq("/apis/networking.k8s.io/v1/namespaces/default/ingresses"),
                        bail!("error")
                    )
                ]
            );

            let req_data = NetworkRequestTargetParams {
                namespace: "default".to_string(),
                name: "test".to_string(),
                version: "v1".to_string(),
            };
            let req = NetworkRequest::Pod(req_data);

            let worker = NetworkDescriptionWorker::new(tx, client, req, ApiResources::shared());

            drop(rx);

            let ret = worker
                .fetch_description::<PodDescriptionWorker<MockTestKubeClient>>()
                .await;

            assert!(ret.is_err())
        }
    }
}
