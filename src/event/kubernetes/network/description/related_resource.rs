#![allow(dead_code)]
#![allow(unused_imports)]

mod pod {
    use std::collections::BTreeMap;

    use anyhow::Result;
    use k8s_openapi::{api::core::v1::Pod, List};

    type FetchedPodList = List<Pod>;

    struct FetchPodClient<'a, C> {
        client: &'a C,
        namespace: &'a str,
        selector: BTreeMap<&'a str, &'a str>,
    }

    impl<'a, C> FetchPodClient<'a, C> {
        fn new(client: &'a C, namespace: &'a str, selector: BTreeMap<&'a str, &'a str>) -> Self {
            Self {
                client,
                namespace,
                selector,
            }
        }

        async fn fetch(&self) -> Result<FetchedPodList> {
            unimplemented!()
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        mod fetch {
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
                    [(
                        FetchedPodList,
                        eq("api/v1/namespaces/default/pods"),
                        Ok(pod_one())
                    )]
                );

                let selector = BTreeMap::from([("app", "test"), ("version", "v1")]);

                let client = FetchPodClient::new(&client, "default", selector);

                let result = client.fetch().await;

                assert_eq!(result.is_ok(), true);
            }

            #[tokio::test]
            async fn エラーのときerrを返す() {
                let mut client = MockTestKubeClient::new();

                mock_expect!(
                    client,
                    request,
                    [(
                        FetchedPodList,
                        eq("api/v1/namespaces/default/pods"),
                        bail!("error")
                    )]
                );

                let selector = BTreeMap::from([("app", "test"), ("version", "v1")]);

                let client = FetchPodClient::new(&client, "default", selector);

                let result = client.fetch().await;

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
}
