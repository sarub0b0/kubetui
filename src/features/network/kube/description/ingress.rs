use std::collections::BTreeMap;

use anyhow::Result;
use k8s_openapi::api::{
    core::v1::{Pod, Service},
    networking::v1::Ingress,
};
use kube::Resource;
use serde_yaml::Mapping;

use crate::{
    features::{
        api_resources::kube::SharedApiResources, network::message::NetworkRequestTargetParams,
    },
    kube::KubeClientRequest,
};

use super::{
    Fetch, FetchedData,
    related_resources::{RelatedClient, to_list_value::ToListValue},
};

use extract::Extract;
use to_value::ToValue;

pub(super) struct IngressDescriptionWorker<'a, C>
where
    C: KubeClientRequest,
{
    client: &'a C,
    namespace: String,
    name: String,
}

#[async_trait::async_trait]
impl<'a, C> Fetch<'a, C> for IngressDescriptionWorker<'a, C>
where
    C: KubeClientRequest,
{
    fn new(client: &'a C, params: NetworkRequestTargetParams, _: SharedApiResources) -> Self {
        let NetworkRequestTargetParams {
            namespace, name, ..
        } = params;

        Self {
            client,
            namespace,
            name,
        }
    }

    async fn fetch(&self) -> Result<FetchedData> {
        let url = format!(
            "{}/{}",
            Ingress::url_path(&(), Some(&self.namespace)),
            self.name
        );

        let ingress: Ingress = self.client.request(&url).await?;
        let ingress = ingress.extract();

        let services: Option<Vec<String>> = backend_service_names(&ingress);

        let related_services = if let Some(services) = services {
            RelatedClient::new(self.client, &self.namespace)
                .related_resources::<Service, _>(&services)
                .await?
        } else {
            None
        };

        let related_pods = if let Some(services) = &related_services {
            let selectors: Vec<BTreeMap<String, String>> = services
                .items
                .iter()
                .filter_map(|svc| svc.spec.as_ref())
                .filter_map(|spec| spec.selector.clone())
                .collect();

            RelatedClient::new(self.client, &self.namespace)
                .related_resources::<Pod, _>(&selectors)
                .await?
        } else {
            None
        };

        let mut related_resources = Mapping::new();

        if let Some(services) = related_services {
            if let Some(value) = services.to_list_value() {
                related_resources.insert("services".into(), value);
            }
        }

        if let Some(pods) = related_pods {
            if let Some(value) = pods.to_list_value() {
                related_resources.insert("pods".into(), value);
            }
        }

        let ingress: Vec<String> = serde_yaml::to_string(&ingress.to_value()?)?
            .lines()
            .map(ToString::to_string)
            .collect();

        let mut value = ingress;

        if !related_resources.is_empty() {
            let mut root = Mapping::new();

            root.insert("relatedResources".into(), related_resources.into());

            let related_resources: Vec<String> = serde_yaml::to_string(&root)?
                .lines()
                .map(ToString::to_string)
                .collect();

            value.push(Default::default());

            value.extend(related_resources);
        }

        Ok(value)
    }
}

fn backend_service_names(ing: &Ingress) -> Option<Vec<String>> {
    let names: Option<Vec<String>> = ing.spec.as_ref().and_then(|spec| {
        spec.rules.as_ref().and_then(|rules| {
            let a: Vec<String> = rules
                .iter()
                .flat_map(|rule| {
                    rule.http.as_ref().map_or(vec![], |http| {
                        http.paths
                            .iter()
                            .filter_map(|path| path.backend.service.as_ref())
                            .map(|service| service.name.clone())
                            .collect()
                    })
                })
                .collect();

            if !a.is_empty() { Some(a) } else { None }
        })
    });

    names
}

#[cfg(test)]
mod tests {
    use anyhow::bail;
    use indoc::indoc;
    use k8s_openapi::{
        List,
        api::{
            core::v1::{Pod, Service},
            networking::v1::Ingress,
        },
    };
    use mockall::predicate::eq;
    use pretty_assertions::assert_eq;

