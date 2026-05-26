use anyhow::{Context, Result};
use crossbeam::channel::Sender;
use k8s_openapi::api::core::v1::{Node, Pod};
use kube::{api::ListParams, core::ObjectList, Api};

use crate::{
    features::node::message::NodeDetailMessage,
    kube::KubeClientRequest,
    logger,
    message::Message,
    workers::kube::InfiniteWorker,
};

const INTERVAL: u64 = 3;

pub struct NodeDetailWorker<C> {
    tx: Sender<Message>,
    client: C,
    name: String,
}

impl<C> NodeDetailWorker<C>
where
    C: KubeClientRequest + Send + Sync + 'static,
{
    pub fn new(tx: Sender<Message>, client: C, name: String) -> Self {
        Self { tx, client, name }
    }

    /// Fetch Node + related Pods and combine into a single line array.
    ///
    /// The fetch is a thin delegation to `kube::Api` (matches `log_streamer` /
    /// `pod_watcher` in this codebase). The pure formatters below are what the
    /// unit tests target.
    pub async fn fetch_for(name: &str, client: &C) -> Result<Vec<String>> {
        let kube_client = client.client().clone();

        // 1) Node: typed get via kube::Api.
        let node_api: Api<Node> = Api::all(kube_client.clone());
        let node = node_api
            .get(name)
            .await
            .with_context(|| format!("failed to fetch node {}", name))?;
        let mut lines = strip_and_serialize_node(node)?;

        // 2) Related Pods: typed list across all namespaces with field selector.
        let pod_api: Api<Pod> = Api::all(kube_client);
        let lp = ListParams::default().fields(&format!("spec.nodeName={}", name));
        let pods = pod_api
            .list(&lp)
            .await
            .with_context(|| format!("failed to list pods on node {}", name))?;

        let pod_rows = format_related_pods(&pods);
        if !pod_rows.is_empty() {
            lines.push("---".to_string());
            lines.push(format!("# Related Pods (spec.nodeName={})", name));
            lines.push("# NAMESPACE  NAME  STATUS".to_string());
            lines.extend(pod_rows);
        }

        Ok(lines)
    }
}

#[async_trait::async_trait]
impl<C> InfiniteWorker for NodeDetailWorker<C>
where
    C: KubeClientRequest + Send + Sync + 'static,
{
    async fn run(&self) {
        if let Err(e) = self.fetch_loop().await {
            logger!(error, "node detail worker exited: {:?}", e);
        }
    }
}

impl<C> NodeDetailWorker<C>
where
    C: KubeClientRequest + Send + Sync + 'static,
{
    async fn fetch_loop(&self) -> Result<()> {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(INTERVAL));

        loop {
            interval.tick().await;

            let result = Self::fetch_for(&self.name, &self.client).await;

            self.tx.send(NodeDetailMessage::Response(result).into())?;
        }
    }
}

/// Strip `metadata.managedFields` and serialize the Node as YAML lines.
fn strip_and_serialize_node(mut node: Node) -> Result<Vec<String>> {
    node.metadata.managed_fields = None;
    let yaml = serde_yaml::to_string(&node).with_context(|| "failed to serialize node as YAML")?;
    Ok(yaml.lines().map(String::from).collect())
}

/// Format the related-Pods list as `# <ns>  <name>  <phase>` rows.
fn format_related_pods(pods: &ObjectList<Pod>) -> Vec<String> {
    pods.items
        .iter()
        .map(|pod| {
            let ns = pod.metadata.namespace.as_deref().unwrap_or("");
            let name = pod.metadata.name.as_deref().unwrap_or("");
            let phase = pod
                .status
                .as_ref()
                .and_then(|s| s.phase.as_deref())
                .unwrap_or("");
            format!("# {}  {}  {}", ns, name, phase)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use k8s_openapi::{
        api::core::v1::{Node, Pod, PodStatus},
        apimachinery::pkg::apis::meta::v1::{ManagedFieldsEntry, ObjectMeta},
    };
    use pretty_assertions::assert_eq;
    use std::collections::BTreeMap;

    fn sample_node_with_managed_fields() -> Node {
        Node {
            metadata: ObjectMeta {
                name: Some("node-a".to_string()),
                labels: Some(BTreeMap::from([("role".to_string(), "worker".to_string())])),
                managed_fields: Some(vec![ManagedFieldsEntry {
                    manager: Some("kubelet".to_string()),
                    ..Default::default()
                }]),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn sample_pod(ns: &str, name: &str, phase: &str) -> Pod {
        Pod {
            metadata: ObjectMeta {
                namespace: Some(ns.to_string()),
                name: Some(name.to_string()),
                ..Default::default()
            },
            status: Some(PodStatus {
                phase: Some(phase.to_string()),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    fn pod_list(items: Vec<Pod>) -> ObjectList<Pod> {
        ObjectList {
            metadata: Default::default(),
            items,
            types: Default::default(),
        }
    }

    #[test]
    fn strip_and_serialize_node_removes_managed_fields() {
        let node = sample_node_with_managed_fields();
        let lines = strip_and_serialize_node(node).unwrap();
        let joined = lines.join("\n");

        assert!(joined.contains("name: node-a"));
        assert!(joined.contains("role: worker"));
        assert!(!joined.contains("managedFields"));
        assert!(!joined.contains("kubelet"));
    }

    #[test]
    fn format_related_pods_yields_one_row_per_pod() {
        let list = pod_list(vec![
            sample_pod("ns1", "pod-a", "Running"),
            sample_pod("ns2", "pod-b", "Pending"),
        ]);

        let rows = format_related_pods(&list);

        assert_eq!(
            rows,
            vec![
                "# ns1  pod-a  Running".to_string(),
                "# ns2  pod-b  Pending".to_string(),
            ]
        );
    }

    #[test]
    fn format_related_pods_empty_list_returns_empty() {
        let list = pod_list(vec![]);
        assert!(format_related_pods(&list).is_empty());
    }
}
