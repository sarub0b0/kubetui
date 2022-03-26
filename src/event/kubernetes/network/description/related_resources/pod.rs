pub mod filter_by_labels {

    use std::collections::BTreeMap;

    use anyhow::{Ok, Result};
    use k8s_openapi::{api::core::v1::Pod, List};
    use serde_yaml::Value;

    use crate::event::kubernetes::{
        client::KubeClientRequest, network::description::related_resources::to_value::ToValue,
    };

    use fetch::FetchPodClient;

    use self::filter::Filter;

    type FetchedPodList = List<Pod>;

    struct RelatedPod<'a, C: KubeClientRequest> {
        client: FetchPodClient<'a, C>,
        selector: BTreeMap<String, String>,
    }

    impl<'a, C: KubeClientRequest> RelatedPod<'a, C> {
        fn new(client: &'a C, namespace: &'a str, selector: BTreeMap<String, String>) -> Self {
            Self {
                client: FetchPodClient::new(client, namespace),
                selector,
            }
        }
    }

    impl<'a, C: KubeClientRequest> RelatedPod<'a, C> {
        async fn related_resources(&self) -> Result<Option<Value>> {
            let list = self.client.fetch().await?;

            if let Some(filtered) = list.filter_by_labels(&self.selector) {
                Ok(filtered.to_value())
            } else {
                Ok(None)
            }
        }
    }

    mod fetch {
        use crate::event::kubernetes::client::KubeClientRequest;

        use anyhow::Result;

        use super::FetchedPodList;

        pub struct FetchPodClient<'a, C: KubeClientRequest> {
            client: &'a C,
            namespace: &'a str,
        }

        impl<'a, C: KubeClientRequest> FetchPodClient<'a, C> {
            pub fn new(client: &'a C, namespace: &'a str) -> Self {
                Self { client, namespace }
            }
        }

        impl<'a, C: KubeClientRequest> FetchPodClient<'_, C> {
            pub async fn fetch(&self) -> Result<FetchedPodList> {
                let url = format!("api/v1/namespaces/{}/pods", self.namespace);

                self.client.request(&url).await
            }
        }

        #[cfg(test)]
        mod tests {

            use indoc::indoc;
            use mockall::predicate::eq;

            use crate::{event::kubernetes::client::mock::MockTestKubeClient, mock_expect};

            use anyhow::bail;

            use super::*;

            fn pod_one() -> FetchedPodList {
                let yaml = indoc! {
                "
                items:
                  - metadata:
                    name: pod-1
                    labels:
                      app: pod-1
                "
                };

                serde_yaml::from_str::<FetchedPodList>(&yaml).unwrap()
            }

            fn pod_two() -> FetchedPodList {
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

                serde_yaml::from_str::<FetchedPodList>(&yaml).unwrap()
            }

            #[tokio::test]
            async fn podリストを取得する() {
                let mut client = MockTestKubeClient::new();

                mock_expect!(
                    client,
                    request,
                    FetchedPodList,
                    eq("api/v1/namespaces/default/pods"),
                    Ok(pod_one())
                );

                let client = FetchPodClient::new(&client, "default");

                let result = client.fetch().await;

                assert_eq!(result.is_ok(), true);
            }

            #[tokio::test]
            async fn エラーのときerrを返す() {
                let mut client = MockTestKubeClient::new();

                mock_expect!(
                    client,
                    request,
                    FetchedPodList,
                    eq("api/v1/namespaces/default/pods"),
                    bail!("error")
                );

                let client = FetchPodClient::new(&client, "default");

                let result = client.fetch().await;

                assert_eq!(result.is_err(), true);
            }
        }
    }

    mod filter {
        use std::collections::BTreeMap;

        use k8s_openapi::api::core::v1::Pod;

        use super::FetchedPodList;

        pub trait Filter {
            fn filter_by_labels(
                &self,
                selector: &BTreeMap<String, String>,
            ) -> Option<FetchedPodList>;
        }

        impl<'a> Filter for FetchedPodList {
            fn filter_by_labels(
                &self,
                selector: &BTreeMap<String, String>,
            ) -> Option<FetchedPodList> {
                let ret: Vec<Pod> = self
                    .items
                    .iter()
                    .filter(|item| {
                        item.metadata
                            .labels
                            .as_ref()
                            .map_or(false, |pod_labels| compare_btree_map(selector, pod_labels))
                    })
                    .cloned()
                    .collect();

                if !ret.is_empty() {
                    Some(FetchedPodList {
                        items: ret,
                        ..Default::default()
                    })
                } else {
                    None
                }
            }
        }

        fn compare_btree_map<K, V>(lhs: &BTreeMap<K, V>, rhs: &BTreeMap<K, V>) -> bool
        where
            K: Ord,
            V: PartialEq,
        {
            lhs.iter().all(|(lhs_key, lhs_value)| {
                rhs.get(lhs_key)
                    .map_or(false, |rhs_value| lhs_value == rhs_value)
            })
        }

        #[cfg(test)]
        mod tests {
            use indoc::indoc;

            use super::*;

            use pretty_assertions::assert_eq;

            fn setup_target() -> FetchedPodList {
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

                serde_yaml::from_str::<FetchedPodList>(&yaml).unwrap()
            }

            #[test]
            fn 値にマッチしたときそのリストを返す() {
                let selector = BTreeMap::from([("app".into(), "pod-1".into())]);

                let list = setup_target();

                let actual = list.filter_by_labels(&selector);

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
            fn 値にマッチする値がないときnoneを返す() {
                let selector = BTreeMap::from([("hoge".into(), "fuga".into())]);

                let list = setup_target();

                let actual = list.filter_by_labels(&selector);

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

            fn setup_pod() -> FetchedPodList {
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

                serde_yaml::from_str::<FetchedPodList>(&yaml).unwrap()
            }

            #[tokio::test]
            async fn 関連するpodのvalueを返す() {
                let mut client = MockTestKubeClient::new();

                mock_expect!(
                    client,
                    request,
                    FetchedPodList,
                    eq("api/v1/namespaces/default/pods"),
                    Ok(setup_pod())
                );

                let selector = BTreeMap::from([("version".into(), "v1".into())]);

                let client = RelatedPod::new(&client, "default", selector);

                let result = client.related_resources().await.unwrap().unwrap();

                let expected = Value::from(vec!["pod-1", "pod-2"]);

                assert_eq!(result, expected);
            }

            #[tokio::test]
            async fn 関連するpodがないときnoneを返す() {
                let mut client = MockTestKubeClient::new();

                mock_expect!(
                    client,
                    request,
                    FetchedPodList,
                    eq("api/v1/namespaces/default/pods"),
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
                    FetchedPodList,
                    eq("api/v1/namespaces/default/pods"),
                    bail!("error")
                );

                let client = RelatedPod::new(&client, "default", BTreeMap::default());

                let result = client.related_resources().await;

                assert_eq!(result.is_err(), true);
            }
        }
    }
}
