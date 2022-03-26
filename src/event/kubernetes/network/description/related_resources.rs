#![allow(dead_code)]
#![allow(unused_imports)]

use anyhow::Result;
use serde_yaml::Value;

pub mod pod;
pub mod service;

trait RelatedResources {
    fn related_resources(&self) -> Result<Option<Value>>;
}

mod btree_map_contains_key_values {
    use std::collections::BTreeMap;

    pub trait BTreeMapContains<K: Ord, V: PartialEq> {
        fn contains_key_values(&self, rhs: &BTreeMap<K, V>) -> bool;
    }

    impl<K, V> BTreeMapContains<K, V> for BTreeMap<K, V>
    where
        K: Ord,
        V: PartialEq,
    {
        fn contains_key_values(&self, arg: &BTreeMap<K, V>) -> bool {
            arg.iter().all(|(arg_key, arg_value)| {
                self.get(arg_key)
                    .map_or(false, |self_value| self_value == arg_value)
            })
        }
    }

    #[cfg(test)]
    mod tests {
        use indoc::indoc;

        use super::*;

        use pretty_assertions::assert_eq;

        #[test]
        fn 引数の値をすべて含んでいたときtrueを返す() {
            let args: BTreeMap<&str, &str> = BTreeMap::from([("app", "pod-1"), ("version", "v1")]);

            let map = BTreeMap::from([("app", "pod-1"), ("version", "v1")]);

            let actual = map.contains_key_values(&args);

            assert_eq!(actual, true);
        }

        #[test]
        fn 引数の値をすべて含んでいないときfalseを返す() {
            let args: BTreeMap<&str, &str> = BTreeMap::from([("app", "pod-1"), ("version", "v1")]);

            let map = BTreeMap::from([("version", "v1")]);

            let actual = map.contains_key_values(&args);

            assert_eq!(actual, false);
        }
    }
}

mod fetch {
    use std::marker::PhantomData;

    use k8s_openapi::{List, ListableResource};
    use kube::Resource;
    use serde::de::DeserializeOwned;

    use super::*;

    use crate::event::kubernetes::client::KubeClientRequest;

    pub struct FetchClient<'a, C: KubeClientRequest> {
        client: &'a C,
        namespace: &'a str,
    }

    impl<'a, C: KubeClientRequest> FetchClient<'a, C> {
        pub fn new(client: &'a C, namespace: &'a str) -> Self {
            Self { client, namespace }
        }
    }

    impl<'a, C: KubeClientRequest> FetchClient<'_, C> {
        pub async fn fetch<K>(&self) -> Result<List<K>>
        where
            K: Resource<DynamicType = ()> + ListableResource,
            K: DeserializeOwned + 'static,
        {
            let url = K::url_path(&(), Some(self.namespace));

            self.client.request(&url).await
        }
    }

    #[cfg(test)]
    mod tests {

        use indoc::indoc;
        use k8s_openapi::api::core::v1::{Pod, Service};
        use mockall::predicate::eq;

        use crate::{event::kubernetes::client::mock::MockTestKubeClient, mock_expect};

        use anyhow::bail;

        use super::*;

        fn pod() -> List<Pod> {
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

        fn service() -> List<Service> {
            let yaml = indoc! {
            "
            items:
              - metadata:
                name: service-1
                labels:
                  app: service-1
                  version: v1
              - metadata:
                name: service-2
                labels:
                  app: service-2
                  version: v1
            "
            };

            serde_yaml::from_str(&yaml).unwrap()
        }

        #[tokio::test]
        async fn podリストを取得する() {
            let mut client = MockTestKubeClient::new();

            mock_expect!(
                client,
                request,
                List<Pod>,
                eq("/api/v1/namespaces/default/pods"),
                { Ok(pod()) }
            );

            let client = FetchClient::new(&client, "default");

            let result: Result<List<Pod>> = client.fetch().await;

            assert_eq!(result.is_ok(), true);
        }

        #[tokio::test]
        async fn serviceリストを取得する() {
            let mut client = MockTestKubeClient::new();

            mock_expect!(
                client,
                request,
                List<Service>,
                eq("/api/v1/namespaces/default/services"),
                { Ok(service()) }
            );

            let client = FetchClient::new(&client, "default");

            let result: Result<List<Service>> = client.fetch().await;

            assert_eq!(result.is_ok(), true);
        }

        #[tokio::test]
        async fn エラーのときerrを返す() {
            let mut client = MockTestKubeClient::new();

            mock_expect!(
                client,
                request,
                List<Pod>,
                eq("/api/v1/namespaces/default/pods"),
                bail!("error")
            );

            let client = FetchClient::new(&client, "default");

            let result: Result<List<Pod>> = client.fetch().await;

            assert_eq!(result.is_err(), true);
        }
    }
}

mod to_value {

    use k8s_openapi::{api::core::v1::Pod, List, ListableResource};
    use kube::ResourceExt;
    use serde_yaml::Value;

    pub trait ResourceList<K: ResourceExt + ListableResource> {
        fn list(&self) -> &[K];
    }

    impl<K: ResourceExt + ListableResource> ResourceList<K> for List<K> {
        fn list(&self) -> &[K] {
            &self.items
        }
    }

    pub trait ToValue<K: ResourceExt + ListableResource, R: ResourceList<K>> {
        fn to_value(&self) -> Option<Value>;
    }

    impl<K: ResourceExt + ListableResource, R: ResourceList<K>> ToValue<K, R> for R {
        fn to_value(&self) -> Option<Value> {
            let ret: Vec<Value> = self.list().iter().map(|r| Value::from(r.name())).collect();
            if !ret.is_empty() {
                Some(ret.into())
            } else {
                None
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        use indoc::indoc;
        use k8s_openapi::api::core::v1::Service;

        fn setup_pod_list() -> List<Pod> {
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

            serde_yaml::from_str::<List<Pod>>(&yaml).unwrap()
        }

        fn setup_service_list() -> List<Service> {
            let yaml = indoc! {
                "
                items:
                  - metadata:
                      name: service-1
                  - metadata:
                      name: service-2
                "
            };

            serde_yaml::from_str::<List<Service>>(&yaml).unwrap()
        }

        #[test]
        fn podのリストからnameのリストをvalue型で返す() {
            let list = setup_pod_list();

            let actual = list.to_value();

            let expected = serde_yaml::from_str(indoc! {
                "
                - pod-1
                - pod-2
                "
            })
            .unwrap();

            assert_eq!(actual, expected)
        }

        #[test]
        fn serviceのリストからnameのリストをvalue型で返す() {
            let list = setup_service_list();

            let actual = list.to_value();

            let expected = serde_yaml::from_str(indoc! {
                "
                - service-1
                - service-2
                "
            })
            .unwrap();

            assert_eq!(actual, expected)
        }

        #[test]
        fn リストが空のときnoneを返す() {
            let list = List::<Pod>::default();

            let actual = list.to_value();

            assert_eq!(actual, None)
        }
    }
}
