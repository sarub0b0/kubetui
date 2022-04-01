use k8s_openapi::{
    api::{
        core::v1::{Pod, Service, ServiceSpec},
        networking::v1::Ingress,
    },
    List,
};
use kube::{Resource, ResourceExt};
use serde_yaml::Mapping;

use crate::{error::Result, event::kubernetes::client::KubeClientRequest};

use self::to_value::ToValue;

use super::{
    related_resources::{
        ingress::filter_by_service::RelatedIngress, pod::filter_by_labels::RelatedPod,
        to_list_value::ToListValue, RelatedResources,
    },
    Fetch, FetchedData,
};

use extract::Extract;

pub(super) struct ServiceDescriptionWorker<'a, C>
where
    C: KubeClientRequest,
{
    client: &'a C,
    namespace: String,
    name: String,
}

#[async_trait::async_trait]
impl<'a, C> Fetch<'a, C> for ServiceDescriptionWorker<'a, C>
where
    C: KubeClientRequest,
{
    fn new(client: &'a C, namespace: String, name: String) -> Self {
        Self {
            client,
            namespace,
            name,
        }
    }

    async fn fetch(&self) -> Result<FetchedData> {
        let url = format!(
            "{}/{}",
            Service::url_path(&(), Some(&self.namespace)),
            self.name
        );

        let service: Service = self.client.request(&url).await?;
        let service = service.extract();

        let related_ingresses: Option<List<Ingress>> =
            RelatedIngress::new(self.client, &self.namespace, vec![service.name()])
                .related_resources()
                .await?;

        let related_pods: Option<List<Pod>> = if let Some(ServiceSpec {
            selector: Some(selector),
            ..
        }) = &service.spec
        {
            RelatedPod::new(self.client, &self.namespace, vec![selector.clone()])
                .related_resources()
                .await?
        } else {
            None
        };

        let mut related_resources = Mapping::new();

        if let Some(ingresses) = related_ingresses {
            if let Some(value) = ingresses.to_list_value() {
                related_resources.insert("ingresses".into(), value);
            }
        }

        if let Some(pods) = related_pods {
            if let Some(value) = pods.to_list_value() {
                related_resources.insert("pods".into(), value);
            }
        }

        let service: Vec<String> = serde_yaml::to_string(&service.to_value()?)?
            .lines()
            .skip(1)
            .map(ToString::to_string)
            .collect();

        let mut value = service;

        if !related_resources.is_empty() {
            let mut root = Mapping::new();

            root.insert("relatedResources".into(), related_resources.into());

            let related_resources: Vec<String> = serde_yaml::to_string(&root)?
                .lines()
                .skip(1)
                .map(ToString::to_string)
                .collect();

            value.push(Default::default());

            value.extend(related_resources);
        }

        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use anyhow::bail;
    use indoc::indoc;
    use k8s_openapi::{
        api::{core::v1::Pod, networking::v1::Ingress},
        List,
    };
    use mockall::predicate::eq;
    use pretty_assertions::assert_eq;

    use crate::{event::kubernetes::client::mock::MockTestKubeClient, mock_expect};

    use super::*;

    fn service() -> Service {
        serde_yaml::from_str(indoc! {
            "
            metadata:
              name: service
            spec:
              clusterIP: 10.101.97.182
              clusterIPs:
                - 10.101.97.182
              ipFamilies:
                - IPv4
              ipFamilyPolicy: SingleStack
              ports:
                - port: 80
                  protocol: TCP
                  targetPort: 80
              selector:
                version: v1
              sessionAffinity: None
              type: ClusterIP
            "
        })
        .unwrap()
    }

    fn pods() -> List<Pod> {
        serde_yaml::from_str(indoc! {
            "
            items:
              - metadata:
                  name: pod-1
                  labels:
                    app: pod-1
                    version: v1
              - metadata:
                  name: pod-2
                  labels:
                    app: pod-2
                    version: v1
              - metadata:
                  name: pod-3
                  labels:
                    app: pod-3
                    version: v2
            "
        })
        .unwrap()
    }

    fn ingresses() -> List<Ingress> {
        serde_yaml::from_str(indoc! {
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
                                name: service
                          - backend:
                              service:
                                name: service-2
              - metadata:
                  name: ingress-2
                spec:
                  rules:
                    - http:
                        paths:
                          - backend:
                              service:
                                name: service-3

            "
        })
        .unwrap()
    }

    #[tokio::test]
    async fn yamlデータを返す() {
        let mut client = MockTestKubeClient::new();
        mock_expect!(
            client,
            request,
            [
                (
                    Service,
                    eq("/api/v1/namespaces/default/services/service"),
                    Ok(service())
                ),
                (
                    List<Ingress>,
                    eq("/apis/networking.k8s.io/v1/namespaces/default/ingresses"),
                    Ok(ingresses())
                ),
                (
                    List<Pod>,
                    eq("/api/v1/namespaces/default/pods"),
                    Ok(pods())
                )
            ]
        );

        let worker =
            ServiceDescriptionWorker::new(&client, "default".to_string(), "service".to_string());

        let result = worker.fetch().await;

        let expected: Vec<String> = indoc! {
            "
            service:
              metadata:
                name: service
              spec:
                clusterIP: 10.101.97.182
                clusterIPs:
                  - 10.101.97.182
                ipFamilies:
                  - IPv4
                ipFamilyPolicy: SingleStack
                ports:
                  - port: 80
                    protocol: TCP
                    targetPort: 80
                selector:
                  version: v1
                sessionAffinity: None
                type: ClusterIP

            relatedResources:
              ingresses:
                - ingress-1
              pods:
                - pod-1
                - pod-2
            "
        }
        .lines()
        .map(ToString::to_string)
        .collect();

        assert_eq!(result.unwrap(), expected);
    }

    #[tokio::test]
    async fn エラーのときerrorを返す() {
        let mut client = MockTestKubeClient::new();
        mock_expect!(
            client,
            request,
            [
                (
                    Service,
                    eq("/api/v1/namespaces/default/services/test"),
                    bail!("error")
                ),
                (
                    List<Ingress>,
                    eq("/apis/networking.k8s.io/v1/namespaces/default/ingresses"),
                    bail!("error")
                ),
                (
                    List<Pod>,
                    eq("/api/v1/namespaces/default/pods"),
                    bail!("error")
                )

            ]
        );

        let worker =
            ServiceDescriptionWorker::new(&client, "default".to_string(), "test".to_string());

        let result = worker.fetch().await;

        assert_eq!(result.is_err(), true);
    }
}

mod to_value {
    use anyhow::Result;
    use k8s_openapi::api::core::v1::Service;
    use serde_yaml::{Mapping, Value};

    pub trait ToValue {
        fn to_value(&self) -> Result<Option<Value>>;
    }

    impl ToValue for Service {
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

                root.insert("service".into(), value.into());

                Some(root.into())
            } else {
                None
            };

            Ok(ret)
        }
    }
}

