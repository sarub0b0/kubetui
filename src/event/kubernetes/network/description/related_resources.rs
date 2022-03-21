#![allow(dead_code)]
#![allow(unused_imports)]

mod pod {
    use std::collections::BTreeMap;

    use anyhow::{Ok, Result};
    use k8s_openapi::{api::core::v1::Pod, List};
    use serde_yaml::Value;

    use crate::event::kubernetes::client::KubeClientRequest;

    trait ToValue {
        fn to_value(&self) -> Option<Value>;
    }

    impl ToValue for FetchedPodList {
        fn to_value(&self) -> Option<Value> {
            todo!()
        }
    }

    type FetchedPodList = List<Pod>;

    use fetch::{Fetch, FetchPodClient};

    struct RelatedPod<'a, F: Fetch> {
        client: &'a F,
        selector: BTreeMap<&'a str, &'a str>,
    }

    impl<'a, F: Fetch> RelatedPod<'a, F> {
        fn new(client: &'a F, selector: BTreeMap<&'a str, &'a str>) -> Self {
            Self { client, selector }
        }
    }

    impl<'a, F: Fetch> RelatedPod<'a, F> {
        async fn related_resources(&self) -> Result<Option<Value>> {
            let list = self.client.fetch().await?;

            if let Some(filter) = self.filter(&list) {
                Ok(filter.to_value())
            } else {
                Ok(None)
            }
        }

        fn filter(&self, list: &FetchedPodList) -> Option<FetchedPodList> {
            todo!()
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

            use crate::{
                event::kubernetes::{
                    client::mock::MockTestKubeClient,
                    network::description::related_resources::pod::fetch::mock::MockTestFetchPodClient,
                },
                mock_expect,
            };

            use self::Fetch;
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
                let mut client = MockTestFetchPodClient::new();

                client.expect_fetch().returning(|| Ok(setup_pod()));

                let selector = BTreeMap::from([("version", "v1")]);

                let client = RelatedPod {
                    client: &client,
                    selector,
                };

                let result = client.related_resources().await.unwrap().unwrap();

                let expected = Value::from(vec!["pod-1", "pod-2"]);

                assert_eq!(result, expected);
            }

            #[tokio::test]
            async fn 関連するpodがないときnoneを返す() {
                let mut client = MockTestFetchPodClient::new();

                client.expect_fetch().returning(|| Ok(setup_pod()));

                let selector = BTreeMap::from([("hoge", "fuga")]);

                let client = RelatedPod {
                    client: &client,
                    selector,
                };

                let result = client.related_resources().await.unwrap();

                assert_eq!(result.is_none(), true);
            }

            #[tokio::test]
            async fn エラーがでたときerrを返す() {
                let mut client = MockTestFetchPodClient::new();

                client.expect_fetch().returning(|| Ok(setup_pod()));

                let client = RelatedPod {
                    client: &client,
                    selector: BTreeMap::new(),
                };

                let result = client.related_resources().await;

                assert_eq!(result.is_err(), true);
            }
        }

        mod filter {
            use super::*;

            #[ignore]
            #[test]
            fn 関連するpodのリストを生成する() {}

            #[ignore]
            #[test]
            fn 関連するpodがないときnoneを返す() {}
        }

        mod to_value {
            use super::*;

            #[ignore]
            #[test]
            fn podのリストからnameのリストをvalue型で返す() {}

            #[ignore]
            #[test]
            fn リストが空のときnoneを返す() {}
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

        #[async_trait::async_trait]
        pub trait Fetch {
            async fn fetch(&self) -> Result<FetchedPodList>;
        }

        #[async_trait::async_trait]
        impl<'a, C: KubeClientRequest> Fetch for FetchPodClient<'_, C> {
            async fn fetch(&self) -> Result<FetchedPodList> {
                let url = format!("api/v1/namespaces/{}/pods", self.namespace);

                self.client.request(&url).await
            }
        }

        #[cfg(test)]
        pub mod mock {
            use super::{Fetch, FetchedPodList, KubeClientRequest, Result};

            use mockall::mock;

            mock! {
                pub TestFetchPodClient {}

                impl Clone for TestFetchPodClient {
                    fn clone(&self) -> Self ;
                }

                #[async_trait::async_trait]
                impl Fetch for TestFetchPodClient {
                    async fn fetch(&self) -> Result<FetchedPodList> ;
                }
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
}
