use std::{collections::BTreeMap, sync::Arc};

use anyhow::Result;
use async_trait::async_trait;
use crossbeam::channel::Sender;
use k8s_openapi::{api::core::v1::Node, Resource as _};
use kube::Resource;
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

#[derive(Debug, Clone, Default)]
pub struct NodeConfig {
    pub default_columns: Option<NodeColumns>,
}

#[derive(Clone)]
pub struct NodePoller {
    tx: Sender<Message>,
    shared_node_columns: SharedNodeColumns,
    kube_client: KubeClient,
}

impl NodePoller {
    pub fn new(
        tx: Sender<Message>,
        shared_node_columns: SharedNodeColumns,
        kube_client: KubeClient,
    ) -> Self {
        Self {
            tx,
            shared_node_columns,
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
            let node_info = get_node_table(&self.kube_client, &self.shared_node_columns).await;
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

    let path = Node::url_path(&(), None);
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
                        // Label values are filled in a later task; empty for now.
                        NodeColumnSpec::Label { .. } => String::new(),
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
        let table = get_node_table(&client, &shared).await.unwrap();

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
}
