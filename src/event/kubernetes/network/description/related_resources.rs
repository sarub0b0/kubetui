#![allow(dead_code)]
#![allow(unused_imports)]

use anyhow::Result;
use k8s_openapi::{List, ListableResource};
use kube::Resource;
use serde::de::DeserializeOwned;
use serde_yaml::Value;

use crate::event::kubernetes::client::KubeClientRequest;

use self::{
    fetch::FetchClient,
    to_list_value::{ResourceList, ToListValue},
};

pub mod ingress;
pub mod network_policy;
pub mod pod;
pub mod service;

pub struct RelatedClient<'a, C: KubeClientRequest>(FetchClient<'a, C>);

pub trait Filter<I> {
    type Filtered;

    fn filter_by_item(&self, arg: &I) -> Option<List<Self::Filtered>>
    where
        Self::Filtered: ListableResource;
}

impl<'a, C: KubeClientRequest> RelatedClient<'a, C> {
    pub fn new(client: &'a C, namespace: &'a str) -> Self {
        Self(FetchClient::new(client, namespace))
    }

    pub async fn related_resources<K, I>(&self, item: &I) -> Result<Option<List<K>>>
    where
        K: Resource<DynamicType = ()> + ListableResource + DeserializeOwned + 'static,
        I: Sync,
        List<K>: Filter<I, Filtered = K> + ToListValue,
    {
        let list = self.0.fetch().await?;

        Ok(list.filter_by_item(item))
    }
}

pub mod label_selector {
    use std::collections::BTreeMap;

    use k8s_openapi::apimachinery::pkg::apis::meta::v1::{LabelSelector, LabelSelectorRequirement};

    use super::btree_map_contains_key_values::BTreeMapContains;

    #[derive(Debug, Default)]
    pub struct LabelSelectorWrapper(LabelSelector);

    impl LabelSelectorWrapper {
        pub fn new(label_selector: LabelSelector) -> Self {
            Self(label_selector)
        }
    }

    impl From<BTreeMap<String, String>> for LabelSelectorWrapper {
        fn from(match_labels: BTreeMap<String, String>) -> Self {
            Self(LabelSelector {
                match_labels: Some(match_labels),
                ..Default::default()
            })
        }
    }

    impl From<Vec<LabelSelectorRequirement>> for LabelSelectorWrapper {
        fn from(match_expressions: Vec<LabelSelectorRequirement>) -> Self {
            Self(LabelSelector {
                match_expressions: Some(match_expressions),
                ..Default::default()
            })
        }
    }

    impl From<LabelSelector> for LabelSelectorWrapper {
        fn from(label_selector: LabelSelector) -> Self {
            Self(label_selector)
        }
    }

    pub trait LabelSelectorExpression {
        fn expression(&self, labels: &BTreeMap<String, String>) -> bool;
    }

    impl LabelSelectorExpression for LabelSelectorWrapper {
        fn expression(&self, labels: &BTreeMap<String, String>) -> bool {
            let requirements = {
                let mut ret = vec![];
                if let Some(match_labels) = &self.0.match_labels {
                    for (k, v) in match_labels {
                        ret.push(LabelSelectorRequirement {
                            key: k.to_string(),
                            operator: "In".to_string(),
                            values: Some(vec![v.to_string()]),
                        });
                    }
                }

                if let Some(match_expressions) = &self.0.match_expressions {
                    ret.extend(match_expressions.clone());
                }

                ret
            };

            requirements
                .iter()
                .all(|requirement| match requirement.operator.as_str() {
                    // A In [B, ..]
                    // Aの値が[B, ..]のいずれか1つ以上と一致する場合にtrue
                    "In" => requirement.values.as_ref().map_or(false, |values| {
                        values.iter().any(|value| {
                            let r = BTreeMap::from([(requirement.key.clone(), value.clone())]);

                            labels.contains_key_values(&r)
                        })
                    }),
                    // A NotIn [B, ..]
                    // Aの値が[B, ..]のいずれとも一致しない場合にtrue
                    "NotIn" => requirement.values.as_ref().map_or(false, |values| {
                        values.iter().all(|value| {
                            let r = BTreeMap::from([(requirement.key.clone(), value.clone())]);

                            !labels.contains_key_values(&r)
                        })
                    }),
                    // A Exists []
                    // Aが存在する場合にtrue
                    "Exists" => labels.contains_key(&requirement.key),
                    // A DoesNotExist []
                    // Aが存在しない場合にtrue
                    "DoesNotExist" => !labels.contains_key(&requirement.key),
                    _ => {
                        unreachable!()
                    }
                })
        }
    }

    #[cfg(test)]
    #[allow(clippy::bool_assert_comparison)]
    mod tests {
        use super::*;

        mod operator_in {
            use super::*;