mod extract {
    use k8s_openapi::api::core::v1::Service;
    use kube::api::ObjectMeta;

    pub trait Extract {
        fn extract(&self) -> Self
        where
            Self: Sized;
    }

    impl Extract for Service {
        fn extract(&self) -> Self {
            let annotations = if let Some(mut annotations) = self.metadata.annotations.clone() {
                annotations.remove("kubectl.kubernetes.io/last-applied-configuration");
                if annotations.is_empty() {
                    None
                } else {
                    Some(annotations)
                }
            } else {
                None
            };
            Service {
                metadata: ObjectMeta {
                    annotations,
                    labels: self.metadata.labels.clone(),
                    name: self.metadata.name.clone(),
                    ..Default::default()
                },
                spec: self.spec.clone(),
                status: self.status.clone(),
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use indoc::indoc;
        use pretty_assertions::assert_eq;

        use super::*;

        fn service() -> Service {
            serde_yaml::from_str(indoc! {
                r#"
                apiVersion: v1
                kind: Service
                metadata:
                  annotations:
                    kubectl.kubernetes.io/last-applied-configuration: |
                      {"apiVersion":"v1","kind":"Service","metadata":{"annotations":{},"name":"service-0","namespace":"kubetui"},"spec":{"ports":[{"port":80,"targetPort":80}],"selector":{"app":"app"}}}
                    foo: bar
                  labels:
                    foo: bar
                  creationTimestamp: "2022-03-27T09:17:06Z"
                  name: service-0
                  namespace: kubetui
                  resourceVersion: "714"
                  uid: 7971c237-d5d8-468d-aeb9-ee6f9449c702
                spec:
                  clusterIP: 10.108.138.180
                  clusterIPs:
                  - 10.108.138.180
                  internalTrafficPolicy: Cluster
                  ipFamilies:
                  - IPv4
                  ipFamilyPolicy: SingleStack
                  ports:
                  - port: 80
                    protocol: TCP
                    targetPort: 80
                  selector:
                    app: app
                  sessionAffinity: None
                  type: ClusterIP
                status:
                  loadBalancer: {}
                "#
            })
            .unwrap()
        }

        #[test]
        fn 必要な情報のみを抽出してserviceを返す() {
            let actual = service().extract();

            let expected = serde_yaml::from_str(indoc! {
                r#"
                apiVersion: v1
                kind: Service
                metadata:
                  annotations:
                    foo: bar
                  labels:
                    foo: bar
                  name: service-0
                spec:
                  clusterIP: 10.108.138.180
                  clusterIPs:
                  - 10.108.138.180
                  internalTrafficPolicy: Cluster
                  ipFamilies:
                  - IPv4
                  ipFamilyPolicy: SingleStack
                  ports:
                  - port: 80
                    protocol: TCP
                    targetPort: 80
                  selector:
                    app: app
                  sessionAffinity: None
                  type: ClusterIP
                status:
                  loadBalancer: {}
                "#
            })
            .unwrap();

            assert_eq!(actual, expected);
        }
    }
}
