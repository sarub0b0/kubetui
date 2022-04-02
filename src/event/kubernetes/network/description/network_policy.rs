use k8s_openapi::api::networking::v1::NetworkPolicy;

use crate::{error::Result, event::kubernetes::client::KubeClientRequest};

use super::{Fetch, FetchedData};

pub(super) struct NetworkPolicyDescriptionWorker<'a, C>
where
    C: KubeClientRequest,
{
    client: &'a C,
    namespace: String,
    name: String,
}

#[async_trait::async_trait]
impl<'a, C> Fetch<'a, C> for NetworkPolicyDescriptionWorker<'a, C>
where
    C: KubeClientRequest,
{
    fn new(client: &'a C, namespace: String, name: String) -> Self {
        Self {
            client,
            namespace,
            name,
        }
    }

    async fn fetch(&self) -> Result<FetchedData> {
        let url = format!(
            "apis/networking.k8s.io/v1/namespaces/{}/networkpolicies/{}",
            self.namespace, self.name
        );

        let res = self.client.request_text(&url).await?;

        let mut value: NetworkPolicy = serde_json::from_str(&res)?;

        value.metadata.managed_fields = None;

        let value = serde_yaml::to_string(&value)?
            .lines()
            .skip(1)
            .map(ToString::to_string)
            .collect();

        Ok(value)
    }
}

mod to_value {
    use anyhow::Result;
    use k8s_openapi::api::networking::v1::NetworkPolicy;
    use serde_yaml::{Mapping, Value};

    pub trait ToValue {
        fn to_value(&self) -> Result<Option<Value>>;
    }

    impl ToValue for NetworkPolicy {
        fn to_value(&self) -> Result<Option<Value>> {
            let mut value = Mapping::new();

            value.insert("metadata".into(), serde_yaml::to_value(&self.metadata)?);

            if let Some(spec) = &self.spec {
                value.insert("spec".into(), serde_yaml::to_value(spec)?);
            }

            let ret = if !value.is_empty() {
                let mut root = Mapping::new();

                root.insert("networkpolicy".into(), value.into());

                Some(root.into())
            } else {
                None
            };

            Ok(ret)
        }
    }
}

mod extract {
    use k8s_openapi::api::networking::v1::NetworkPolicy;
    use kube::api::ObjectMeta;

    pub trait Extract {
        fn extract(&self) -> Self
        where
            Self: Sized;
    }

    impl Extract for NetworkPolicy {
        fn extract(&self) -> Self {
            let annotations = if let Some(mut annotations) = self.metadata.annotations.clone() {
                annotations.remove("kubectl.kubernetes.io/last-applied-configuration");
                if annotations.is_empty() {
                    None
                } else {
                    Some(annotations)
                }
            } else {
                None
            };
            NetworkPolicy {
                metadata: ObjectMeta {
                    annotations,
                    labels: self.metadata.labels.clone(),
                    name: self.metadata.name.clone(),
                    ..Default::default()
                },
                spec: self.spec.clone(),
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use indoc::indoc;
        use pretty_assertions::assert_eq;

        use super::*;

        fn networkpolicy() -> NetworkPolicy {
            serde_yaml::from_str(indoc! {
                r#"
                apiVersion: networking.k8s.io/v1
                kind: NetworkPolicy
                metadata:
                  annotations:
                    kubectl.kubernetes.io/last-applied-configuration: |
                      {"apiVersion":"networking.k8s.io/v1","kind":"NetworkPolicy","metadata":{"annotations":{},"name":"allow-all-ingress","namespace":"kubetui"},"spec":{"ingress":[{}],"podSelector":{},"policyTypes":["Ingress"]}}
                    foo: bar
                  creationTimestamp: "2022-03-27T09:17:06Z"
                  generation: 1
                  name: test
                  namespace: kubetui
                  resourceVersion: "777"
                  uid: c3a2c3c9-c74a-4a2f-be06-88e7cf527f5d
                spec:
                  ingress:
                    - {}
                  podSelector: {}
                  policyTypes:
                    - Ingress
                "#
            })
            .unwrap()
        }

        #[test]
        fn 必要な情報のみを抽出してserviceを返す() {
            let actual = networkpolicy().extract();

            let expected = serde_yaml::from_str(indoc! {
                r#"
                apiVersion: networking.k8s.io/v1
                kind: NetworkPolicy
                metadata:
                  annotations:
                    foo: bar
                  name: test
                spec:
                  ingress:
                    - {}
                  podSelector: {}
                  policyTypes:
                    - Ingress
                "#
            })
            .unwrap();

            assert_eq!(actual, expected);
        }
    }
}

