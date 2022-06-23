use k8s_openapi::{
    api::{
        core::v1::{Pod, Service},
        networking::v1::{Ingress, NetworkPolicy},
    },
    List,
};
use kube::{Resource, ResourceExt};
use serde_yaml::Mapping;

use self::{extract::Extract, to_value::ToValue};

use super::{
    related_resources::{to_list_value::ToListValue, RelatedClient},
    Fetch, FetchedData, Result,
};

use crate::event::kubernetes::client::KubeClientRequest;

pub(super) struct PodDescriptionWorker<'a, C>
where
    C: KubeClientRequest,
{
    client: &'a C,
    namespace: String,
    name: String,
}

#[async_trait::async_trait]
impl<'a, C: KubeClientRequest> Fetch<'a, C> for PodDescriptionWorker<'a, C> {
    fn new(client: &'a C, namespace: String, name: String) -> Self {
        PodDescriptionWorker {
            client,
            namespace,
            name,
        }
    }

    async fn fetch(&self) -> Result<FetchedData> {
        let url = format!(
            "{}/{}",
            Pod::url_path(&(), Some(&self.namespace)),
            self.name
        );

        let pod: Pod = self.client.request(&url).await?;
        let pod = pod.extract();

        let related_services = RelatedClient::new(self.client, &self.namespace)
            .related_resources::<Service, _>(pod.labels())
            .await?;

        let related_ingresses: Option<List<Ingress>> = if let Some(services) = &related_services {
            let services = services.items.iter().map(|svc| svc.name()).collect();

            RelatedClient::new(self.client, &self.namespace)
                .related_resources::<Ingress, _>(&services)
                .await?
        } else {
            None
        };

        let related_networkpolicies: Option<List<NetworkPolicy>> =
            if let Some(labels) = &pod.metadata.labels {
                RelatedClient::new(self.client, &self.namespace)
                    .related_resources(labels)
                    .await?
            } else {
                None
            };

        let pod: Vec<String> = serde_yaml::to_string(&pod.to_value()?)?
            .lines()
            .skip(1)
            .map(ToString::to_string)
            .collect();

        let mut value = pod;

        let mut related_resources = Mapping::new();

        if let Some(services) = related_services {
            if let Some(value) = services.to_list_value() {
                related_resources.insert("services".into(), value);
            }
        }

        if let Some(ingresses) = related_ingresses {
            if let Some(value) = ingresses.to_list_value() {
                related_resources.insert("ingresses".into(), value);
            }
        }

        if let Some(networkpolicies) = related_networkpolicies {
            if let Some(value) = networkpolicies.to_list_value() {
                related_resources.insert("networkpolicies".into(), value);
            }
        }

        if !related_resources.is_empty() {
            let mut root = Mapping::new();

            root.insert("relatedResources".into(), related_resources.into());

            let resources = serde_yaml::to_string(&root)?;
            let vec: Vec<String> = resources.lines().skip(1).map(ToString::to_string).collect();

            value.push(String::default());
            value.extend(vec);
        }

        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{event::kubernetes::client::mock::MockTestKubeClient, mock_expect};
    use indoc::indoc;
    use k8s_openapi::{
        api::{
            core::v1::{Pod, Service},
            networking::v1::{Ingress, NetworkPolicy},
        },
        List,
    };
    use mockall::predicate::eq;

    use pretty_assertions::assert_eq;

    use anyhow::bail;

    fn pod() -> Pod {
        serde_yaml::from_str(indoc! {
            r#"
            apiVersion: v1
            kind: Pod
            metadata:
              creationTimestamp: "2022-04-04T03:05:46Z"
              generateName: test-5d69d5ddc6-
              labels:
                app: pod-1
                version: v1
              name: test
              namespace: kubetui
              ownerReferences:
                - apiVersion: apps/v1
                  blockOwnerDeletion: true
                  controller: true
                  kind: ReplicaSet
                  name: test-5d69d5ddc6
                  uid: f9be5c32-b4a5-4ec9-b8e8-53c240f4e255
              resourceVersion: "367972"
              uid: 7a1ffede-c201-4438-893b-f81dc5ded89e
            spec:
              containers:
                - args:
                    - while true; do echo app-0; sleep 1; done
                  command:
                    - sh
                    - -c
                  image: nginx
                  imagePullPolicy: Always
                  name: app-0
                  resources: {}
                  terminationMessagePath: /dev/termination-log
                  terminationMessagePolicy: File
                  volumeMounts:
                    - mountPath: /var/run/secrets/kubernetes.io/serviceaccount
                      name: kube-api-access-jdfbz
                      readOnly: true
                - args:
                    - while true; do echo app-1; sleep 1; done
                  command:
                    - sh
                    - -c
                  image: alpine
                  imagePullPolicy: Always
                  name: app-1
                  resources: {}
                  terminationMessagePath: /dev/termination-log
                  terminationMessagePolicy: File
                  volumeMounts:
                    - mountPath: /var/run/secrets/kubernetes.io/serviceaccount
                      name: kube-api-access-jdfbz
                      readOnly: true
                - image: nginx
                  imagePullPolicy: Always
                  name: web
                  ports:
                    - containerPort: 80
                      name: http
                      protocol: TCP
                  resources: {}
                  terminationMessagePath: /dev/termination-log
                  terminationMessagePolicy: File
                  volumeMounts:
                    - mountPath: /var/run/secrets/kubernetes.io/serviceaccount
                      name: kube-api-access-jdfbz
                      readOnly: true
              dnsPolicy: ClusterFirst
              enableServiceLinks: true
              initContainers:
                - args:
                    - echo init-0; exit 0
                  command:
                    - sh
                    - -c
                  image: alpine
                  imagePullPolicy: Always
                  name: init-0
                  resources: {}
                  terminationMessagePath: /dev/termination-log
                  terminationMessagePolicy: File
                  volumeMounts:
                    - mountPath: /var/run/secrets/kubernetes.io/serviceaccount
                      name: kube-api-access-jdfbz
                      readOnly: true
                - args:
                    - echo init-1; exit 0
                  command:
                    - sh
                    - -c
                  image: alpine
                  imagePullPolicy: Always
                  name: init-1
                  resources: {}
                  terminationMessagePath: /dev/termination-log
                  terminationMessagePolicy: File
                  volumeMounts:
                    - mountPath: /var/run/secrets/kubernetes.io/serviceaccount
                      name: kube-api-access-jdfbz
                      readOnly: true
              nodeName: docker-desktop
              preemptionPolicy: PreemptLowerPriority
              priority: 0
              restartPolicy: Always
              schedulerName: default-scheduler
              securityContext: {}
              serviceAccount: default
              serviceAccountName: default
              terminationGracePeriodSeconds: 30
              tolerations:
                - effect: NoExecute
                  key: node.kubernetes.io/not-ready
                  operator: Exists
                  tolerationSeconds: 300
                - effect: NoExecute
                  key: node.kubernetes.io/unreachable
                  operator: Exists
                  tolerationSeconds: 300
              volumes:
                - name: kube-api-access-jdfbz
                  projected:
                    defaultMode: 420
                    sources:
                      - serviceAccountToken:
                          expirationSeconds: 3607
                          path: token
                      - configMap:
                          items:
                            - key: ca.crt
                              path: ca.crt
                          name: kube-root-ca.crt
                      - downwardAPI:
                          items:
                            - fieldRef:
                                apiVersion: v1
                                fieldPath: metadata.namespace
                              path: namespace
            status:
              conditions:
                - lastProbeTime: null
                  lastTransitionTime: "2022-04-04T03:05:55Z"
                  status: "True"
                  type: Initialized
                - lastProbeTime: null
                  lastTransitionTime: "2022-04-04T03:06:03Z"
                  status: "True"
                  type: Ready
                - lastProbeTime: null
                  lastTransitionTime: "2022-04-04T03:06:03Z"
                  status: "True"
                  type: ContainersReady
                - lastProbeTime: null
                  lastTransitionTime: "2022-04-04T03:05:46Z"
                  status: "True"
                  type: PodScheduled
              containerStatuses:
                - containerID: docker://5851ea5a23c5983d846ae1e1c0b8ffa2e24340396d7620967177993c0880b0cf
                  image: nginx:latest
                  imageID: docker-pullable://nginx@sha256:2275af0f20d71b293916f1958f8497f987b8d8fd8113df54635f2a5915002bf1
                  lastState: {}
                  name: app-0
                  ready: true
                  restartCount: 0
                  started: true
                  state:
                    running:
                      startedAt: "2022-04-04T03:05:57Z"
                - containerID: docker://a3896301901509b738877fab4ebfba123693e8966c7be5ef8cfeace059158cc4
                  image: alpine:latest
                  imageID: docker-pullable://alpine@sha256:f22945d45ee2eb4dd463ed5a431d9f04fcd80ca768bb1acf898d91ce51f7bf04
                  lastState: {}
                  name: app-1
                  ready: true
                  restartCount: 0
                  started: true
                  state:
                    running:
                      startedAt: "2022-04-04T03:06:00Z"
                - containerID: docker://a04620d3e5b2f230837fb745395f70af6289aee96a797e99c0f42f2d68571ae3
                  image: nginx:latest
                  imageID: docker-pullable://nginx@sha256:2275af0f20d71b293916f1958f8497f987b8d8fd8113df54635f2a5915002bf1
                  lastState: {}
                  name: web
                  ready: true
                  restartCount: 0
                  started: true
                  state:
                    running:
                      startedAt: "2022-04-04T03:06:02Z"
              hostIP: 192.168.65.4
              initContainerStatuses:
                - containerID: docker://9bffed2937e48ee8fe5e08447456e07b814308485acaaed541157e4e07bbd95d
                  image: alpine:latest
                  imageID: docker-pullable://alpine@sha256:f22945d45ee2eb4dd463ed5a431d9f04fcd80ca768bb1acf898d91ce51f7bf04
                  lastState: {}
                  name: init-0
                  ready: true
                  restartCount: 0
                  state:
                    terminated:
                      containerID: docker://9bffed2937e48ee8fe5e08447456e07b814308485acaaed541157e4e07bbd95d
                      exitCode: 0
                      finishedAt: "2022-04-04T03:05:49Z"
                      reason: Completed
                      startedAt: "2022-04-04T03:05:49Z"
                - containerID: docker://85aa30c49fe7cb8343c52bc3a100f6c226cfef18fad6d68d04cf02d53cda0ca1
                  image: alpine:latest
                  imageID: docker-pullable://alpine@sha256:f22945d45ee2eb4dd463ed5a431d9f04fcd80ca768bb1acf898d91ce51f7bf04
                  lastState: {}
                  name: init-1
                  ready: true
                  restartCount: 0
                  state:
                    terminated:
                      containerID: docker://85aa30c49fe7cb8343c52bc3a100f6c226cfef18fad6d68d04cf02d53cda0ca1
                      exitCode: 0
                      finishedAt: "2022-04-04T03:05:54Z"
                      reason: Completed
                      startedAt: "2022-04-04T03:05:54Z"
              phase: Running
              podIP: 10.1.0.212
              podIPs:
                - ip: 10.1.0.212
              qosClass: BestEffort
              startTime: "2022-04-04T03:05:46Z"
            "#
        })
        .unwrap()
    }

    fn services() -> List<Service> {
        let yaml = indoc! {
            "
            items:
              - metadata:
                  name: service-1
                spec:
                  selector:
                    app: pod-1
              - metadata:
                  name: service-2
                spec:
                  selector:
                    version: v1
            "
        };

        serde_yaml::from_str(yaml).unwrap()
    }

    fn ingresses() -> List<Ingress> {
        let yaml = indoc! {
            "
            items:
              - metadata:
                  name: ingress-1
                spec:
                  rules:
                    - http:
                        paths:
                          - backend:
                              service:
                                name: service-1
              - metadata:
                  name: ingress-2
                spec:
                  rules:
                    - http:
                        paths:
                          - backend:
                              service:
                                name: service-2
            "
        };

        serde_yaml::from_str(yaml).unwrap()
    }

    fn networkpolicies() -> List<NetworkPolicy> {
        serde_yaml::from_str(indoc! {
            r#"
            items:
              - apiVersion: networking.k8s.io/v1
                kind: NetworkPolicy
                metadata:
                  name: allow-all-egress
                spec:
                  egress:
                    - {}
                  podSelector: {}
                  policyTypes:
                    - Egress
              - apiVersion: networking.k8s.io/v1
                kind: NetworkPolicy
                metadata:
                  name: allow-all-ingress
                spec:
                  ingress:
                    - {}
                  podSelector: {}
                  policyTypes:
                    - Ingress
              - apiVersion: networking.k8s.io/v1
                kind: NetworkPolicy
                metadata:
                  name: test
                spec:
                  ingress:
                    - {}
                  podSelector:
                    matchLabels:
                      foo: bar
                  policyTypes:
                    - Ingress
            "#
        })
        .unwrap()
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn yamlデータを送信してokを返す() {
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
                        Ok(services())
                    ),
                    (
                        List<Ingress>,
                        eq("/apis/networking.k8s.io/v1/namespaces/default/ingresses"),
                        Ok(ingresses())
                    ),
                    (
                        List<NetworkPolicy>,
                        eq("/apis/networking.k8s.io/v1/namespaces/default/networkpolicies"),
                        Ok(networkpolicies())
                    )
                ]
            );

        let worker = PodDescriptionWorker::new(&client, "default".to_string(), "test".to_string());

        let result = worker.fetch().await;

        let expected: Vec<String> = indoc! {
            "
            pod:
              metadata:
                labels:
                  app: pod-1
                  version: v1
                name: test
              spec:
                containers:
                  - image: nginx
                    name: app-0
                  - image: alpine
                    name: app-1
                  - image: nginx
                    name: web
                    ports:
                      - containerPort: 80
                        name: http
                        protocol: TCP
                dnsPolicy: ClusterFirst
                enableServiceLinks: true
                initContainers:
                  - image: alpine
                    name: init-0
                  - image: alpine
                    name: init-1
                nodeName: docker-desktop
                securityContext: {}
                serviceAccount: default
                serviceAccountName: default
              status:
                hostIP: 192.168.65.4
                phase: Running
                podIP: 10.1.0.212
                podIPs:
                  - ip: 10.1.0.212

            relatedResources:
              services:
                - service-1
                - service-2
              ingresses:
                - ingress-1
                - ingress-2
              networkpolicies:
                - allow-all-egress
                - allow-all-ingress
            "
        }
        .lines()
        .map(ToString::to_string)
        .collect();

        assert_eq!(result.unwrap(), expected)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn エラーが出たときerrを返す() {
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

        let worker = PodDescriptionWorker::new(&client, "default".to_string(), "test".to_string());

        let result = worker.fetch().await;

        assert!(result.is_err());
    }
}

