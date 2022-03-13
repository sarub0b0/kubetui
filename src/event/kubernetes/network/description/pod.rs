mod fetched_ingress;
mod fetched_network_policy;
mod fetched_pod;
mod fetched_service;

use fetched_ingress::*;
use fetched_pod::*;
use fetched_service::*;
use k8s_openapi::api::{
    core::v1::Service,
    networking::v1::{Ingress, IngressSpec},
};
use serde_yaml::{Mapping, Value};

use super::DescriptionWorker;

use std::collections::BTreeMap;

use crossbeam::channel::Sender;

use serde::{Deserialize, Serialize};

use crate::{
    error::Result,
    event::{
        kubernetes::{client::KubeClientRequest, network::NetworkMessage},
        Event,
    },
};

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
    C: KubeClientRequest + Clone,
{
    client: &'a C,
    tx: &'a Sender<Event>,
    namespace: String,
    name: String,
}

#[async_trait::async_trait]
impl<'a, C: KubeClientRequest + Clone> DescriptionWorker<'a, C> for PodDescriptionWorker<'a, C> {
    fn new(client: &'a C, tx: &'a Sender<Event>, namespace: String, name: String) -> Self {
        PodDescriptionWorker {
            client,
            tx,
            namespace,
            name,
        }
    }

    // TODO 関連するService, Ingress, NetworkPolicyの情報を合わせて表示する
    async fn run(&self) -> Result<()> {
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

            if let Ok(resources) = serde_yaml::to_string(&root) {
                let vec: Vec<String> = resources.lines().skip(1).map(ToString::to_string).collect();

                value.push("\n".to_string());
                value.extend(vec);
            }
        }

        self.tx.send(NetworkMessage::Response(Ok(value)).into())?;

        Ok(())
    }
}

impl<C: KubeClientRequest + Clone> PodDescriptionWorker<'_, C> {
    async fn fetch_pod(&self) -> Result<FetchedPod> {
        let url = format!("api/v1/namespaces/{}/pods/{}", self.namespace, self.name);

        let res = self.client.request_text(&url).await?;

        let value: FetchedPod = serde_json::from_str(&res)?;

        Ok(value)
    }

    async fn fetch_service(
        &self,
        pod_labels: &Option<BTreeMap<String, String>>,
    ) -> Result<Option<FetchedService>> {
        let url = format!("api/v1/namespaces/{}/services", self.namespace);
        let res = self.client.request_text(&url).await?;

        let list: FetchedServiceList = serde_json::from_str(&res)?;

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

        let res = self.client.request_text(&url).await?;

        let list: FetchedIngressList = serde_json::from_str(&res)?;

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
}
