use anyhow::{Context, Result};
use crossbeam::channel::Sender;
use k8s_openapi::api::core::v1::{Node, Pod};
use kube::{api::ListParams, core::ObjectList, Api};
use serde::Serialize;

use crate::{
    features::node::message::NodeDetailMessage,
    kube::KubeClientRequest,
    logger,
    message::Message,
    workers::kube::InfiniteWorker,
};

const INTERVAL: u64 = 3;

#[derive(Clone)]
pub struct NodeDetailWorker<C>
where
    C: Clone,
{
    tx: Sender<Message>,
    client: C,
    name: String,
}

impl<C> NodeDetailWorker<C>
where
    C: KubeClientRequest + Clone + Send + Sync + 'static,
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

        let related = build_related_pods_yaml(&pods)?;
        if !related.is_empty() {
            // Blank line between the main Node YAML and the relatedPods
            // section, matching Network description's layout.
            lines.push(String::new());
            lines.extend(related);
        }

        Ok(lines)
    }
}

#[async_trait::async_trait]
impl<C> InfiniteWorker for NodeDetailWorker<C>
where
    C: KubeClientRequest + Clone + Send + Sync + 'static,
{
    async fn run(&self) {
        if let Err(e) = self.fetch_loop().await {
            logger!(error, "node detail worker exited: {:?}", e);
        }
    }
}

impl<C> NodeDetailWorker<C>
where
    C: KubeClientRequest + Clone + Send + Sync + 'static,
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

#[derive(Serialize)]
struct RelatedPodRow {
    namespace: String,
    name: String,
    status: String,
}

/// Build the YAML lines for the `relatedPods` section. Returns an empty `Vec`
/// when the list is empty so the caller can skip the separator blank line.
///
/// Mirrors Network description's `relatedResources` shape so the entire
/// detail pane is a single valid YAML document.
fn build_related_pods_yaml(pods: &ObjectList<Pod>) -> Result<Vec<String>> {
    if pods.items.is_empty() {
        return Ok(Vec::new());
    }

    let rows: Vec<RelatedPodRow> = pods
        .items
        .iter()
        .map(|pod| {
            RelatedPodRow {
                namespace: pod.metadata.namespace.clone().unwrap_or_default(),
                name: pod.metadata.name.clone().unwrap_or_default(),
                status: pod
                    .status
                    .as_ref()
                    .and_then(|s| s.phase.clone())
                    .unwrap_or_default(),
            }
        })
        .collect();

    let mut root = serde_yaml::Mapping::new();
    root.insert(
        serde_yaml::Value::String("relatedPods".to_string()),
        serde_yaml::to_value(&rows).with_context(|| "failed to serialize related pods")?,
    );

    let yaml =
        serde_yaml::to_string(&root).with_context(|| "failed to serialize relatedPods section")?;

    Ok(yaml.lines().map(String::from).collect())
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
    fn build_related_pods_yaml_emits_valid_yaml_document() {
        let list = pod_list(vec![
            sample_pod("gpu", "gpu-train-0", "Running"),
            sample_pod("kube-system", "kube-proxy-x9f2", "Running"),
        ]);

        let lines = build_related_pods_yaml(&list).unwrap();
        let yaml = lines.join("\n");

        // 全体を YAML として roundtrip し、構造で検証する（インデントや改行の
        // 細部に依存しない）。
        let parsed: serde_yaml::Value = serde_yaml::from_str(&yaml).unwrap();
        let related = parsed.get("relatedPods").unwrap().as_sequence().unwrap();
        assert_eq!(related.len(), 2);

        assert_eq!(
            related[0].get("namespace").and_then(|v| v.as_str()),
            Some("gpu")
        );
        assert_eq!(
            related[0].get("name").and_then(|v| v.as_str()),
            Some("gpu-train-0")
        );
        assert_eq!(
            related[0].get("status").and_then(|v| v.as_str()),
            Some("Running")
        );

        assert_eq!(
            related[1].get("namespace").and_then(|v| v.as_str()),
            Some("kube-system")
        );
        assert_eq!(
            related[1].get("name").and_then(|v| v.as_str()),
            Some("kube-proxy-x9f2")
        );
        assert_eq!(
            related[1].get("status").and_then(|v| v.as_str()),
            Some("Running")
        );
    }

    #[test]
    fn build_related_pods_yaml_empty_list_returns_empty() {
        let list = pod_list(vec![]);
        assert!(build_related_pods_yaml(&list).unwrap().is_empty());
    }
}
