mod fetched_pod;

pub(super) use fetched_pod::*;

use k8s_openapi::api::core::v1::Pod;
use kube::{Resource, ResourceExt};
use serde_yaml::{Mapping, Value};

use super::{
    related_resources::{
        ingress::filter_by_service::RelatedIngress, service::filter_by_selector::RelatedService,
        RelatedResources,
    },
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

        let related_services = RelatedService::new(self.client, &self.namespace, pod.0.labels())
            .related_resources()
            .await?;

        let related_ingresses: Option<Value> = if let Some(services) = &related_services {
            let services: Vec<String> = serde_yaml::from_value(services.clone())?;
            RelatedIngress::new(self.client, &self.namespace, services)
                .related_resources()
                .await?
        } else {
            None
        };

        value.extend(pod.to_vec_string());

        let mut related_resources = Mapping::new();
        if let Some(services) = related_services {
            related_resources.insert("services".into(), services);
        }

        if let Some(ingresses) = related_ingresses {
            related_resources.insert("ingresses".into(), ingresses);
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
            networking::v1::Ingress,
        },
        List,
    };
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
            "
        };

        let pod: Pod = serde_yaml::from_str(&yaml).unwrap();

        FetchedPod(pod)
    }

    fn setup_services() -> List<Service> {
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

    fn setup_ingresses() -> List<Ingress> {
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
                        eq("/api/v1/namespaces/default/pods/test"),
                        Ok(setup_pod())
                    ),
                    (
                        List<Service>,
                        eq("/api/v1/namespaces/default/services"),
                        Ok(setup_services())
                    ),
                    (
                        List<Ingress>,
                        eq("/apis/networking.k8s.io/v1/namespaces/default/ingresses"),
                        Ok(setup_ingresses())
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
                    )
                ]
            );

        let worker = PodDescriptionWorker::new(&client, "default".to_string(), "test".to_string());

        let result = worker.fetch().await;

        assert!(result.is_err());
    }
}
