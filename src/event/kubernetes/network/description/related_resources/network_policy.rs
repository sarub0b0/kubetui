use anyhow::{Ok, Result};
use k8s_openapi::{api::networking::v1::NetworkPolicy, List};

use std::collections::BTreeMap;

use kube::Resource;
use serde_yaml::Value;

use crate::event::kubernetes::client::KubeClientRequest;

use super::{
    btree_map_contains_key_values::BTreeMapContains,
    fetch::FetchClient,
    label_selector::{LabelSelectorExpression, LabelSelectorWrapper},
    Filter, RelatedClient,
};

impl Filter<BTreeMap<String, String>> for List<NetworkPolicy> {
    type Filtered = NetworkPolicy;

    fn filter_by_item(&self, arg: &BTreeMap<String, String>) -> Option<List<Self::Filtered>> {
        let ret: Vec<NetworkPolicy> = self
            .items
            .iter()
            .filter(|item| {
                item.spec.as_ref().map_or(false, |spec| {
                    let wrapper: LabelSelectorWrapper = spec.pod_selector.clone().into();
                    wrapper.expression(arg)
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

impl Filter<Vec<BTreeMap<String, String>>> for List<NetworkPolicy> {
    type Filtered = NetworkPolicy;

    fn filter_by_item(&self, arg: &Vec<BTreeMap<String, String>>) -> Option<List<Self::Filtered>> {
        let ret: Vec<NetworkPolicy> = self
            .items
            .iter()
            .filter(|item| {
                item.spec.as_ref().map_or(false, |spec| {
                    let wrapper: LabelSelectorWrapper = spec.pod_selector.clone().into();

                    arg.iter().any(|arg| wrapper.expression(arg))
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

#[allow(clippy::bool_assert_comparison)]
#[cfg(test)]
mod tests {
    use super::*;

    use indoc::indoc;

    fn networkpolicies() -> List<NetworkPolicy> {
        serde_yaml::from_str(indoc! {
            "
            items:
              - apiVersion: networking.k8s.io/v1
                kind: NetworkPolicy
                metadata:
                  name: allow-egress
                spec:
                  egress:
                    - {}
                  podSelector:
                    matchLabels:
                      version: v1
                  policyTypes:
                    - Egress
              - apiVersion: networking.k8s.io/v1
                kind: NetworkPolicy
                metadata:
                  name: allow-ingress
                spec:
                  ingress:
                    - {}
                  podSelector:
                    matchLabels:
                      app: pod-2
                  policyTypes:
                    - Ingress
            "
        })
        .unwrap()
    }

    mod filter {
        use super::*;

        mod not_vec {
            use super::*;

            use indoc::indoc;
            use pretty_assertions::assert_eq;

            #[test]
            fn labelsを対象とするpod_selectorのときそのnetworkpolicyのリストを返す() {
                let labels = BTreeMap::from([
                    ("app".into(), "pod-1".into()),
                    ("version".into(), "v1".into()),
                ]);

                let list = networkpolicies();

                let actual = list.filter_by_item(&labels);

                let expected = serde_yaml::from_str(indoc! {
                    "
                    items:
                      - apiVersion: networking.k8s.io/v1
                        kind: NetworkPolicy
                        metadata:
                          name: allow-egress
                        spec:
                          egress:
                            - {}
                          podSelector:
                            matchLabels:
                              version: v1
                          policyTypes:
                            - Egress
                    "
                })
                .unwrap();

                assert_eq!(actual, Some(expected));
            }

            #[test]
            fn labelsが対象とならないときnoneを返す() {
                let labels = BTreeMap::from([("hoge".into(), "fuga".into())]);

                let list = networkpolicies();

                let actual = list.filter_by_item(&labels);

                assert_eq!(actual.is_none(), true);
            }
        }

        mod vec {
            use super::*;

            use indoc::indoc;
            use pretty_assertions::assert_eq;

            #[test]
            fn labelsを対象とするpod_selectorのときそのnetworkpolicyのリストを返す() {
                let labels = vec![
                    BTreeMap::from([
                        ("app".into(), "pod-1".into()),
                        ("version".into(), "v1".into()),
                    ]),
                    BTreeMap::from([
                        ("app".into(), "pod-2".into()),
                        ("version".into(), "v1".into()),
                    ]),
                ];

                let list = networkpolicies();

                let actual = list.filter_by_item(&labels);

                let expected = serde_yaml::from_str(indoc! {
                    "
                    items:
                      - apiVersion: networking.k8s.io/v1
                        kind: NetworkPolicy
                        metadata:
                          name: allow-egress
                        spec:
                          egress:
                            - {}
                          podSelector:
                            matchLabels:
                              version: v1
                          policyTypes:
                            - Egress
                      - apiVersion: networking.k8s.io/v1
                        kind: NetworkPolicy
                        metadata:
                          name: allow-ingress
                        spec:
                          ingress:
                            - {}
                          podSelector:
                            matchLabels:
                              app: pod-2
                          policyTypes:
                            - Ingress
                    "
                })
                .unwrap();

                assert_eq!(actual, Some(expected));
            }

            #[test]
            fn labelsが対象とならないときnoneを返す() {
                let labels = vec![
                    BTreeMap::from([("hoge".into(), "fuga".into())]),
                    BTreeMap::from([("foo".into(), "bar".into())]),
                ];

                let list = networkpolicies();

                let actual = list.filter_by_item(&labels);

                assert_eq!(actual.is_none(), true);
            }
        }
    }

    mod related_resources {
        use anyhow::bail;
        use indoc::indoc;
        use mockall::predicate::eq;

        use super::*;

        use crate::{event::kubernetes::client::mock::MockTestKubeClient, mock_expect};

        #[tokio::test]
        async fn labelsを対象とするnetworkpolicyのlistを返す() {
            let mut client = MockTestKubeClient::new();

            mock_expect!(
                client,
                request,
                List<NetworkPolicy>,
                eq("/apis/networking.k8s.io/v1/namespaces/default/networkpolicies"),
                Ok(networkpolicies())
            );

            let labels = vec![
                BTreeMap::from([
                    ("app".into(), "pod-1".into()),
                    ("version".into(), "v1".into()),
                ]),
                BTreeMap::from([("app".into(), "pod-2".into())]),
            ];

            let client = RelatedClient::new(&client, "default");

            let result = client
                .related_resources::<NetworkPolicy, _>(&labels)
                .await
                .unwrap()
                .unwrap();

            let expected = serde_yaml::from_str(indoc! {
                "
                items:
                  - apiVersion: networking.k8s.io/v1
                    kind: NetworkPolicy
                    metadata:
                      name: allow-egress
                    spec:
                      egress:
                        - {}
                      podSelector:
                        matchLabels:
                          version: v1
                      policyTypes:
                        - Egress
                  - apiVersion: networking.k8s.io/v1
                    kind: NetworkPolicy
                    metadata:
                      name: allow-ingress
                    spec:
                      ingress:
                        - {}
                      podSelector:
                        matchLabels:
                          app: pod-2
                      policyTypes:
                        - Ingress
                "
            })
            .unwrap();

            assert_eq!(result, expected);
        }

        #[tokio::test]
        async fn labelsが対象にならないときnoneを返す() {
            let mut client = MockTestKubeClient::new();

            mock_expect!(
                client,
                request,
                List<NetworkPolicy>,
                eq("/apis/networking.k8s.io/v1/namespaces/default/networkpolicies"),
                Ok(networkpolicies())
            );

            let labels = vec![BTreeMap::from([("hoge".into(), "fuga".into())])];

            let client = RelatedClient::new(&client, "default");

            let result = client
                .related_resources::<NetworkPolicy, _>(&labels)
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
                List<NetworkPolicy>,
                eq("/apis/networking.k8s.io/v1/namespaces/default/networkpolicies"),
                bail!("error")
            );

            let client = RelatedClient::new(&client, "default");

            let result = client
                .related_resources::<NetworkPolicy, _>(&BTreeMap::default())
                .await;

            assert_eq!(result.is_err(), true);
        }
    }
}
