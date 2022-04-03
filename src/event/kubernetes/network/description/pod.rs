mod fetched_pod;

pub(super) use fetched_pod::*;

use k8s_openapi::{
    api::{
        core::v1::{Pod, Service},
        networking::v1::{Ingress, NetworkPolicy},
    },
    List,
};
use kube::{Resource, ResourceExt};
use serde_yaml::Mapping;

use super::{
    related_resources::{to_list_value::ToListValue, RelatedClient},
    Fetch, FetchedData, Result,
};

use serde::Deserialize;

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
        let mut value = Vec::new();

        let url = format!(
            "{}/{}",
            Pod::url_path(&(), Some(&self.namespace)),
            self.name
        );

        let pod: FetchedPod = self.client.request(&url).await?;

        let related_services = RelatedClient::new(self.client, &self.namespace)
            .related_resources::<Service, _>(pod.0.labels())
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
            if let Some(labels) = &pod.0.metadata.labels {
                RelatedClient::new(self.client, &self.namespace)
                    .related_resources(labels)
                    .await?
            } else {
                None
            };

        value.extend(pod.to_vec_string());

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

    use anyhow::anyhow;

    fn pod() -> FetchedPod {
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

        let pod: Pod = serde_yaml::from_str(&yaml).unwrap();

        FetchedPod(pod)
    }

    fn services() -> List<Service> {
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

        serde_yaml::from_str(&yaml).unwrap()
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
                        FetchedPod,
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
                        FetchedPod,
                        eq("/api/v1/namespaces/default/pods/test"),
                        Err(anyhow!("error"))
                    ),
                    (
                        List<Service>,
                        eq("/api/v1/namespaces/default/services"),
                        Err(anyhow!("error"))
                    ),
                    (
                        List<Ingress>,
                        eq("/apis/networking.k8s.io/v1/namespaces/default/ingresses"),
                        Err(anyhow!("error"))
                    ),
                    (
                        List<NetworkPolicy>,
                        eq("/apis/networking.k8s.io/v1/namespaces/default/networkpolicies"),
                        Err(anyhow!("error"))
                    )
                ]
            );

        let worker = PodDescriptionWorker::new(&client, "default".to_string(), "test".to_string());

        let result = worker.fetch().await;

        assert!(result.is_err());
    }
}
