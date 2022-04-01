use anyhow::{Ok, Result};

use k8s_openapi::{api::networking::v1::Ingress, List};

pub mod filter_by_service {
    use super::*;

    use std::collections::BTreeMap;

    use kube::Resource;
    use serde_yaml::Value;

    use crate::event::kubernetes::{
        client::KubeClientRequest,
        network::description::related_resources::{
            btree_map_contains_key_values::BTreeMapContains, fetch::FetchClient, Filter,
            RelatedResources,
        },
    };

    pub struct RelatedIngress<'a, C: KubeClientRequest> {
        client: FetchClient<'a, C>,
        services: Vec<String>,
    }

    impl<'a, C: KubeClientRequest> RelatedIngress<'a, C> {
        pub fn new(client: &'a C, namespace: &'a str, services: Vec<String>) -> Self {
            Self {
                client: FetchClient::new(client, namespace),
                services,
            }
        }
    }

    #[async_trait::async_trait]
    impl<'a, C: KubeClientRequest> RelatedResources<C> for RelatedIngress<'a, C> {
        type Item = Vec<String>;
        type Filtered = Ingress;

        fn client(&self) -> &FetchClient<C> {
            &self.client
        }

        fn item(&self) -> &Self::Item {
            &self.services
        }
    }

    mod filter {
        use super::*;
        use std::collections::BTreeMap;

        use k8s_openapi::List;

        use crate::event::kubernetes::network::description::related_resources::{
            btree_map_contains_key_values::BTreeMapContains, Filter,
        };

        impl Filter<Vec<String>> for List<Ingress> {
            type Filtered = Ingress;

            fn filter_by_item(&self, arg: &Vec<String>) -> Option<List<Self::Filtered>> {
                let ret: Vec<Ingress> = self
                    .items
                    .iter()
                    .filter(|ing| {
                        ing.spec.as_ref().map_or(false, |spec| {
                            spec.rules.as_ref().map_or(false, |rules| {
                                rules.iter().any(|rule| {
                                    rule.http.as_ref().map_or(false, |http| {
                                        http.paths.iter().any(|path| {
                                            path.backend.service.as_ref().map_or(false, |service| {
                                                arg.iter().any(|arg_name| arg_name == &service.name)
                                            })
                                        })
                                    })
                                })
                            })
                        })
                    })
                    .cloned()
                    .collect();

                if !ret.is_empty() {
                    Some(List {
                        items: ret,
                        ..Default::default()
                    })
                } else {
                    None
                }
            }
        }

        #[cfg(test)]
        mod tests {
            use indoc::indoc;

            use super::*;

            use pretty_assertions::assert_eq;

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
                      - metadata:
                          name: ingress-3
                        spec:
                          rules:
                            - http:
                                paths:
                                  - backend:
                                      service:
                                        name: service-1
                                  - backend:
                                      service:
                                        name: service-3
                            - http:
                                paths:
                                  - backend:
                                      service:
                                        name: service-2
                    "
                };

                serde_yaml::from_str(&yaml).unwrap()
            }

            #[test]
            fn backend_serviceに指定されたservice名を含むときそのingressのリストを返す() {
                let services = vec!["service-1".into(), "service-2".into()];

                let list = ingresses();

                let actual = list.filter_by_item(&services);

                let expected = serde_yaml::from_str(indoc! {
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
                      - metadata:
                          name: ingress-3
                        spec:
                          rules:
                            - http:
                                paths:
                                  - backend:
                                      service:
                                        name: service-1
                                  - backend:
                                      service:
                                        name: service-3
                            - http:
                                paths:
                                  - backend:
                                      service:
                                        name: service-2

                    "
                })
                .unwrap();

                assert_eq!(actual, Some(expected));
            }

            #[test]
            fn backend_serviceに指定されたservice名を含まないときnoneを返す() {
                let services = vec!["hoge".into(), "fuga".into()];

                let list = ingresses();

                let actual = list.filter_by_item(&services);

                assert_eq!(actual.is_none(), true);
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        mod related_resources {
            use anyhow::bail;
            use indoc::indoc;
            use mockall::predicate::eq;

            use super::*;

            use crate::{event::kubernetes::client::mock::MockTestKubeClient, mock_expect};

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
                      - metadata:
                          name: ingress-3
                        spec:
                          rules:
                            - http:
                                paths:
                                  - backend:
                                      service:
                                        name: service-1
                                  - backend:
                                      service:
                                        name: service-3
                            - http:
                                paths:
                                  - backend:
                                      service:
                                        name: service-2
                    "
                };

                serde_yaml::from_str(&yaml).unwrap()
            }

            #[tokio::test]
            async fn service名リストのいずれかをbackend_serviceに含むingressのvalueを返す() {
                let mut client = MockTestKubeClient::new();

                mock_expect!(
                    client,
                    request,
                    List<Ingress>,
                    eq("/apis/networking.k8s.io/v1/namespaces/default/ingresses"),
                    Ok(ingresses())
                );

                let client = RelatedIngress::new(&client, "default", vec!["service-1".into()]);

                let result = client.related_resources().await.unwrap().unwrap();

                let expected = Value::from(vec!["ingress-1", "ingress-3"]);

                assert_eq!(result, expected);
            }

            #[tokio::test]
            async fn service名リストのいずれもbackend_serviceに含まないときnoneを返す() {
                let mut client = MockTestKubeClient::new();

                mock_expect!(
                    client,
                    request,
                    List<Ingress>,
                    eq("/apis/networking.k8s.io/v1/namespaces/default/ingresses"),
                    Ok(ingresses())
                );

                let client =
                    RelatedIngress::new(&client, "default", vec!["foo".into(), "bar".into()]);

                let result = client.related_resources().await.unwrap();

                assert_eq!(result.is_none(), true);
            }

            #[tokio::test]
            async fn エラーがでたときerrを返す() {
                let mut client = MockTestKubeClient::new();

                mock_expect!(
                    client,
                    request,
                    List<Ingress>,
                    eq("/apis/networking.k8s.io/v1/namespaces/default/ingresses"),
                    bail!("error")
                );

                let client = RelatedIngress::new(&client, "default", vec![]);

                let result = client.related_resources().await;

                assert_eq!(result.is_err(), true);
            }
        }
    }
}
