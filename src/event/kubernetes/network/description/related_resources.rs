#![allow(dead_code)]
#![allow(unused_imports)]

mod pod {
    use std::collections::BTreeMap;

    use anyhow::Result;
    use k8s_openapi::{api::core::v1::Pod, List};

    use crate::event::kubernetes::client::KubeClientRequest;

    type FetchedPodList = List<Pod>;

    struct FetchPodClient<'a, C: KubeClientRequest> {
        client: &'a C,
        namespace: &'a str,
        selector: BTreeMap<&'a str, &'a str>,
    }

    impl<'a, C: KubeClientRequest> FetchPodClient<'a, C> {
        fn new(client: &'a C, namespace: &'a str, selector: BTreeMap<&'a str, &'a str>) -> Self {
            Self {
                client,
                namespace,
                selector,
            }
        }

        async fn fetch(&self) -> Result<FetchedPodList> {
            let url = format!("api/v1/namespaces/{}/pods", self.namespace);

            self.client.request(&url).await
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
                let selector = BTreeMap::from([("version", "v1")]);

                mock_expect!(
                    client,
                    request,
                    FetchedPodList,
                    eq("api/v1/namespaces/default/pods"),
                    Ok(setup_pod())
                );

                let client = FetchPodClient::new(&client, "default", selector);

                let result = client.related_resources().await.unwrap().unwrap();

                let expected = Value::from(vec!["pod-1", "pod-2"]);

                assert_eq!(result, expected);
            }

            #[tokio::test]
            async fn 関連するpodがないときnoneを返す() {
                let mut client = MockTestKubeClient::new();

                let selector = BTreeMap::from([("hoge", "fuga")]);

                mock_expect!(
                    client,
                    request,
                    FetchedPodList,
                    eq("api/v1/namespaces/default/pods"),
                    Ok(setup_pod())
                );

                let client = FetchPodClient::new(&client, "default", selector);

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

                let client = FetchPodClient::new(&client, "default", BTreeMap::default());

                let result = client.related_resources().await;

                assert_eq!(result.is_err(), true);
            }
        }

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
                    FetchedPodList,
                    eq("api/v1/namespaces/default/pods"),
                    Ok(pod_one())
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
                    FetchedPodList,
                    eq("api/v1/namespaces/default/pods"),
                    bail!("error")
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

