use anyhow::{Ok, Result};

use k8s_openapi::{api::networking::v1::Ingress, List};

use std::collections::BTreeMap;

use kube::Resource;
use serde_yaml::Value;

use crate::workers::kube::client::KubeClientRequest;

use super::{btree_map_contains_key_values::BTreeMapContains, Filter, RelatedClient};

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

#[allow(clippy::bool_assert_comparison)]
#[cfg(test)]
mod tests {
    use super::*;

    mod filter {
        use super::*;

        use indoc::indoc;
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

            serde_yaml::from_str(yaml).unwrap()
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

    mod related_resources {
        use super::*;

        use anyhow::bail;
        use indoc::indoc;
        use mockall::predicate::eq;

        use crate::{mock_expect, workers::kube::client::mock::MockTestKubeClient};

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

            serde_yaml::from_str(yaml).unwrap()
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

            let client = RelatedClient::new(&client, "default");

            let result = client
                .related_resources::<Ingress, Vec<String>>(&vec!["service-1".into()])
                .await
                .unwrap()
                .unwrap();

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

            let client = RelatedClient::new(&client, "default");

            let result = client
                .related_resources::<Ingress, Vec<String>>(&vec!["foo".into(), "bar".into()])
                .await
                .unwrap();

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

            let client = RelatedClient::new(&client, "default");

            let result = client
                .related_resources::<Ingress, Vec<String>>(&vec![])
                .await;

            assert_eq!(result.is_err(), true);
        }
    }
}