mod extract {
    use k8s_openapi::api::core::v1::{Container, EphemeralContainer, Pod, PodSpec, PodStatus};
    use kube::api::ObjectMeta;

    pub trait Extract {
        fn extract(&self) -> Self
        where
            Self: Sized;
    }

    impl Extract for Pod {
        fn extract(&self) -> Self {
            let metadata = ObjectMeta {
                annotations: self.metadata.annotations.clone(),
                labels: self.metadata.labels.clone(),
                name: self.metadata.name.clone(),
                ..Default::default()
            };

            let spec = self.spec.as_ref().map(|spec| PodSpec {
                containers: spec.containers.iter().map(|c| c.extract()).collect(),
                dns_config: spec.dns_config.clone(),
                dns_policy: spec.dns_policy.clone(),
                enable_service_links: spec.enable_service_links,
                ephemeral_containers: spec.ephemeral_containers.as_ref().map(
                    |ephemeral_containers| {
                        ephemeral_containers.iter().map(|c| c.extract()).collect()
                    },
                ),
                host_aliases: spec.host_aliases.clone(),
                host_ipc: spec.host_ipc,
                host_network: spec.host_network,
                host_pid: spec.host_pid,
                hostname: spec.hostname.clone(),
                init_containers: spec
                    .init_containers
                    .as_ref()
                    .map(|init_containers| init_containers.iter().map(|c| c.extract()).collect()),
                node_name: spec.node_name.clone(),
                node_selector: spec.node_selector.clone(),
                readiness_gates: spec.readiness_gates.clone(),
                security_context: spec.security_context.clone(),
                service_account: spec.service_account.clone(),
                service_account_name: spec.service_account_name.clone(),
                set_hostname_as_fqdn: spec.set_hostname_as_fqdn,
                subdomain: spec.subdomain.clone(),
                ..Default::default()
            });

            let status = self.status.as_ref().map(|status| PodStatus {
                host_ip: status.host_ip.clone(),
                phase: status.phase.clone(),
                pod_ip: status.pod_ip.clone(),
                pod_ips: status.pod_ips.clone(),
                ..Default::default()
            });

            Pod {
                metadata,
                spec,
                status,
            }
        }
    }

