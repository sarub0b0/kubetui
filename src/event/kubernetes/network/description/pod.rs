mod fetched_pod;
mod fetched_service;

use fetched_pod::*;
use fetched_service::*;

use super::DescriptionWorker;

use std::collections::BTreeMap;

use crossbeam::channel::Sender;

use serde::{Deserialize, Serialize};

use crate::{
    error::Result,
    event::{
        kubernetes::{client::KubeClient, network::NetworkMessage},
        Event,
    },
};

pub(super) struct PodDescriptionWorker<'a> {
    client: &'a KubeClient,
    tx: &'a Sender<Event>,
    namespace: String,
    name: String,
}

#[async_trait::async_trait]
impl<'a> DescriptionWorker<'a> for PodDescriptionWorker<'a> {
    fn new(client: &'a KubeClient, tx: &'a Sender<Event>, namespace: String, name: String) -> Self {
        PodDescriptionWorker {
            client,
            tx,
            namespace,
            name,
        }
    }

    // TODO 関連するService, Ingress, NetworkPolicyの情報を合わせて表示する
    async fn run(&self) -> Result<()> {
        let mut value = Vec::new();

        let pod = self.fetch_pod().await?;
        let service = self.fetch_service(&pod.0.metadata.labels).await?;

        value.extend(pod.to_vec_string());

        if let Some(service) = service {
            value.push("\n".to_string());
            value.extend(service.to_vec_string());
        }

        self.tx.send(NetworkMessage::Response(Ok(value)).into())?;

        Ok(())
    }
}

impl<'a> PodDescriptionWorker<'a> {
    async fn fetch_pod(&self) -> Result<FetchedPod> {
        let url = format!("api/v1/namespaces/{}/pods/{}", self.namespace, self.name);

        let res = self.client.request_text(&url).await?;

        let value: FetchedPod = serde_json::from_str(&res)?;

        Ok(value)
    }

    async fn fetch_service(
        &self,
        pod_labels: &Option<BTreeMap<String, String>>,
    ) -> Result<Option<FetchedService>> {
        let url = format!("api/v1/namespaces/{}/services", self.namespace);
        let res = self.client.request_text(&url).await?;

        let list: FetchedServiceList = serde_json::from_str(&res)?;

        if let Some(service) = list.items.iter().find(|s| {
            s.spec.as_ref().map_or(false, |spec| {
                contains_key_values(&spec.selector, pod_labels)
            })
        }) {
            Ok(Some(FetchedService(service.clone())))
        } else {
            Ok(None)
        }
    }
}

fn contains_key_values(
    lhs: &Option<BTreeMap<String, String>>,
    rhs: &Option<BTreeMap<String, String>>,
) -> bool {
    #[cfg(feature = "logging")]
    ::log::debug!("match_selector {:#?} <=> {:#?}", service_labels, pod_labels);

    lhs.as_ref().map_or(false, |lhs| {
        rhs.as_ref().map_or(false, |rhs| {
            lhs.iter().all(|(lhs_key, lhs_value)| {
                rhs.get(lhs_key)
                    .map_or(false, |rhs_value| lhs_value == rhs_value)
            })
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    mod match_selector {
        use super::*;

        #[test]
        fn lhsの値すべてがrhsにふくまれていればtrueを返す() {
            let lhs = Some(BTreeMap::from_iter(vec![
                ("a".to_string(), "aaa".to_string()),
                ("b".to_string(), "bbb".to_string()),
            ]));

            let rhs = Some(BTreeMap::from_iter(vec![
                ("a".to_string(), "aaa".to_string()),
                ("b".to_string(), "bbb".to_string()),
                ("c".to_string(), "ccc".to_string()),
            ]));

            assert!(contains_key_values(&lhs, &rhs));
        }

        #[test]
        fn lhsの値すべてがrhsにふくまれていなければfalseを返す() {
            let lhs = Some(BTreeMap::from_iter(vec![
                ("a".to_string(), "aaa".to_string()),
                ("b".to_string(), "bbb".to_string()),
            ]));

            let rhs = Some(BTreeMap::from_iter(vec![
                ("b".to_string(), "bbb".to_string()),
                ("c".to_string(), "ccc".to_string()),
            ]));

            assert!(!contains_key_values(&lhs, &rhs));
        }
    }
}
