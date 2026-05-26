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

        let pod_lines = format_related_pods_table(&pods);
        if !pod_lines.is_empty() {
            lines.push("---".to_string());
            lines.push(format!("# Related Pods (spec.nodeName={})", name));
            lines.extend(pod_lines);
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

/// Format the related-Pods list as a `# `-prefixed table whose columns are
/// padded so the header and rows line up. Returns an empty `Vec` when the
/// list is empty so the caller can decide whether to render the section.
fn format_related_pods_table(pods: &ObjectList<Pod>) -> Vec<String> {
    if pods.items.is_empty() {
        return Vec::new();
    }

    const HEADERS: [&str; 3] = ["NAMESPACE", "NAME", "STATUS"];

    let rows: Vec<[&str; 3]> = pods
        .items
        .iter()
        .map(|pod| {
            [
                pod.metadata.namespace.as_deref().unwrap_or(""),
                pod.metadata.name.as_deref().unwrap_or(""),
                pod.status
                    .as_ref()
                    .and_then(|s| s.phase.as_deref())
                    .unwrap_or(""),
            ]
        })
        .collect();

    // Per-column max width across header + rows. Pod names and namespaces are
    // ASCII (DNS-1123), so byte length equals display width.
    let col_width = |i: usize| -> usize {
        HEADERS[i]
            .len()
            .max(rows.iter().map(|r| r[i].len()).max().unwrap_or(0))
    };
    let w0 = col_width(0);
    let w1 = col_width(1);
    // The last column is right-most, so no trailing padding is needed.

    let format_row = |cols: &[&str; 3]| -> String {
        format!(
            "# {:<w0$}  {:<w1$}  {}",
            cols[0],
            cols[1],
            cols[2],
            w0 = w0,
            w1 = w1
        )
    };

    let mut out = Vec::with_capacity(1 + rows.len());
    out.push(format_row(&HEADERS));
    for row in &rows {
        out.push(format_row(row));
    }
    out
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
    fn format_related_pods_table_aligns_header_and_rows() {
        let list = pod_list(vec![
            sample_pod("gpu", "gpu-train-0", "Running"),
            sample_pod("gpu", "dcgm-exporter-x9f2", "Running"),
        ]);

        let lines = format_related_pods_table(&list);

        // NAMESPACE 列幅: max(NAMESPACE=9, gpu=3) = 9
        // NAME 列幅:      max(NAME=4, gpu-train-0=11, dcgm-exporter-x9f2=18) = 18
        // STATUS 列は末尾なのでパディングなし。
        assert_eq!(
            lines,
            vec![
                "# NAMESPACE  NAME                STATUS".to_string(),
                "# gpu        gpu-train-0         Running".to_string(),
                "# gpu        dcgm-exporter-x9f2  Running".to_string(),
            ]
        );
    }

    #[test]
    fn format_related_pods_table_empty_list_returns_empty() {
        let list = pod_list(vec![]);
        assert!(format_related_pods_table(&list).is_empty());
    }

    #[test]
    fn format_related_pods_table_header_widens_with_short_data() {
        // すべて NAMESPACE/NAME より短ければヘッダ幅が幅を決める。
        let list = pod_list(vec![sample_pod("a", "b", "X")]);
        let lines = format_related_pods_table(&list);
        assert_eq!(
            lines,
            vec![
                "# NAMESPACE  NAME  STATUS".to_string(),
                "# a          b     X".to_string(),
            ]
        );
    }
}
