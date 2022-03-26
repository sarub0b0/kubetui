#![allow(dead_code)]
#![allow(unused_imports)]

use anyhow::Result;
use serde_yaml::Value;

pub mod pod;

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
