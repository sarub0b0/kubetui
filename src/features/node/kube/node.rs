use std::{collections::BTreeMap, sync::Arc};

use anyhow::Result;
use async_trait::async_trait;
use crossbeam::channel::Sender;
use k8s_openapi::{api::core::v1::Node, Resource as _};
use kube::Resource;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use tokio::sync::RwLock;

use crate::{
    features::node::{message::NodeMessage, NodeColumn, NodeColumnSpec, NodeColumns},
    kube::{
        apis::v1_table::Table,
        table::{KubeTable, KubeTableRow},
        KubeClient,
        KubeClientRequest,
    },
    logger,
    message::Message,
    workers::kube::InfiniteWorker,
};

pub type SharedNodeColumns = Arc<RwLock<NodeColumns>>;
pub type SharedNodeFilter = Arc<RwLock<Option<String>>>;

#[derive(Debug, Clone, Default)]
pub struct NodeConfig {
    pub default_columns: Option<NodeColumns>,
}

#[derive(Clone)]
pub struct NodePoller {
    tx: Sender<Message>,
    shared_node_columns: SharedNodeColumns,
    shared_node_filter: SharedNodeFilter,
    kube_client: KubeClient,
}

impl NodePoller {
    pub fn new(
        tx: Sender<Message>,
        shared_node_columns: SharedNodeColumns,
        shared_node_filter: SharedNodeFilter,
        kube_client: KubeClient,
    ) -> Self {
        Self {
            tx,
            shared_node_columns,
            shared_node_filter,
            kube_client,
        }
    }
}

#[async_trait]
impl InfiniteWorker for NodePoller {
    async fn run(&self) {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
        let Self { tx, .. } = self;
        loop {
            interval.tick().await;
            let node_info = get_node_table(
                &self.kube_client,
                &self.shared_node_columns,
                &self.shared_node_filter,
            )
            .await;
            if let Err(e) = tx.send(NodeMessage::Poll(node_info).into()) {
                logger!(error, "Failed to send NodeMessage::Poll: {}", e);
                return;
            }
        }
    }
}