    use crate::{
        features::{
            api_resources::kube::ApiResources, network::message::NetworkRequestTargetParams,
        },
        kube::mock::MockTestKubeClient,
        mock_expect,
    };

    use super::*;

    fn ingress() -> Ingress {
        serde_yaml::from_str(indoc! {
            r#"
            apiVersion: networking.k8s.io/v1
            kind: Ingress
            metadata:
              annotations:
                kubectl.kubernetes.io/last-applied-configuration: |
                  {"apiVersion":"networking.k8s.io/v1","kind":"Ingress","metadata":{"annotations":{},"name":"ingress","namespace":"kubetui"},"spec":{"rules":[{"host":"example-0.com","http":{"paths":[{"backend":{"service":{"name":"service-0","port":{"number":80}}},"path":"/path","pathType":"ImplementationSpecific"}]}},{"host":"example-1.com","http":{"paths":[{"backend":{"service":{"name":"service-1","port":{"number":80}}},"path":"/path","pathType":"ImplementationSpecific"}]}}],"tls":[{"hosts":["example.com"],"secretName":"secret-name"}]}}
              creationTimestamp: "2022-03-27T09:17:06Z"
              generation: 1
              name: ingress
              resourceVersion: "710"
              uid: 28a8cecd-8bbb-476f-8e34-eb86a8a8255f
            spec:
              rules:
              - host: example-0.com
                http:
                  paths:
                  - backend:
                      service:
                        name: service-1
                        port:
                          number: 80
                    path: /path
                    pathType: ImplementationSpecific
                  - backend:
                      service:
                        name: service-2
                        port:
                          number: 80
                    path: /path
                    pathType: ImplementationSpecific

              - host: example-1.com
                http:
                  paths:
                  - backend:
                      service:
                        name: service-3
                        port:
                          number: 80
                    path: /path
                    pathType: ImplementationSpecific
              tls:
              - hosts:
                - example.com
                secretName: secret-name
            status:
              loadBalancer: {}
            "#
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

    fn services() -> List<Service> {
        serde_yaml::from_str(indoc! {
            "
            items:
            - metadata:
                name: service-1
              spec:
                selector:
                  app: pod-1
                  version: v1
            - metadata:
                name: service-2
              spec:
                 selector:
                  app: pod-2
                  version: v1
            - metadata:
                name: service-3
              spec:
                 selector:
                  app: pod-3
                  version: v2
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
                    Ingress,
                    eq("/apis/networking.k8s.io/v1/namespaces/default/ingresses/ingress"),
                    Ok(ingress())
                ),
                (
                    List<Service>,
                    eq("/api/v1/namespaces/default/services"),
                    Ok(services())
                ),
                (
                    List<Pod>,
                    eq("/api/v1/namespaces/default/pods"),
                    Ok(pods())
                )
            ]
        );

        let target_params = NetworkRequestTargetParams {
            namespace: "default".to_string(),
            name: "ingress".to_string(),
            version: "v1".to_string(),
        };

        let worker = IngressDescriptionWorker::new(&client, target_params, ApiResources::shared());

        let result = worker.fetch().await;

        let expected: Vec<String> = indoc! {
            "
            ingress:
              metadata:
                name: ingress
              spec:
                rules:
                - host: example-0.com
                  http:
                    paths:
                    - backend:
                        service:
                          name: service-1
                          port:
                            number: 80
                      path: /path
                      pathType: ImplementationSpecific
                    - backend:
                        service:
                          name: service-2
                          port:
                            number: 80
                      path: /path
                      pathType: ImplementationSpecific
                - host: example-1.com
                  http:
                    paths:
                    - backend:
                        service:
                          name: service-3
                          port:
                            number: 80
                      path: /path
                      pathType: ImplementationSpecific
                tls:
                - hosts:
                  - example.com
                  secretName: secret-name
              status:
                loadBalancer: {}

            relatedResources:
              services:
              - service-1
              - service-2
              - service-3
              pods:
              - pod-1
              - pod-2
              - pod-3
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
                    Ingress,
                    eq("/apis/networking.k8s.io/v1/namespaces/default/ingresses/test"),
                    bail!("error")
                ),
                (
                    List<Service>,
                    eq("/api/v1/namespaces/default/services"),
                    bail!("error")
                ),
                (
                    List<Pod>,
                    eq("/api/v1/namespaces/default/pods"),
                    bail!("error")
                )

            ]
        );

        let target_params = NetworkRequestTargetParams {
            namespace: "default".to_string(),
            name: "test".to_string(),
            version: "v1".to_string(),
        };

        let worker = IngressDescriptionWorker::new(&client, target_params, ApiResources::shared());

        let result = worker.fetch().await;

        assert_eq!(result.is_err(), true);
    }
}

