#![allow(dead_code)]
mod fetched_ingress;
mod fetched_network_policy;
mod fetched_pod;
mod fetched_service;

pub(super) use fetched_ingress::*;
pub(super) use fetched_pod::*;
pub(super) use fetched_service::*;

use k8s_openapi::api::{
    core::v1::Service,
    networking::v1::{Ingress, IngressSpec},
};
use serde_yaml::{Mapping, Value};

use super::{Fetch, FetchedData, Result};

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::event::kubernetes::client::KubeClientRequest;

#[derive(Debug)]
enum FetchArgs {
    Value(String),
    List(Vec<String>),
}

trait FetchedResource {
    type Resource;

    fn fetch(args: Option<FetchArgs>) -> Result<Self::Resource>;
    fn to_value(&self) -> Option<Value>;
}

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

    // TODO 関連するService, Ingress, NetworkPolicyの情報を合わせて表示する
    async fn fetch(&self) -> Result<FetchedData> {
        let mut value = Vec::new();

        let pod = self.fetch_pod().await?;
        let services = self.fetch_service(&pod.0.metadata.labels).await?;

        let ingresses = if let Some(services) = &services {
            let services: Vec<String> = services
                .0
                .iter()
                .cloned()
                .filter_map(|service| service.metadata.name)
                .collect();

            self.fetch_ingress(&services).await?
        } else {
            None
        };

        value.extend(pod.to_vec_string());

        let mut related_resources = Mapping::new();
        if let Some(services) = services {
            if let Some(svc) = services.to_value() {
                related_resources.insert("services".into(), svc);
            }
        }

        if let Some(ingresses) = ingresses {
            if let Some(ing) = ingresses.to_value() {
                related_resources.insert("ingresses".into(), ing);
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

impl<C: KubeClientRequest> PodDescriptionWorker<'_, C> {
    async fn fetch_pod(&self) -> Result<FetchedPod> {
        let url = format!("api/v1/namespaces/{}/pods/{}", self.namespace, self.name);

        let value: FetchedPod = self.client.request(&url).await?;

        Ok(value)
    }

    async fn fetch_service(
        &self,
        pod_labels: &Option<BTreeMap<String, String>>,
    ) -> Result<Option<FetchedService>> {
        let url = format!("api/v1/namespaces/{}/services", self.namespace);

        let list: FetchedServiceList = self.client.request(&url).await?;

        let services: Vec<Service> = list
            .items
            .into_iter()
            .filter(|s| {
                s.spec.as_ref().map_or(false, |spec| {
                    contains_key_values(&spec.selector, pod_labels)
                })
            })
            .collect();

        if !services.is_empty() {
            Ok(Some(FetchedService(services)))
        } else {
            Ok(None)
        }
    }

    async fn fetch_ingress(&self, services: &[String]) -> Result<Option<FetchedIngress>> {
        let url = format!(
            "apis/networking.k8s.io/v1/namespaces/{}/ingresses",
            self.namespace
        );

        let list: FetchedIngressList = self.client.request(&url).await?;

        let ingresses: Vec<Ingress> = list
            .items
            .into_iter()
            .filter(|ing| {
                ing.spec
                    .as_ref()
                    .map_or(false, |spec| contains_service_into_ingress(spec, services))
            })
            .collect();

        if !ingresses.is_empty() {
            Ok(Some(FetchedIngress(ingresses)))
        } else {
            Ok(None)
        }
    }
}

fn contains_service_into_ingress(ingress_spec: &IngressSpec, services: &[String]) -> bool {
    ingress_spec
        .default_backend
        .as_ref()
        .map_or(false, |default_backend| {
            default_backend
                .service
                .as_ref()
                .map_or(false, |backend_service| {
                    services
                        .iter()
                        .any(|service| &backend_service.name == service)
                })
        })
        || ingress_spec.rules.as_ref().map_or(false, |rules| {
            rules.iter().any(|rule| {
                rule.http.as_ref().map_or(false, |http| {
                    http.paths.iter().any(|path| {
                        path.backend
                            .service
                            .as_ref()
                            .map_or(false, |backend_service| {
                                services
                                    .iter()
                                    .any(|service| &backend_service.name == service)
                            })
                    })
                })
            })
        })
}

fn contains_key_values(
    lhs: &Option<BTreeMap<String, String>>,
    rhs: &Option<BTreeMap<String, String>>,
) -> bool {
    #[cfg(feature = "logging")]
    ::log::debug!("match_selector {:#?} <=> {:#?}", service_labels, pod_labels);

    lhs.as_ref().map_or(false, |lhs| {
        rhs.as_ref().map_or(false, |rhs| {
            lhs.iter().all(|(lhs_key, lhs_value)| {
                rhs.get(lhs_key)
                    .map_or(false, |rhs_value| lhs_value == rhs_value)
            })
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    mod match_selector {
        use super::*;

        #[test]
        fn lhsの値すべてがrhsにふくまれていればtrueを返す() {
            let lhs = Some(BTreeMap::from_iter(vec![
                ("a".to_string(), "aaa".to_string()),
                ("b".to_string(), "bbb".to_string()),
            ]));

            let rhs = Some(BTreeMap::from_iter(vec![
                ("a".to_string(), "aaa".to_string()),
                ("b".to_string(), "bbb".to_string()),
                ("c".to_string(), "ccc".to_string()),
            ]));

            assert!(contains_key_values(&lhs, &rhs));
        }

        #[test]
        fn lhsの値すべてがrhsにふくまれていなければfalseを返す() {
            let lhs = Some(BTreeMap::from_iter(vec![
                ("a".to_string(), "aaa".to_string()),
                ("b".to_string(), "bbb".to_string()),
            ]));

            let rhs = Some(BTreeMap::from_iter(vec![
                ("b".to_string(), "bbb".to_string()),
                ("c".to_string(), "ccc".to_string()),
            ]));

            assert!(!contains_key_values(&lhs, &rhs));
        }
    }

    mod fetch {
        use super::*;

        use crate::{event::kubernetes::client::mock::MockTestKubeClient, mock_expect};
        use indoc::indoc;
        use k8s_openapi::api::core::v1::Pod;
        use mockall::predicate::eq;

        use pretty_assertions::assert_eq;

        use anyhow::anyhow;

        fn setup_pod() -> FetchedPod {
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
            " };

            let pod: Pod = serde_yaml::from_str(&yaml).unwrap();

            FetchedPod(pod)
        }

        fn setup_services() -> FetchedServiceList {
            let yaml = indoc! {
            "
            items:
              - metadata:
                  name: service-1
                spec:
                  selector:
                    job-name: kubetui-text-color
              - metadata:
                  name: service-2
                spec:
                  selector:
                    job-name: kubetui-text-color
            "
            };

            serde_yaml::from_str(&yaml).unwrap()
        }

        fn setup_ingresses() -> FetchedIngressList {
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

            serde_yaml::from_str(&yaml).unwrap()
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn yamlデータを送信してokを返す() {
            let mut client = MockTestKubeClient::new();
            mock_expect!(
                client,
                request,
                [
                    (
                        FetchedPod,
                        eq("api/v1/namespaces/default/pods/test"),
                        Ok(setup_pod())
                    ),
                    (
                        FetchedServiceList,
                        eq("api/v1/namespaces/default/services"),
                        Ok(setup_services())
                    ),
                    (
                        FetchedIngressList,
                        eq("apis/networking.k8s.io/v1/namespaces/default/ingresses"),
                        Ok(setup_ingresses())
                    )
                ]
            );

            let worker =
                PodDescriptionWorker::new(&client, "default".to_string(), "test".to_string());

            let result = worker.fetch().await;

            let expected: Vec<String> = indoc! {
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

                relatedResources:
                  services:
                    - service-1
                    - service-2
                  ingresses:
                    - ingress-1
                    - ingress-2
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
                        FetchedPod,
                        eq("api/v1/namespaces/default/pods/test"),
                        Err(anyhow!("error"))
                    ),
                    (
                        FetchedServiceList,
                        eq("api/v1/namespaces/default/services"),
                        Err(anyhow!("error"))
                    ),
                    (
                        FetchedIngressList,
                        eq("apis/networking.k8s.io/v1/namespaces/default/ingresses"),
                        Err(anyhow!("error"))
                    )
                ]
            );

            let worker =
                PodDescriptionWorker::new(&client, "default".to_string(), "test".to_string());

            let result = worker.fetch().await;

            assert!(result.is_err());
        }
    }
}
