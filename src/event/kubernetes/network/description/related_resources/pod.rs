use anyhow::{Ok, Result};

use k8s_openapi::{api::core::v1::Pod, List};

pub mod filter_by_labels {
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

    pub struct RelatedPod<'a, C: KubeClientRequest> {
        client: FetchClient<'a, C>,
        selector: BTreeMap<String, String>,
    }

    impl<'a, C: KubeClientRequest> RelatedPod<'a, C> {
        pub fn new(client: &'a C, namespace: &'a str, selector: BTreeMap<String, String>) -> Self {
            Self {
                client: FetchClient::new(client, namespace),
                selector,
            }
        }
    }

    #[async_trait::async_trait]
    impl<'a, C: KubeClientRequest> RelatedResources<C> for RelatedPod<'a, C> {
        type Item = BTreeMap<String, String>;
        type Filtered = Pod;

        fn client(&self) -> &FetchClient<C> {
            &self.client
        }

        fn item(&self) -> &Self::Item {
            &self.selector
        }
    }

    mod filter {
        use std::collections::BTreeMap;

        use k8s_openapi::{api::core::v1::Pod, List};

        use crate::event::kubernetes::network::description::related_resources::{
            btree_map_contains_key_values::BTreeMapContains, Filter,
        };

        impl Filter<BTreeMap<String, String>> for List<Pod> {
            type Filtered = Pod;

            fn filter_by_item(
                &self,
                arg: &BTreeMap<String, String>,
            ) -> Option<List<Self::Filtered>> {
                let ret: Vec<Pod> = self
                    .items
                    .iter()
                    .filter(|item| {
                        item.metadata
                            .labels
                            .as_ref()
                            .map_or(false, |pod_labels| pod_labels.contains_key_values(arg))
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

            fn setup_target() -> List<Pod> {
                let yaml = indoc! {
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
                    "
                };

                serde_yaml::from_str(&yaml).unwrap()
            }

            #[test]
            fn labelsにselectorの値を含むときそのpodのリストを返す() {
                let selector = BTreeMap::from([("app".into(), "pod-1".into())]);

                let list = setup_target();

                let actual = list.filter_by_item(&selector);

                let expected = serde_yaml::from_str(indoc! {
                    "
                    items:
                      - metadata:
                          name: pod-1
                          labels:
                            app: pod-1
                            version: v1
                    "
                })
                .unwrap();

                assert_eq!(actual, Some(expected));
            }

            #[test]
            fn labelsにselectorの値を含むpodがないときnoneを返す() {
                let selector = BTreeMap::from([("hoge".into(), "fuga".into())]);

                let list = setup_target();

                let actual = list.filter_by_item(&selector);

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

            fn setup_pod() -> List<Pod> {
                let yaml = indoc! {
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
                "
                };

                serde_yaml::from_str(&yaml).unwrap()
            }

            #[tokio::test]
            async fn selectorの対象になるpodのvalueを返す() {
                let mut client = MockTestKubeClient::new();

                mock_expect!(
                    client,
                    request,
                    List<Pod>,
                    eq("/api/v1/namespaces/default/pods"),
                    Ok(setup_pod())
                );

                let selector = BTreeMap::from([("version".into(), "v1".into())]);

                let client = RelatedPod::new(&client, "default", selector);

                let result = client.related_resources().await.unwrap().unwrap();

                let expected = serde_yaml::from_str(indoc! {
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
                    "
                })
                .unwrap();

                assert_eq!(result, expected);
            }

            #[tokio::test]
            async fn selectorの対象になるpodがないときnoneを返す() {
                let mut client = MockTestKubeClient::new();

                mock_expect!(
                    client,
                    request,
                    List<Pod>,
                    eq("/api/v1/namespaces/default/pods"),
                    Ok(setup_pod())
                );

                let selector = BTreeMap::from([("hoge".into(), "fuga".into())]);

                let client = RelatedPod::new(&client, "default", selector);

                let result = client.related_resources().await.unwrap();

                assert_eq!(result.is_none(), true);
            }

            #[tokio::test]
            async fn エラーがでたときerrを返す() {
                let mut client = MockTestKubeClient::new();

                mock_expect!(
                    client,
                    request,
                    List<Pod>,
                    eq("/api/v1/namespaces/default/pods"),
                    bail!("error")
                );

                let client = RelatedPod::new(&client, "default", BTreeMap::default());

                let result = client.related_resources().await;

                assert_eq!(result.is_err(), true);
            }
        }
    }
}
