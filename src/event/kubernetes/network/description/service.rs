use k8s_openapi::api::core::v1::Service;

use crate::{error::Result, event::kubernetes::client::KubeClientRequest};

use super::{Fetch, FetchedData};

pub(super) struct ServiceDescriptionWorker<'a, C>
where
    C: KubeClientRequest,
{
    client: &'a C,
    namespace: String,
    name: String,
}

#[async_trait::async_trait]
impl<'a, C> Fetch<'a, C> for ServiceDescriptionWorker<'a, C>
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
            "api/v1/namespaces/{}/services/{}",
            self.namespace, self.name
        );

        let res = self.client.request_text(&url).await?;

        let mut value: Service = serde_json::from_str(&res)?;

        value.metadata.managed_fields = None;

        let value = serde_yaml::to_string(&value)?
            .lines()
            .skip(1)
            .map(ToString::to_string)
            .collect();

        Ok(value)
    }
}
mod extract {
    use anyhow::Result;
    use k8s_openapi::api::core::v1::Service;
    use kube::api::ObjectMeta;

    pub trait Extract {
        fn extract(&self) -> Self
        where
            Self: Sized;
    }

    impl Extract for Service {
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
            Service {
                metadata: ObjectMeta {
                    annotations,
                    labels: self.metadata.labels.clone(),
                    name: self.metadata.name.clone(),
                    ..Default::default()
                },
                spec: self.spec.clone(),
                status: self.status.clone(),
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use indoc::indoc;
        use pretty_assertions::assert_eq;

        use super::*;

        fn service() -> Service {
            serde_yaml::from_str(indoc! {
                r#"
                apiVersion: v1
                kind: Service
                metadata:
                  annotations:
                    kubectl.kubernetes.io/last-applied-configuration: |
                      {"apiVersion":"v1","kind":"Service","metadata":{"annotations":{},"name":"service-0","namespace":"kubetui"},"spec":{"ports":[{"port":80,"targetPort":80}],"selector":{"app":"app"}}}
                    foo: bar
                  labels:
                    foo: bar
                  creationTimestamp: "2022-03-27T09:17:06Z"
                  name: service-0
                  namespace: kubetui
                  resourceVersion: "714"
                  uid: 7971c237-d5d8-468d-aeb9-ee6f9449c702
                spec:
                  clusterIP: 10.108.138.180
                  clusterIPs:
                  - 10.108.138.180
                  internalTrafficPolicy: Cluster
                  ipFamilies:
                  - IPv4
                  ipFamilyPolicy: SingleStack
                  ports:
                  - port: 80
                    protocol: TCP
                    targetPort: 80
                  selector:
                    app: app
                  sessionAffinity: None
                  type: ClusterIP
                status:
                  loadBalancer: {}
                "#
            })
            .unwrap()
        }

        #[test]
        fn 必要な情報のみを抽出してserviceを返す() {
            let actual = service().extract();

            let expected = serde_yaml::from_str(indoc! {
                r#"
                apiVersion: v1
                kind: Service
                metadata:
                  annotations:
                    foo: bar
                  labels:
                    foo: bar
                  name: service-0
                spec:
                  clusterIP: 10.108.138.180
                  clusterIPs:
                  - 10.108.138.180
                  internalTrafficPolicy: Cluster
                  ipFamilies:
                  - IPv4
                  ipFamilyPolicy: SingleStack
                  ports:
                  - port: 80
                    protocol: TCP
                    targetPort: 80
                  selector:
                    app: app
                  sessionAffinity: None
                  type: ClusterIP
                status:
                  loadBalancer: {}
                "#
            })
            .unwrap();

            assert_eq!(actual, expected);
        }
    }
}
