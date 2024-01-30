use std::collections::BTreeMap;

use k8s_openapi::{api::core::v1::Service, List};
use kube::ResourceExt;

use crate::workers::kube::client::KubeClientRequest;

use super::{
    btree_map_contains_key_values::BTreeMapContains, fetch::FetchClient, Filter, RelatedClient,
};

impl Filter<Vec<String>> for List<Service> {
    type Filtered = Service;

    fn filter_by_item(&self, arg: &Vec<String>) -> Option<List<Self::Filtered>>
    where
        Self::Filtered: k8s_openapi::ListableResource,
    {
        let ret: Vec<Service> = self
            .items
            .iter()
            .filter(|svc| arg.iter().any(|name| &svc.name_any() == name))
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

impl Filter<BTreeMap<String, String>> for List<Service> {
    type Filtered = Service;

    fn filter_by_item(&self, arg: &BTreeMap<String, String>) -> Option<List<Self::Filtered>>
    where
        Self::Filtered: k8s_openapi::ListableResource,
    {
        let ret: Vec<Service> = self
            .items
            .iter()
            .filter(|svc| {
                if let Some(spec) = &svc.spec {
                    if let Some(selector) = &spec.selector {
                        return arg.contains_key_values(selector);
                    }
                }
                false
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

    mod filter_by_names {
        use super::*;

        mod filter {
            use super::*;
            use indoc::indoc;
            use pretty_assertions::assert_eq;

            fn services() -> List<Service> {
                let yaml = indoc! {
                    "
                    items:
                      - metadata:
                          name: service-1
                      - metadata:
                          name: service-2
                      - metadata:
                          name: service-3
                    "
                };

                serde_yaml::from_str(yaml).unwrap()
            }

            #[test]
            fn namesに一致するserviceのリストを返す() {
                let arg = vec!["service-1".into(), "service-2".into()];

                let list = services();

                let actual = list.filter_by_item(&arg);

                let expected = serde_yaml::from_str(indoc! {
                    "
                    items:
                      - metadata:
                          name: service-1
                      - metadata:
                          name: service-2
                    "
                })
                .unwrap();

                assert_eq!(actual, Some(expected))
            }

            #[test]
            fn namesに一致するserviceがないときnoneを返す() {
                let arg = vec!["hoge".into()];

                let list = services();

                let actual = list.filter_by_item(&arg);

                assert_eq!(actual.is_none(), true)
            }
        }

        mod related_resources {
            use crate::{mock_expect, workers::kube::client::mock::MockTestKubeClient};

            use super::*;

            use anyhow::bail;
            use indoc::indoc;
            use mockall::predicate::eq;
            use pretty_assertions::assert_eq;

            fn services() -> List<Service> {
                let yaml = indoc! {
                    "
                    items:
                      - metadata:
                          name: service-1
                      - metadata:
                          name: service-2
                      - metadata:
                          name: service-3
                    "
                };

                serde_yaml::from_str(yaml).unwrap()
            }

            #[tokio::test]
            async fn nameリストに含まれるservice名のリストを返す() {
                let mut client = MockTestKubeClient::new();

                mock_expect!(
                    client,
                    request,
                    List<Service>,
                    eq("/api/v1/namespaces/default/services"),
                    Ok(services())
                );

                let client = RelatedClient::new(&client, "default");

                let result = client
                    .related_resources::<Service, _>(&vec!["service-1".into(), "service-3".into()])
                    .await
                    .unwrap()
                    .unwrap();

                let expected = serde_yaml::from_str(indoc! {
                    "
                    items:
                      - metadata:
                          name: service-1
                      - metadata:
                          name: service-3
                    "
                })
                .unwrap();

                assert_eq!(result, expected);
            }

            #[tokio::test]
            async fn nameリストに含まれるserviceがないときnoneを返す() {
                let mut client = MockTestKubeClient::new();

                mock_expect!(
                    client,
                    request,
                    List<Service>,
                    eq("/api/v1/namespaces/default/services"),
                    Ok(services())
                );

                let client = RelatedClient::new(&client, "default");

                let result = client
                    .related_resources::<Service, _>(&vec!["hoge".into()])
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
                    List<Service>,
                    eq("/api/v1/namespaces/default/services"),
                    bail!("error")
                );

                let client = RelatedClient::new(&client, "default");

                let result = client
                    .related_resources::<Service, _>(&vec!["service-1".into()])
                    .await;

                assert_eq!(result.is_err(), true);
            }
        }
    }

    mod filter_by_selector {
        use super::*;

        mod filter {
            use super::*;

            use indoc::indoc;
            use pretty_assertions::assert_eq;

            fn services() -> List<Service> {
                let yaml = indoc! {
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
                };

                serde_yaml::from_str(yaml).unwrap()
            }

            #[test]
            fn labelsにselectorの値を含むときそのserviceのリストを返す() {
                let arg = BTreeMap::from([
                    ("version".into(), "v1".into()),
                    ("app".into(), "pod-1".into()),
                ]);

                let list = services();

                let actual = list.filter_by_item(&arg);

                let expected = serde_yaml::from_str(indoc! {
                    "
                    items:
                      - metadata:
                          name: service-1
                        spec:
                          selector:
                            app: pod-1
                            version: v1
                    "
                })
                .unwrap();

                assert_eq!(actual, Some(expected))
            }

            #[test]
            fn labelsにselectorの値を含まないときnoneを返す() {
                let arg = BTreeMap::from([("version".into(), "v1".into())]);

                let list = services();

                let actual = list.filter_by_item(&arg);

                assert_eq!(actual.is_none(), true)
            }
        }

        mod related_resources {
            use anyhow::bail;
            use indoc::indoc;
            use k8s_openapi::{api::core::v1::Service, List};
            use mockall::predicate::eq;
            use serde_yaml::Value;

            use crate::{mock_expect, workers::kube::client::mock::MockTestKubeClient};

            use super::*;

            fn services() -> List<Service> {
                let yaml = indoc! {
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
                };

                serde_yaml::from_str(yaml).unwrap()
            }

            #[tokio::test]
            async fn labelsリストに含まれるservice名のリストを返す() {
                let mut client = MockTestKubeClient::new();

                mock_expect!(
                    client,
                    request,
                    List<Service>,
                    eq("/api/v1/namespaces/default/services"),
                    Ok(services())
                );

                let labels = BTreeMap::from([
                    ("version".to_string(), "v1".to_string()),
                    ("app".to_string(), "pod-1".to_string()),
                ]);

                let client = RelatedClient::new(&client, "default");

                let result = client
                    .related_resources::<Service, _>(&labels)
                    .await
                    .unwrap()
                    .unwrap();

                let expected = serde_yaml::from_str(indoc! {
                    "
                    items:
                      - metadata:
                          name: service-1
                        spec:
                          selector:
                            app: pod-1
                            version: v1
                    "
                })
                .unwrap();

                assert_eq!(result, expected);
            }

            #[tokio::test]
            async fn labelsリストに含まれるserviceがないときnoneを返す() {
                let mut client = MockTestKubeClient::new();

                mock_expect!(
                    client,
                    request,
                    List<Service>,
                    eq("/api/v1/namespaces/default/services"),
                    Ok(services())
                );

                let labels = BTreeMap::from([("foo".to_string(), "bar".to_string())]);

                let client = RelatedClient::new(&client, "default");

                let result = client
                    .related_resources::<Service, _>(&labels)
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
                    List<Service>,
                    eq("/api/v1/namespaces/default/services"),
                    bail!("error")
                );

                let labels = BTreeMap::from([("version".to_string(), "v1".to_string())]);

                let client = RelatedClient::new(&client, "default");

                let result = client.related_resources::<Service, _>(&labels).await;

                assert_eq!(result.is_err(), true);
            }
        }
    }
}
