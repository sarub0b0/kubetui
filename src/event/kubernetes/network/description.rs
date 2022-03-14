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
        use super::*;

        use crossbeam::channel::{bounded, Receiver};
        use indoc::indoc;
        use mockall::predicate::eq;
        use pretty_assertions::assert_eq;

        use crate::event::kubernetes::client::mock::MockTestKubeClient;

        #[tokio::test(flavor = "multi_thread")]
        async fn 正常系のときtxにデータを送信する() {
            let is_terminated = Arc::new(AtomicBool::new(false));
            let (tx, rx): (Sender<Event>, Receiver<Event>) = bounded(3);
            let mut client = MockTestKubeClient::new();

            client
                .expect_request_text()
                .with(eq("api/v1/namespaces/default/pods/test"))
                .returning(|_| Ok(String::from(POD_JSON)));
            client
                .expect_request_text()
                .with(eq("api/v1/namespaces/default/services"))
                .returning(|_| Ok(String::from(SERVICE_JSON)));
            client
                .expect_request_text()
                .with(eq("apis/networking.k8s.io/v1/namespaces/default/ingresses"))
                .returning(|_| Ok(String::from(INGRESS_JSON)));

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

            assert!(matches!(
                event,
                Event::Kube(Kube::Network(NetworkMessage::Response(Ok(data))))
                    if data == expected
            ))
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

        #[ignore]
        #[test]
        fn 内部でエラーがでたときループを抜けてerrを返す() {
            unimplemented!()
        }

        const POD_JSON: &str = "{\"apiVersion\":\"v1\",\"kind\":\"Pod\",\"metadata\":{\"creationTimestamp\":\"2022-03-09T08:16:50Z\",\"generateName\":\"kubetui-text-color--1-\",\"labels\":{\"controller-uid\":\"30d417a8-cb1c-467b-92fe-7819601a6ef8\",\"job-name\":\"kubetui-text-color\"},\"name\":\"kubetui-text-color--1-g8gcl\",\"namespace\":\"kubetui\",\"ownerReferences\":[{\"apiVersion\":\"batch/v1\",\"blockOwnerDeletion\":true,\"controller\":true,\"kind\":\"Job\",\"name\":\"kubetui-text-color\",\"uid\":\"30d417a8-cb1c-467b-92fe-7819601a6ef8\"}],\"resourceVersion\":\"4858\",\"uid\":\"36041cab-e840-42dd-aa2c-417134fcfa47\"},\"spec\":{\"containers\":[{\"command\":[\"/opt/bin/color.sh\"],\"image\":\"alpine\",\"imagePullPolicy\":\"Always\",\"name\":\"job\",\"resources\":{},\"terminationMessagePath\":\"/dev/termination-log\",\"terminationMessagePolicy\":\"File\",\"volumeMounts\":[{\"mountPath\":\"/opt/bin/color.sh\",\"name\":\"script\",\"subPath\":\"color.sh\"},{\"mountPath\":\"/var/run/secrets/kubernetes.io/serviceaccount\",\"name\":\"kube-api-access-p4d2h\",\"readOnly\":true}]}],\"dnsPolicy\":\"ClusterFirst\",\"enableServiceLinks\":true,\"nodeName\":\"docker-desktop\",\"preemptionPolicy\":\"PreemptLowerPriority\",\"priority\":0,\"restartPolicy\":\"Never\",\"schedulerName\":\"default-scheduler\",\"securityContext\":{},\"serviceAccount\":\"default\",\"serviceAccountName\":\"default\",\"terminationGracePeriodSeconds\":30,\"tolerations\":[{\"effect\":\"NoExecute\",\"key\":\"node.kubernetes.io/not-ready\",\"operator\":\"Exists\",\"tolerationSeconds\":300},{\"effect\":\"NoExecute\",\"key\":\"node.kubernetes.io/unreachable\",\"operator\":\"Exists\",\"tolerationSeconds\":300}],\"volumes\":[{\"configMap\":{\"defaultMode\":493,\"name\":\"24bit-color-script\"},\"name\":\"script\"},{\"name\":\"kube-api-access-p4d2h\",\"projected\":{\"defaultMode\":420,\"sources\":[{\"serviceAccountToken\":{\"expirationSeconds\":3607,\"path\":\"token\"}},{\"configMap\":{\"items\":[{\"key\":\"ca.crt\",\"path\":\"ca.crt\"}],\"name\":\"kube-root-ca.crt\"}},{\"downwardAPI\":{\"items\":[{\"fieldRef\":{\"apiVersion\":\"v1\",\"fieldPath\":\"metadata.namespace\"},\"path\":\"namespace\"}]}}]}}]},\"status\":{\"conditions\":[{\"lastProbeTime\":null,\"lastTransitionTime\":\"2022-03-09T08:16:50Z\",\"reason\":\"PodCompleted\",\"status\":\"True\",\"type\":\"Initialized\"},{\"lastProbeTime\":null,\"lastTransitionTime\":\"2022-03-09T08:16:50Z\",\"reason\":\"PodCompleted\",\"status\":\"False\",\"type\":\"Ready\"},{\"lastProbeTime\":null,\"lastTransitionTime\":\"2022-03-09T08:16:50Z\",\"reason\":\"PodCompleted\",\"status\":\"False\",\"type\":\"ContainersReady\"},{\"lastProbeTime\":null,\"lastTransitionTime\":\"2022-03-09T08:16:50Z\",\"status\":\"True\",\"type\":\"PodScheduled\"}],\"containerStatuses\":[{\"containerID\":\"docker://e10aca86212a3c1c5c19bf4ba707dc9aa92a12428e12201932fa2985a572edec\",\"image\":\"alpine:latest\",\"imageID\":\"docker-pullable://alpine@sha256:21a3deaa0d32a8057914f36584b5288d2e5ecc984380bc0118285c70fa8c9300\",\"lastState\":{},\"name\":\"job\",\"ready\":false,\"restartCount\":0,\"started\":false,\"state\":{\"terminated\":{\"containerID\":\"docker://e10aca86212a3c1c5c19bf4ba707dc9aa92a12428e12201932fa2985a572edec\",\"exitCode\":0,\"finishedAt\":\"2022-03-09T08:17:39Z\",\"reason\":\"Completed\",\"startedAt\":\"2022-03-09T08:17:39Z\"}}}],\"hostIP\":\"192.168.65.4\",\"phase\":\"Succeeded\",\"podIP\":\"10.1.0.21\",\"podIPs\":[{\"ip\":\"10.1.0.21\"}],\"qosClass\":\"BestEffort\",\"startTime\":\"2022-03-09T08:16:50Z\"}}" ;

        const SERVICE_JSON: &str= "{\"kind\":\"ServiceList\",\"apiVersion\":\"v1\",\"metadata\":{\"resourceVersion\":\"254708\"},\"items\":[{\"metadata\":{\"name\":\"kubetui-running\",\"namespace\":\"kubetui\",\"uid\":\"dc175803-fc06-4b20-a150-2b68c962a2c4\",\"resourceVersion\":\"4627\",\"creationTimestamp\":\"2022-03-09T08:16:50Z\",\"annotations\":{\"kubectl.kubernetes.io/last-applied-configuration\":\"{\\\"apiVersion\\\":\\\"v1\\\",\\\"kind\\\":\\\"Service\\\",\\\"metadata\\\":{\\\"annotations\\\":{},\\\"name\\\":\\\"kubetui-running\\\",\\\"namespace\\\":\\\"kubetui\\\"},\\\"spec\\\":{\\\"ports\\\":[{\\\"port\\\":80,\\\"targetPort\\\":80}],\\\"selector\\\":{\\\"app\\\":\\\"kubetui-running\\\"}}}\\n\"},\"managedFields\":[{\"manager\":\"kubectl-client-side-apply\",\"operation\":\"Update\",\"apiVersion\":\"v1\",\"time\":\"2022-03-09T08:16:50Z\",\"fieldsType\":\"FieldsV1\",\"fieldsV1\":{\"f:metadata\":{\"f:annotations\":{\".\":{},\"f:kubectl.kubernetes.io/last-applied-configuration\":{}}},\"f:spec\":{\"f:internalTrafficPolicy\":{},\"f:ports\":{\".\":{},\"k:{\\\"port\\\":80,\\\"protocol\\\":\\\"TCP\\\"}\":{\".\":{},\"f:port\":{},\"f:protocol\":{},\"f:targetPort\":{}}},\"f:selector\":{},\"f:sessionAffinity\":{},\"f:type\":{}}}}]},\"spec\":{\"ports\":[{\"protocol\":\"TCP\",\"port\":80,\"targetPort\":80}],\"selector\":{\"app\":\"kubetui-running\"},\"clusterIP\":\"10.109.244.57\",\"clusterIPs\":[\"10.109.244.57\"],\"type\":\"ClusterIP\",\"sessionAffinity\":\"None\",\"ipFamilies\":[\"IPv4\"],\"ipFamilyPolicy\":\"SingleStack\",\"internalTrafficPolicy\":\"Cluster\"},\"status\":{\"loadBalancer\":{}}},{\"metadata\":{\"name\":\"service-0\",\"namespace\":\"kubetui\",\"uid\":\"c3e392e4-bb72-4a92-8814-109f89c951dd\",\"resourceVersion\":\"4554\",\"creationTimestamp\":\"2022-03-09T08:16:50Z\",\"annotations\":{\"kubectl.kubernetes.io/last-applied-configuration\":\"{\\\"apiVersion\\\":\\\"v1\\\",\\\"kind\\\":\\\"Service\\\",\\\"metadata\\\":{\\\"annotations\\\":{},\\\"name\\\":\\\"service-0\\\",\\\"namespace\\\":\\\"kubetui\\\"},\\\"spec\\\":{\\\"ports\\\":[{\\\"port\\\":80,\\\"targetPort\\\":80}],\\\"selector\\\":{\\\"app\\\":\\\"app\\\"}}}\\n\"},\"managedFields\":[{\"manager\":\"kubectl-client-side-apply\",\"operation\":\"Update\",\"apiVersion\":\"v1\",\"time\":\"2022-03-09T08:16:50Z\",\"fieldsType\":\"FieldsV1\",\"fieldsV1\":{\"f:metadata\":{\"f:annotations\":{\".\":{},\"f:kubectl.kubernetes.io/last-applied-configuration\":{}}},\"f:spec\":{\"f:internalTrafficPolicy\":{},\"f:ports\":{\".\":{},\"k:{\\\"port\\\":80,\\\"protocol\\\":\\\"TCP\\\"}\":{\".\":{},\"f:port\":{},\"f:protocol\":{},\"f:targetPort\":{}}},\"f:selector\":{},\"f:sessionAffinity\":{},\"f:type\":{}}}}]},\"spec\":{\"ports\":[{\"protocol\":\"TCP\",\"port\":80,\"targetPort\":80}],\"selector\":{\"app\":\"app\"},\"clusterIP\":\"10.102.47.75\",\"clusterIPs\":[\"10.102.47.75\"],\"type\":\"ClusterIP\",\"sessionAffinity\":\"None\",\"ipFamilies\":[\"IPv4\"],\"ipFamilyPolicy\":\"SingleStack\",\"internalTrafficPolicy\":\"Cluster\"},\"status\":{\"loadBalancer\":{}}},{\"metadata\":{\"name\":\"service-1\",\"namespace\":\"kubetui\",\"uid\":\"78c65502-8600-4a6e-a55a-a5420bd42609\",\"resourceVersion\":\"4557\",\"creationTimestamp\":\"2022-03-09T08:16:50Z\",\"annotations\":{\"kubectl.kubernetes.io/last-applied-configuration\":\"{\\\"apiVersion\\\":\\\"v1\\\",\\\"kind\\\":\\\"Service\\\",\\\"metadata\\\":{\\\"annotations\\\":{},\\\"name\\\":\\\"service-1\\\",\\\"namespace\\\":\\\"kubetui\\\"},\\\"spec\\\":{\\\"ports\\\":[{\\\"port\\\":80,\\\"targetPort\\\":80}],\\\"selector\\\":{\\\"app\\\":\\\"app\\\"}}}\\n\"},\"managedFields\":[{\"manager\":\"kubectl-client-side-apply\",\"operation\":\"Update\",\"apiVersion\":\"v1\",\"time\":\"2022-03-09T08:16:50Z\",\"fieldsType\":\"FieldsV1\",\"fieldsV1\":{\"f:metadata\":{\"f:annotations\":{\".\":{},\"f:kubectl.kubernetes.io/last-applied-configuration\":{}}},\"f:spec\":{\"f:internalTrafficPolicy\":{},\"f:ports\":{\".\":{},\"k:{\\\"port\\\":80,\\\"protocol\\\":\\\"TCP\\\"}\":{\".\":{},\"f:port\":{},\"f:protocol\":{},\"f:targetPort\":{}}},\"f:selector\":{},\"f:sessionAffinity\":{},\"f:type\":{}}}}]},\"spec\":{\"ports\":[{\"protocol\":\"TCP\",\"port\":80,\"targetPort\":80}],\"selector\":{\"app\":\"app\"},\"clusterIP\":\"10.96.217.254\",\"clusterIPs\":[\"10.96.217.254\"],\"type\":\"ClusterIP\",\"sessionAffinity\":\"None\",\"ipFamilies\":[\"IPv4\"],\"ipFamilyPolicy\":\"SingleStack\",\"internalTrafficPolicy\":\"Cluster\"},\"status\":{\"loadBalancer\":{}}}]}";

        const INGRESS_JSON: &str = "{\"kind\":\"IngressList\",\"apiVersion\":\"networking.k8s.io/v1\",\"metadata\":{\"resourceVersion\":\"254764\"},\"items\":[{\"metadata\":{\"name\":\"ingress\",\"namespace\":\"kubetui\",\"uid\":\"649efd06-afe2-4783-a8ae-96778f5c5e23\",\"resourceVersion\":\"4549\",\"generation\":1,\"creationTimestamp\":\"2022-03-09T08:16:50Z\",\"annotations\":{\"kubectl.kubernetes.io/last-applied-configuration\":\"{\\\"apiVersion\\\":\\\"networking.k8s.io/v1\\\",\\\"kind\\\":\\\"Ingress\\\",\\\"metadata\\\":{\\\"annotations\\\":{},\\\"name\\\":\\\"ingress\\\",\\\"namespace\\\":\\\"kubetui\\\"},\\\"spec\\\":{\\\"rules\\\":[{\\\"host\\\":\\\"example-0.com\\\",\\\"http\\\":{\\\"paths\\\":[{\\\"backend\\\":{\\\"service\\\":{\\\"name\\\":\\\"service-0\\\",\\\"port\\\":{\\\"number\\\":80}}},\\\"path\\\":\\\"/path\\\",\\\"pathType\\\":\\\"ImplementationSpecific\\\"}]}},{\\\"host\\\":\\\"example-1.com\\\",\\\"http\\\":{\\\"paths\\\":[{\\\"backend\\\":{\\\"service\\\":{\\\"name\\\":\\\"service-1\\\",\\\"port\\\":{\\\"number\\\":80}}},\\\"path\\\":\\\"/path\\\",\\\"pathType\\\":\\\"ImplementationSpecific\\\"}]}}],\\\"tls\\\":[{\\\"hosts\\\":[\\\"example.com\\\"],\\\"secretName\\\":\\\"secret-name\\\"}]}}\\n\"},\"managedFields\":[{\"manager\":\"kubectl-client-side-apply\",\"operation\":\"Update\",\"apiVersion\":\"networking.k8s.io/v1\",\"time\":\"2022-03-09T08:16:50Z\",\"fieldsType\":\"FieldsV1\",\"fieldsV1\":{\"f:metadata\":{\"f:annotations\":{\".\":{},\"f:kubectl.kubernetes.io/last-applied-configuration\":{}}},\"f:spec\":{\"f:rules\":{},\"f:tls\":{}}}}]},\"spec\":{\"tls\":[{\"hosts\":[\"example.com\"],\"secretName\":\"secret-name\"}],\"rules\":[{\"host\":\"example-0.com\",\"http\":{\"paths\":[{\"path\":\"/path\",\"pathType\":\"ImplementationSpecific\",\"backend\":{\"service\":{\"name\":\"service-0\",\"port\":{\"number\":80}}}}]}},{\"host\":\"example-1.com\",\"http\":{\"paths\":[{\"path\":\"/path\",\"pathType\":\"ImplementationSpecific\",\"backend\":{\"service\":{\"name\":\"service-1\",\"port\":{\"number\":80}}}}]}}]},\"status\":{\"loadBalancer\":{}}}]}";
    }
}