    impl Extract for Container {
        fn extract(&self) -> Self {
            Self {
                image: self.image.clone(),
                liveness_probe: self.liveness_probe.clone(),
                name: self.name.clone(),
                ports: self.ports.clone(),
                readiness_probe: self.readiness_probe.clone(),
                security_context: self.security_context.clone(),
                startup_probe: self.startup_probe.clone(),
                ..Default::default()
            }
        }
    }

    impl Extract for EphemeralContainer {
        fn extract(&self) -> Self {
            Self {
                image: self.image.clone(),
                liveness_probe: self.liveness_probe.clone(),
                name: self.name.clone(),
                ports: self.ports.clone(),
                readiness_probe: self.readiness_probe.clone(),
                security_context: self.security_context.clone(),
                startup_probe: self.startup_probe.clone(),
                ..Default::default()
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use indoc::indoc;
        use pretty_assertions::assert_eq;

        use super::*;

        fn pod() -> Pod {
            serde_yaml::from_str(indoc! {
                r#"
                apiVersion: v1
                kind: Pod
                metadata:
                  creationTimestamp: "2022-04-04T03:05:46Z"
                  generateName: kubetui-multi-container-5d69d5ddc6-
                  labels:
                    app: kubetui-multi-container
                    pod-template-hash: 5d69d5ddc6
                  name: kubetui-multi-container-5d69d5ddc6-f7fkw
                  namespace: kubetui
                  ownerReferences:
                    - apiVersion: apps/v1
                      blockOwnerDeletion: true
                      controller: true
                      kind: ReplicaSet
                      name: kubetui-multi-container-5d69d5ddc6
                      uid: f9be5c32-b4a5-4ec9-b8e8-53c240f4e255
                  resourceVersion: "367972"
                  uid: 7a1ffede-c201-4438-893b-f81dc5ded89e
                spec:
                  containers:
                    - args:
                        - while true; do echo app-0; sleep 1; done
                      command:
                        - sh
                        - -c
                      image: nginx
                      imagePullPolicy: Always
                      name: app-0
                      resources: {}
                      terminationMessagePath: /dev/termination-log
                      terminationMessagePolicy: File
                      volumeMounts:
                        - mountPath: /var/run/secrets/kubernetes.io/serviceaccount
                          name: kube-api-access-jdfbz
                          readOnly: true
                    - args:
                        - while true; do echo app-1; sleep 1; done
                      command:
                        - sh
                        - -c
                      image: alpine
                      imagePullPolicy: Always
                      name: app-1
                      resources: {}
                      terminationMessagePath: /dev/termination-log
                      terminationMessagePolicy: File
                      volumeMounts:
                        - mountPath: /var/run/secrets/kubernetes.io/serviceaccount
                          name: kube-api-access-jdfbz
                          readOnly: true
                    - image: nginx
                      imagePullPolicy: Always
                      name: web
                      ports:
                        - containerPort: 80
                          name: http
                          protocol: TCP
                      resources: {}
                      terminationMessagePath: /dev/termination-log
                      terminationMessagePolicy: File
                      volumeMounts:
                        - mountPath: /var/run/secrets/kubernetes.io/serviceaccount
                          name: kube-api-access-jdfbz
                          readOnly: true
                  dnsPolicy: ClusterFirst
                  enableServiceLinks: true
                  initContainers:
                    - args:
                        - echo init-0; exit 0
                      command:
                        - sh
                        - -c
                      image: alpine
                      imagePullPolicy: Always
                      name: init-0
                      resources: {}
                      terminationMessagePath: /dev/termination-log
                      terminationMessagePolicy: File
                      volumeMounts:
                        - mountPath: /var/run/secrets/kubernetes.io/serviceaccount
                          name: kube-api-access-jdfbz
                          readOnly: true
                    - args:
                        - echo init-1; exit 0
                      command:
                        - sh
                        - -c
                      image: alpine
                      imagePullPolicy: Always
                      name: init-1
                      resources: {}
                      terminationMessagePath: /dev/termination-log
                      terminationMessagePolicy: File
                      volumeMounts:
                        - mountPath: /var/run/secrets/kubernetes.io/serviceaccount
                          name: kube-api-access-jdfbz
                          readOnly: true
                  nodeName: docker-desktop
                  preemptionPolicy: PreemptLowerPriority
                  priority: 0
                  restartPolicy: Always
                  schedulerName: default-scheduler
                  securityContext: {}
                  serviceAccount: default
                  serviceAccountName: default
                  terminationGracePeriodSeconds: 30
                  tolerations:
                    - effect: NoExecute
                      key: node.kubernetes.io/not-ready
                      operator: Exists
                      tolerationSeconds: 300
                    - effect: NoExecute
                      key: node.kubernetes.io/unreachable
                      operator: Exists
                      tolerationSeconds: 300
                  volumes:
                    - name: kube-api-access-jdfbz
                      projected:
                        defaultMode: 420
                        sources:
                          - serviceAccountToken:
                              expirationSeconds: 3607
                              path: token
                          - configMap:
                              items:
                                - key: ca.crt
                                  path: ca.crt
                              name: kube-root-ca.crt
                          - downwardAPI:
                              items:
                                - fieldRef:
                                    apiVersion: v1
                                    fieldPath: metadata.namespace
                                  path: namespace
                status:
                  conditions:
                    - lastProbeTime: null
                      lastTransitionTime: "2022-04-04T03:05:55Z"
                      status: "True"
                      type: Initialized
                    - lastProbeTime: null
                      lastTransitionTime: "2022-04-04T03:06:03Z"
                      status: "True"
                      type: Ready
                    - lastProbeTime: null
                      lastTransitionTime: "2022-04-04T03:06:03Z"
                      status: "True"
                      type: ContainersReady
                    - lastProbeTime: null
                      lastTransitionTime: "2022-04-04T03:05:46Z"
                      status: "True"
                      type: PodScheduled
                  containerStatuses:
                    - containerID: docker://5851ea5a23c5983d846ae1e1c0b8ffa2e24340396d7620967177993c0880b0cf
                      image: nginx:latest
                      imageID: docker-pullable://nginx@sha256:2275af0f20d71b293916f1958f8497f987b8d8fd8113df54635f2a5915002bf1
                      lastState: {}
                      name: app-0
                      ready: true
                      restartCount: 0
                      started: true
                      state:
                        running:
                          startedAt: "2022-04-04T03:05:57Z"
                    - containerID: docker://a3896301901509b738877fab4ebfba123693e8966c7be5ef8cfeace059158cc4
                      image: alpine:latest
                      imageID: docker-pullable://alpine@sha256:f22945d45ee2eb4dd463ed5a431d9f04fcd80ca768bb1acf898d91ce51f7bf04
                      lastState: {}
                      name: app-1
                      ready: true
                      restartCount: 0
                      started: true
                      state:
                        running:
                          startedAt: "2022-04-04T03:06:00Z"
                    - containerID: docker://a04620d3e5b2f230837fb745395f70af6289aee96a797e99c0f42f2d68571ae3
                      image: nginx:latest
                      imageID: docker-pullable://nginx@sha256:2275af0f20d71b293916f1958f8497f987b8d8fd8113df54635f2a5915002bf1
                      lastState: {}
                      name: web
                      ready: true
                      restartCount: 0
                      started: true
                      state:
                        running:
                          startedAt: "2022-04-04T03:06:02Z"
                  hostIP: 192.168.65.4
                  initContainerStatuses:
                    - containerID: docker://9bffed2937e48ee8fe5e08447456e07b814308485acaaed541157e4e07bbd95d
                      image: alpine:latest
                      imageID: docker-pullable://alpine@sha256:f22945d45ee2eb4dd463ed5a431d9f04fcd80ca768bb1acf898d91ce51f7bf04
                      lastState: {}
                      name: init-0
                      ready: true
                      restartCount: 0
                      state:
                        terminated:
                          containerID: docker://9bffed2937e48ee8fe5e08447456e07b814308485acaaed541157e4e07bbd95d
                          exitCode: 0
                          finishedAt: "2022-04-04T03:05:49Z"
                          reason: Completed
                          startedAt: "2022-04-04T03:05:49Z"
                    - containerID: docker://85aa30c49fe7cb8343c52bc3a100f6c226cfef18fad6d68d04cf02d53cda0ca1
                      image: alpine:latest
                      imageID: docker-pullable://alpine@sha256:f22945d45ee2eb4dd463ed5a431d9f04fcd80ca768bb1acf898d91ce51f7bf04
                      lastState: {}
                      name: init-1
                      ready: true
                      restartCount: 0
                      state:
                        terminated:
                          containerID: docker://85aa30c49fe7cb8343c52bc3a100f6c226cfef18fad6d68d04cf02d53cda0ca1
                          exitCode: 0
                          finishedAt: "2022-04-04T03:05:54Z"
                          reason: Completed
                          startedAt: "2022-04-04T03:05:54Z"
                  phase: Running
                  podIP: 10.1.0.212
                  podIPs:
                    - ip: 10.1.0.212
                  qosClass: BestEffort
                  startTime: "2022-04-04T03:05:46Z"
                "#
            })
            .unwrap()
        }

        #[test]
        fn 必要な情報のみを抽出してpodを返す() {
            let actual = pod().extract();

            let expected = serde_yaml::from_str(indoc! {
                r#"
                apiVersion: v1
                kind: Pod
                metadata:
                  labels:
                    app: kubetui-multi-container
                    pod-template-hash: 5d69d5ddc6
                  name: kubetui-multi-container-5d69d5ddc6-f7fkw
                spec:
                  containers:
                    - image: nginx
                      name: app-0
                    - image: alpine
                      name: app-1
                    - image: nginx
                      name: web
                      ports:
                        - containerPort: 80
                          name: http
                          protocol: TCP
                  dnsPolicy: ClusterFirst
                  enableServiceLinks: true
                  initContainers:
                    - image: alpine
                      name: init-0
                    - image: alpine
                      name: init-1
                  nodeName: docker-desktop
                  securityContext: {}
                  serviceAccount: default
                  serviceAccountName: default
                status:
                  hostIP: 192.168.65.4
                  phase: Running
                  podIP: 10.1.0.212
                  podIPs:
                    - ip: 10.1.0.212
                "#
            })
            .unwrap();

            assert_eq!(actual, expected);
        }
    }
}

mod to_value {
    use anyhow::Result;
    use k8s_openapi::api::core::v1::Pod;
    use serde_yaml::{Mapping, Value};

    pub trait ToValue {
        fn to_value(&self) -> Result<Option<Value>>;
    }

    impl ToValue for Pod {
        fn to_value(&self) -> Result<Option<Value>> {
            let mut value = Mapping::new();

            value.insert("metadata".into(), serde_yaml::to_value(&self.metadata)?);

            if let Some(spec) = &self.spec {
                value.insert("spec".into(), serde_yaml::to_value(spec)?);
            }

            if let Some(status) = &self.status {
                value.insert("status".into(), serde_yaml::to_value(status)?);
            }

            let ret = if !value.is_empty() {
                let mut root = Mapping::new();

                root.insert("pod".into(), value.into());

                Some(root.into())
            } else {
                None
            };

            Ok(ret)
        }
    }
}