async fn get_node_table<C: KubeClientRequest>(
    client: &C,
    shared_node_columns: &SharedNodeColumns,
    shared_node_filter: &SharedNodeFilter,
) -> Result<KubeTable> {
    let node_columns = shared_node_columns.read().await;

    let specs = node_columns.specs();

    let builtin_targets: Vec<&str> = specs
        .iter()
        .filter_map(|s| {
            match s {
                NodeColumnSpec::Builtin(c) => Some(c.as_str()),
                NodeColumnSpec::Label { .. } => None,
            }
        })
        .collect();

    let base_path = Node::url_path(&(), None);
    let path = {
        let filter = shared_node_filter.read().await;
        match filter.as_deref().filter(|s| !s.is_empty()) {
            Some(sel) => {
                format!(
                    "{}?labelSelector={}",
                    base_path,
                    utf8_percent_encode(sel, NON_ALPHANUMERIC)
                )
            }
            None => base_path,
        }
    };
    let table: Table = client.request_table(&path).await?;

    let builtin_indexes = table.find_indexes(&builtin_targets)?;

    let name_pos = specs
        .iter()
        .position(|s| matches!(s, NodeColumnSpec::Builtin(NodeColumn::Name)))
        .expect("Name column must be present in node columns");

    let rows: Vec<KubeTableRow> = table
        .rows
        .iter()
        .map(|row| {
            let mut builtin_iter = builtin_indexes.iter();
            let cells: Vec<String> = specs
                .iter()
                .map(|spec| {
                    match spec {
                        NodeColumnSpec::Builtin(_) => {
                            let i = builtin_iter.next().expect("builtin index available");
                            row.cells[*i].to_string()
                        }
                        NodeColumnSpec::Label { key, .. } => {
                            row.object
                                .as_ref()
                                .and_then(|o| o.0.get("metadata"))
                                .and_then(|m| m.get("labels"))
                                .and_then(|l| l.get(key))
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string()
                        }
                    }
                })
                .collect();
            let name = cells[name_pos].clone();
            KubeTableRow {
                namespace: String::new(),
                name,
                metadata: Some(BTreeMap::from([(
                    "kind".to_string(),
                    Node::KIND.to_string(),
                )])),
                row: cells,
            }
        })
        .collect();

    let header: Vec<String> = specs.iter().map(|s| s.header()).collect();

    let mut kube_table = KubeTable {
        header,
        ..Default::default()
    };
    kube_table.update_rows(rows);

    Ok(kube_table)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kube::apis::v1_table::{Table, TableColumnDefinition, TableRow, Value};
    use crate::mock_expect;
    use k8s_openapi::apimachinery::pkg::runtime::RawExtension;
    use mockall::predicate::eq;
    use pretty_assertions::assert_eq;
    use serde_json::Value as JsonValue;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    fn coldef(name: &str) -> TableColumnDefinition {
        TableColumnDefinition {
            name: name.to_string(),
            ..Default::default()
        }
    }

    fn row(cells: &[&str]) -> TableRow {
        TableRow {
            cells: cells
                .iter()
                .map(|c| Value(JsonValue::String(c.to_string())))
                .collect(),
            ..Default::default()
        }
    }

    fn node_table_fixture() -> Table {
        Table {
            column_definitions: vec![
                coldef("Name"),
                coldef("Status"),
                coldef("Roles"),
                coldef("Age"),
                coldef("Version"),
            ],
            rows: vec![
                row(&["node-a", "Ready", "worker", "10d", "v1.29.0"]),
                row(&["node-b", "NotReady", "control-plane", "11d", "v1.29.0"]),
            ],
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn builds_kube_table_from_default_columns() {
        let mut client = crate::kube::mock::MockTestKubeClient::new();
        mock_expect!(
            client,
            request_table,
            Table,
            eq("/api/v1/nodes"),
            Ok(node_table_fixture())
        );

        let shared = Arc::new(RwLock::new(NodeColumns::default()));
        let shared_filter = Arc::new(RwLock::new(None));
        let table = get_node_table(&client, &shared, &shared_filter)
            .await
            .unwrap();

        assert_eq!(
            table.header,
            vec!["NAME", "STATUS", "ROLES", "AGE", "VERSION"]
        );
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.rows[0].name, "node-a");
        assert_eq!(
            table.rows[0].row,
            vec!["node-a", "Ready", "worker", "10d", "v1.29.0"]
        );
    }

    fn row_with_labels(cells: &[&str], labels: &[(&str, &str)]) -> TableRow {
        let labels_json: serde_json::Map<String, JsonValue> = labels
            .iter()
            .map(|(k, v)| (k.to_string(), JsonValue::String(v.to_string())))
            .collect();
        let object = serde_json::json!({ "metadata": { "labels": labels_json } });
        TableRow {
            cells: cells
                .iter()
                .map(|c| Value(JsonValue::String(c.to_string())))
                .collect(),
            object: Some(RawExtension(object)),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn extracts_label_column_value_from_object() {
        let mut client = crate::kube::mock::MockTestKubeClient::new();
        mock_expect!(
            client,
            request_table,
            Table,
            eq("/api/v1/nodes"),
            Ok(Table {
                column_definitions: vec![coldef("Name"), coldef("Status")],
                rows: vec![row_with_labels(
                    &["node-a", "Ready"],
                    &[("nvidia.com/mig", "success")]
                )],
                ..Default::default()
            })
        );

        let specs = NodeColumns::new([
            NodeColumnSpec::Builtin(NodeColumn::Name),
            NodeColumnSpec::Label {
                key: "nvidia.com/mig".to_string(),
                header: "MIG".to_string(),
            },
        ]);
        let shared = Arc::new(RwLock::new(specs));
        let shared_filter = Arc::new(RwLock::new(None));
        let table = get_node_table(&client, &shared, &shared_filter)
            .await
            .unwrap();

        assert_eq!(table.header, vec!["NAME", "MIG"]);
        assert_eq!(table.rows[0].name, "node-a");
        assert_eq!(table.rows[0].row, vec!["node-a", "success"]);
    }

    #[tokio::test]
    async fn url_encodes_label_selector_equality_expression() {
        let mut client = crate::kube::mock::MockTestKubeClient::new();
        // `env=prod` の `=` は URL の sub-delim だが、percent-encoding でも
        // k8s API server は decode して labelSelector 文法として解釈する。
        mock_expect!(
            client,
            request_table,
            Table,
            eq("/api/v1/nodes?labelSelector=env%3Dprod"),
            Ok(node_table_fixture())
        );

        let shared = Arc::new(RwLock::new(NodeColumns::default()));
        let shared_filter = Arc::new(RwLock::new(Some("env=prod".to_string())));
        let table = get_node_table(&client, &shared, &shared_filter)
            .await
            .unwrap();
        assert_eq!(table.rows.len(), 2);
    }

    #[tokio::test]
    async fn url_encodes_comma_separated_and_expression() {
        // k8s labelSelector の AND 区切りはカンマ。`,` も `=` も URL の sub-delim
        // だが、percent-encoding でも k8s API server は decode して文法どおり
        // 解釈するので往復性が保たれる。
        let mut client = crate::kube::mock::MockTestKubeClient::new();
        mock_expect!(
            client,
            request_table,
            Table,
            eq("/api/v1/nodes?labelSelector=env%3Dprod%2Ctier%3Dfrontend"),
            Ok(node_table_fixture())
        );

        let shared = Arc::new(RwLock::new(NodeColumns::default()));
        let shared_filter = Arc::new(RwLock::new(Some("env=prod,tier=frontend".to_string())));
        let table = get_node_table(&client, &shared, &shared_filter)
            .await
            .unwrap();
        assert_eq!(table.rows.len(), 2);
    }

    #[tokio::test]
    async fn url_encodes_space_in_set_based_expression() {
        // `key in (a, b)` は labelSelector の set-based 構文。
        // 空白 / カンマ / カッコは全て URL-encode される。
        let mut client = crate::kube::mock::MockTestKubeClient::new();
        mock_expect!(
            client,
            request_table,
            Table,
            eq("/api/v1/nodes?labelSelector=env%20in%20%28prod%2Cdev%29"),
            Ok(node_table_fixture())
        );

        let shared = Arc::new(RwLock::new(NodeColumns::default()));
        let shared_filter = Arc::new(RwLock::new(Some("env in (prod,dev)".to_string())));
        let table = get_node_table(&client, &shared, &shared_filter)
            .await
            .unwrap();
        assert_eq!(table.rows.len(), 2);
    }

    #[tokio::test]
    async fn url_omits_label_selector_when_empty() {
        let mut client = crate::kube::mock::MockTestKubeClient::new();
        mock_expect!(
            client,
            request_table,
            Table,
            eq("/api/v1/nodes"),
            Ok(node_table_fixture())
        );

        let shared = Arc::new(RwLock::new(NodeColumns::default()));
        let shared_filter = Arc::new(RwLock::new(Some(String::new())));
        let table = get_node_table(&client, &shared, &shared_filter)
            .await
            .unwrap();
        assert_eq!(table.rows.len(), 2);
    }
}
