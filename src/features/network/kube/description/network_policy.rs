use anyhow::Result;
use k8s_openapi::{
    api::{
        core::v1::Pod,
        networking::v1::{NetworkPolicy, NetworkPolicySpec},
    },
    List,
};
use kube::Resource;
use serde_yaml::Mapping;

use crate::{
    features::{
        api_resources::kube::SharedApiResources, network::message::NetworkRequestTargetParams,
    },
    kube::KubeClientRequest,
};

use self::{extract::Extract, to_value::ToValue};

use super::{
    related_resources::{
        label_selector::LabelSelectorWrapper, to_list_value::ToListValue, RelatedClient,
    },
    Fetch, FetchedData,
};

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
    fn new(client: &'a C, params: NetworkRequestTargetParams, _: SharedApiResources) -> Self {
        let NetworkRequestTargetParams {
            namespace, name, ..
        } = params;

        Self {
            client,
            namespace,
            name,
        }
    }

    async fn fetch(&self) -> Result<FetchedData> {
        let url = format!(
            "{}/{}",
            NetworkPolicy::url_path(&Default::default(), Some(&self.namespace)),
            self.name
        );

        let networkpolicy: NetworkPolicy = self.client.request(&url).await?;
        let networkpolicy = networkpolicy.extract();

        let related_pods: Option<List<Pod>> = if let Some(NetworkPolicySpec {
            pod_selector: Some(pod_selector),
            ..
        }) = &networkpolicy.spec
        {
            RelatedClient::new(self.client, &self.namespace)
                .related_resources::<Pod, LabelSelectorWrapper>(&pod_selector.clone().into())
                .await?
        } else {
            None
        };

        let mut related_resources = Mapping::new();

        if let Some(pods) = related_pods {
            if let Some(value) = pods.to_list_value() {
                related_resources.insert("pods".into(), value);
            }
        }

        let value: Vec<String> = serde_yaml::to_string(&networkpolicy.to_value()?)?
            .lines()
            .map(ToString::to_string)
            .collect();

        let mut value = value;

        if !related_resources.is_empty() {
            let mut root = Mapping::new();

            root.insert("relatedResources".into(), related_resources.into());

            let related_resources: Vec<String> = serde_yaml::to_string(&root)?
                .lines()
                .map(ToString::to_string)
                .collect();

            value.push(Default::default());

            value.extend(related_resources);
        }

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

#[cfg(test)]
mod tests {
    use anyhow::bail;
    use indoc::indoc;
    use k8s_openapi::{api::core::v1::Pod, List};
    use mockall::predicate::eq;
    use pretty_assertions::assert_eq;

    use crate::{
        features::api_resources::kube::ApiResources, kube::mock::MockTestKubeClient, mock_expect,
    };

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
        }).unwrap()
    }

    fn pods() -> List<Pod> {
        serde_yaml::from_str(indoc! {
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
            - metadata:
                name: pod-3
                labels:
                  app: pod-3
                  version: v2
            "
        })
        .unwrap()
    }

    #[tokio::test]
    async fn yamlデータを返す() {
        let mut client = MockTestKubeClient::new();

        mock_expect!(
            client,
            request,
            [
                (
                    NetworkPolicy,
                    eq("/apis/networking.k8s.io/v1/namespaces/default/networkpolicies/test"),
                    Ok(networkpolicy())
                ),
                (
                    List<Pod>,
                    eq("/api/v1/namespaces/default/pods"),
                    Ok(pods())
                )
            ]
        );

        let target_parmas = NetworkRequestTargetParams {
            namespace: "default".to_string(),
            name: "test".to_string(),
            version: "v1".to_string(),
        };

        let worker =
            NetworkPolicyDescriptionWorker::new(&client, target_parmas, ApiResources::shared());

        let result = worker.fetch().await;

        let expected: Vec<String> = indoc! {
            r#"
            networkpolicy:
              metadata:
                name: test
              spec:
                ingress:
                - {}
                podSelector: {}
                policyTypes:
                - Ingress

            relatedResources:
              pods:
              - pod-1
              - pod-2
              - pod-3
            "#
        }
        .lines()
        .map(ToString::to_string)
        .collect();

        assert_eq!(result.unwrap(), expected);
    }

    #[tokio::test]
    async fn エラーのときerrorを返す() {
        let mut client = MockTestKubeClient::new();
        mock_expect!(
            client,
            request,
            [
                (
                    NetworkPolicy,
                    eq("/apis/networking.k8s.io/v1/namespaces/default/networkpolicies/test"),
                    bail!("error")
                ),
                (
                    List<Pod>,
                    eq("/api/v1/namespaces/default/pods"),
                    bail!("error")
                )
            ]
        );

        let target_parmas = NetworkRequestTargetParams {
            namespace: "default".to_string(),
            name: "test".to_string(),
            version: "v1".to_string(),
        };

        let worker =
            NetworkPolicyDescriptionWorker::new(&client, target_parmas, ApiResources::shared());

        let result = worker.fetch().await;

        assert_eq!(result.is_err(), true);
    }
}