mod to_value {
    use anyhow::Result;
    use k8s_openapi::api::networking::v1::Ingress;
    use serde_yaml::{Mapping, Value};

    pub trait ToValue {
        fn to_value(&self) -> Result<Option<Value>>;
    }

    impl ToValue for Ingress {
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

                root.insert("ingress".into(), value.into());

                Some(root.into())
            } else {
                None
            };

            Ok(ret)
        }
    }
}

mod extract {
    use k8s_openapi::api::networking::v1::Ingress;
    use kube::api::ObjectMeta;

    pub trait Extract {
        fn extract(&self) -> Self
        where
            Self: Sized;
    }

    impl Extract for Ingress {
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
            Ingress {
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

        fn ingress() -> Ingress {
            serde_yaml::from_str(indoc! {
                r#"
                apiVersion: networking.k8s.io/v1
                kind: Ingress
                metadata:
                  annotations:
                    kubectl.kubernetes.io/last-applied-configuration: |
                      {"apiVersion":"networking.k8s.io/v1","kind":"Ingress","metadata":{"annotations":{},"name":"ingress","namespace":"kubetui"},"spec":{"rules":[{"host":"example-0.com","http":{"paths":[{"backend":{"service":{"name":"service-0","port":{"number":80}}},"path":"/path","pathType":"ImplementationSpecific"}]}},{"host":"example-1.com","http":{"paths":[{"backend":{"service":{"name":"service-1","port":{"number":80}}},"path":"/path","pathType":"ImplementationSpecific"}]}}],"tls":[{"hosts":["example.com"],"secretName":"secret-name"}]}}
                  creationTimestamp: "2022-03-27T09:17:06Z"
                  generation: 1
                  name: ingress
                  namespace: kubetui
                  resourceVersion: "710"
                  uid: 28a8cecd-8bbb-476f-8e34-eb86a8a8255f
                spec:
                  rules:
                  - host: example-0.com
                    http:
                      paths:
                      - backend:
                          service:
                            name: service-0
                            port:
                              number: 80
                        path: /path
                        pathType: ImplementationSpecific
                  - host: example-1.com
                    http:
                      paths:
                      - backend:
                          service:
                            name: service-1
                            port:
                              number: 80
                        path: /path
                        pathType: ImplementationSpecific
                  tls:
                  - hosts:
                    - example.com
                    secretName: secret-name
                status:
                  loadBalancer: {}
                "#
            })
            .unwrap()
        }

        #[test]
        fn 必要な情報のみを抽出してserviceを返す() {
            let actual = ingress().extract();

            let expected = serde_yaml::from_str(indoc! {
                r#"
                apiVersion: networking.k8s.io/v1
                kind: Ingress
                metadata:
                  annotations:
                  name: ingress
                spec:
                  rules:
                  - host: example-0.com
                    http:
                      paths:
                      - backend:
                          service:
                            name: service-0
                            port:
                              number: 80
                        path: /path
                        pathType: ImplementationSpecific
                  - host: example-1.com
                    http:
                      paths:
                      - backend:
                          service:
                            name: service-1
                            port:
                              number: 80
                        path: /path
                        pathType: ImplementationSpecific
                  tls:
                  - hosts:
                    - example.com
                    secretName: secret-name
                status:
                  loadBalancer: {}
                "#
            })
            .unwrap();

            assert_eq!(actual, expected);
        }
    }
}