            #[test]
            fn labelsにkey_valueが存在するときtrueを返す() {
                let expr = LabelSelectorWrapper::from(vec![LabelSelectorRequirement {
                    key: "a".into(),
                    operator: "In".into(),
                    values: Some(vec!["b".into(), "c".into()]),
                }]);

                let labels = BTreeMap::from([("a".into(), "b".into()), ("c".into(), "d".into())]);

                assert_eq!(expr.expression(&labels), true)
            }

            #[test]
            fn labelsにkey_valueが存在しないときfalseを返す() {
                let expr = LabelSelectorWrapper::from(vec![LabelSelectorRequirement {
                    key: "a".into(),
                    operator: "In".into(),
                    values: Some(vec!["b".into(), "c".into()]),
                }]);

                let labels =
                    BTreeMap::from([("a".into(), "d".into()), ("aaa".into(), "dddd".into())]);

                assert_eq!(expr.expression(&labels), false)
            }
        }

        mod operator_not_int {
            use super::*;

            #[test]
            fn labelsにkey_valueが存在しないときtrueを返す() {
                let expr = LabelSelectorWrapper::from(vec![LabelSelectorRequirement {
                    key: "a".into(),
                    operator: "NotIn".into(),
                    values: Some(vec!["b".into(), "c".into()]),
                }]);

                let labels = BTreeMap::from([("a".into(), "d".into())]);

                assert_eq!(expr.expression(&labels), true)
            }

            #[test]
            fn labelsにkey_valueが存在するときfalseを返す() {
                let expr = LabelSelectorWrapper::from(vec![LabelSelectorRequirement {
                    key: "a".into(),
                    operator: "NotIn".into(),
                    values: Some(vec!["b".into(), "c".into()]),
                }]);

                let labels = BTreeMap::from([("a".into(), "b".into())]);

                assert_eq!(expr.expression(&labels), false)
            }
        }

        mod operator_exists {
            use super::*;

            #[test]
            fn labelsにkeyが存在するときtrueを返す() {
                let expr = LabelSelectorWrapper::from(vec![LabelSelectorRequirement {
                    key: "a".into(),
                    operator: "Exists".into(),
                    values: None,
                }]);

                let labels = BTreeMap::from([("a".into(), "".into()), ("b".into(), "".into())]);

                assert_eq!(expr.expression(&labels), true)
            }

            #[test]
            fn labelsにkeyが存在しないときfalseを返す() {
                let expr = LabelSelectorWrapper::from(vec![LabelSelectorRequirement {
                    key: "c".into(),
                    operator: "Exists".into(),
                    values: None,
                }]);

                let labels = BTreeMap::from([("a".into(), "".into()), ("b".into(), "".into())]);

                assert_eq!(expr.expression(&labels), false)
            }
        }

        mod operator_does_not_exist {
            use super::*;

            #[test]
            fn labelsにkeyが存在しないときtrueを返す() {
                let expr = LabelSelectorWrapper::from(vec![LabelSelectorRequirement {
                    key: "c".into(),
                    operator: "DoesNotExist".into(),
                    values: None,
                }]);

                let labels = BTreeMap::from([("a".into(), "".into()), ("b".into(), "".into())]);

                assert_eq!(expr.expression(&labels), true)
            }

            #[test]
            fn labelsにkeyが存在するときfalseを返す() {
                let expr = LabelSelectorWrapper::from(vec![LabelSelectorRequirement {
                    key: "a".into(),
                    operator: "DoesNotExist".into(),
                    values: None,
                }]);

                let labels = BTreeMap::from([("a".into(), "".into()), ("b".into(), "".into())]);

                assert_eq!(expr.expression(&labels), false)
            }
        }

        mod complex_operator {
            use super::*;
        }
    }
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

    #[allow(clippy::bool_assert_comparison)]
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

            serde_yaml::from_str(yaml).unwrap()
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

            serde_yaml::from_str(yaml).unwrap()
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

pub mod to_list_value {

    use k8s_openapi::{api::core::v1::Pod, List, ListableResource};
    use kube::ResourceExt;
    use serde_yaml::Value;

    pub trait ResourceList {
        type Value: ResourceExt + ListableResource;
        fn list(&self) -> &[Self::Value];
    }

    impl<K: ResourceExt + ListableResource> ResourceList for List<K> {
        type Value = K;

        fn list(&self) -> &[Self::Value] {
            &self.items
        }
    }

    pub trait ToListValue {
        fn to_list_value(&self) -> Option<Value>;
    }

    impl<R: ResourceList> ToListValue for R {
        fn to_list_value(&self) -> Option<Value> {
            let ret: Vec<Value> = self
                .list()
                .iter()
                .map(|r| Value::from(r.name_any()))
                .collect();
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

            serde_yaml::from_str::<List<Pod>>(yaml).unwrap()
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

            serde_yaml::from_str::<List<Service>>(yaml).unwrap()
        }

        #[test]
        fn podのリストからnameのリストをvalue型で返す() {
            let list = setup_pod_list();

            let actual = list.to_list_value();

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

            let actual = list.to_list_value();

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

            let actual = list.to_list_value();

            assert_eq!(actual, None)
        }
    }
}
