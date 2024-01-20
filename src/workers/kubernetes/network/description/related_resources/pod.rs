use anyhow::{Ok, Result};

use k8s_openapi::{api::core::v1::Pod, List};

use std::collections::BTreeMap;

use kube::Resource;
use serde_yaml::Value;

use crate::workers::kubernetes::client::KubeClientRequest;

use super::{
    btree_map_contains_key_values::BTreeMapContains,
    fetch::FetchClient,
    label_selector::{LabelSelectorExpression, LabelSelectorWrapper},
    Filter, RelatedClient,
};

impl Filter<BTreeMap<String, String>> for List<Pod> {
    type Filtered = Pod;

    fn filter_by_item(&self, arg: &BTreeMap<String, String>) -> Option<List<Self::Filtered>> {
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

impl Filter<Vec<BTreeMap<String, String>>> for List<Pod> {
    type Filtered = Pod;

    fn filter_by_item(&self, arg: &Vec<BTreeMap<String, String>>) -> Option<List<Self::Filtered>> {
        let ret: Vec<Pod> = self
            .items
            .iter()
            .filter(|item| {
                item.metadata.labels.as_ref().map_or(false, |pod_labels| {
                    arg.iter().any(|arg| pod_labels.contains_key_values(arg))
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

impl Filter<LabelSelectorWrapper> for List<Pod> {
    type Filtered = Pod;
    fn filter_by_item(&self, arg: &LabelSelectorWrapper) -> Option<List<Self::Filtered>> {
        let ret: Vec<Pod> = self
            .items
            .iter()
            .filter(|item| {
                item.metadata
                    .labels
                    .as_ref()
                    .map_or(false, |pod_labels| arg.expression(pod_labels))
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

            serde_yaml::from_str(yaml).unwrap()
        }

        #[test]
        fn labelsにselectorの値を含むときそのpodのリストを返す() {
            let selectors = vec![
                BTreeMap::from([("app".into(), "pod-1".into())]),
                BTreeMap::from([("version".into(), "v1".into())]),
            ];

            let list = setup_target();

            let actual = list.filter_by_item(&selectors);

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

            assert_eq!(actual, Some(expected));
        }

        #[test]
        fn labelsにselectorの値を含むpodがないときnoneを返す() {
            let selectors = vec![BTreeMap::from([("hoge".into(), "fuga".into())])];

            let list = setup_target();

            let actual = list.filter_by_item(&selectors);

            assert_eq!(actual.is_none(), true);
        }
    }

    use super::*;

    mod related_resources {
        use anyhow::bail;
        use indoc::indoc;
        use mockall::predicate::eq;

        use super::*;

        use crate::{mock_expect, workers::kubernetes::client::mock::MockTestKubeClient};

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

            serde_yaml::from_str(yaml).unwrap()
        }

        #[tokio::test]
        async fn selectorの対象になるpodのリストを返す() {
            let mut client = MockTestKubeClient::new();

            mock_expect!(
                client,
                request,
                List<Pod>,
                eq("/api/v1/namespaces/default/pods"),
                Ok(setup_pod())
            );

            let selectors = vec![
                BTreeMap::from([("app".into(), "pod-1".into())]),
                BTreeMap::from([("version".into(), "v1".into())]),
            ];

            let client = RelatedClient::new(&client, "default");

            let result = client
                .related_resources::<Pod, _>(&selectors)
                .await
                .unwrap()
                .unwrap();

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

            let selectors = vec![BTreeMap::from([("hoge".into(), "fuga".into())])];

            let client = RelatedClient::new(&client, "default");

            let result = client
                .related_resources::<Pod, _>(&selectors)
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
                List<Pod>,
                eq("/api/v1/namespaces/default/pods"),
                bail!("error")
            );

            let client = RelatedClient::new(&client, "default");

            let result = client
                .related_resources::<Pod, BTreeMap<String, String>>(&Default::default())
                .await;

            assert_eq!(result.is_err(), true);
        }
    }
}
