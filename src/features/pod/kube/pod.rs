use std::collections::BTreeMap;

use anyhow::Result;
use async_trait::async_trait;
use crossbeam::channel::Sender;
use futures::future::try_join_all;
use k8s_openapi::{api::core::v1::Pod, Resource as _};
use kube::Resource;
use ratatui::style::{Color, Style};
use regex::Regex;

use crate::{
    features::pod::{message::PodMessage, PodColumn, PodColumnSpec, PodColumns},
    kube::{
        apis::v1_table::TableRow,
        table::{get_resource_per_namespace, insert_ns, KubeTable, KubeTableRow},
        KubeClient,
    },
    logger,
    message::Message,
    ui::widget::ansi_color::style_to_ansi,
    workers::kube::{InfiniteWorker, SharedPodColumns, SharedPodFilter, SharedTargetNamespaces},
};

#[derive(Debug, Clone)]
pub struct PodConfig {
    pub pod_highlight_rules: Vec<PodHighlightRule>,
    pub default_columns: Option<PodColumns>,
}

impl Default for PodConfig {
    fn default() -> Self {
        Self {
            pod_highlight_rules: vec![
                PodHighlightRule {
                    status_regex: Regex::new(r"(Completed|Evicted)").expect("invalid regex"),
                    style: Style::default().fg(Color::DarkGray),
                },
                PodHighlightRule {
                    status_regex: Regex::new(r"(BackOff|Err|Unknown)").expect("invalid regex"),
                    style: Style::default().fg(Color::Red),
                },
            ],
            default_columns: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PodHighlightRule {
    pub status_regex: Regex,
    pub style: Style,
}

#[derive(Clone)]
pub struct PodPoller {
    tx: Sender<Message>,
    shared_target_namespaces: SharedTargetNamespaces,
    shared_pod_columns: SharedPodColumns,
    shared_pod_filter: SharedPodFilter,
    kube_client: KubeClient,
    config: PodConfig,
}

impl PodPoller {
    pub fn new(
        tx: Sender<Message>,
        shared_target_namespaces: SharedTargetNamespaces,
        shared_pod_columns: SharedPodColumns,
        shared_pod_filter: SharedPodFilter,
        kube_client: KubeClient,
        config: PodConfig,
    ) -> Self {
        Self {
            tx,
            shared_target_namespaces,
            shared_pod_columns,
            shared_pod_filter,
            kube_client,
            config,
        }
    }
}

#[async_trait]
impl InfiniteWorker for PodPoller {
    async fn run(&self) {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));

        let Self { tx, .. } = self;

        loop {
            interval.tick().await;

            let pod_info = self.get_pod_info().await;

            if let Err(e) = tx.send(PodMessage::Poll(pod_info).into()) {
                logger!(error, "Failed to send PodMessage::Poll: {}", e);
                return;
            }
        }
    }
}

/// Build the per-row cell vector from a spec list and a k8s API `TableRow`.
///
/// `builtin_indexes` are the positional indexes into `row.cells` that correspond,
/// in order, to each `PodColumnSpec::Builtin` entry in `specs`.
pub(crate) fn build_row_cells(
    specs: &[PodColumnSpec],
    row: &TableRow,
    builtin_indexes: &[usize],
) -> Vec<String> {
    let mut builtin_iter = builtin_indexes.iter();
    specs
        .iter()
        .map(|s| match s {
            PodColumnSpec::Builtin(_) => {
                let i = builtin_iter.next().expect("builtin index available");
                row.cells[*i].to_string()
            }
            PodColumnSpec::Label { key, .. } => row
                .object
                .as_ref()
                .and_then(|o| o.0.get("metadata"))
                .and_then(|m| m.get("labels"))
                .and_then(|l| l.get(key))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        })
        .collect()
}

impl PodPoller {
    async fn get_pod_info(&self) -> Result<KubeTable> {
        let namespaces = self.shared_target_namespaces.read().await;
        let pod_columns = self.shared_pod_columns.read().await;
        let label_selector = self.shared_pod_filter.read().await.clone();

        let jobs = self
            .get_pods_per_namespace(&namespaces, &pod_columns, label_selector.as_deref())
            .await;

        let ok_only: Vec<KubeTableRow> = jobs?.into_iter().flatten().collect();

        let mut display_columns: Vec<String> = pod_columns
            .specs()
            .iter()
            .map(|s| s.header())
            .collect();

        if namespaces.len() != 1 {
            display_columns.insert(0, "NAMESPACE".to_string());
        }

        let mut table = KubeTable {
            header: display_columns,
            ..Default::default()
        };

        table.update_rows(ok_only);

        Ok(table)
    }

    async fn get_pods_per_namespace(
        &self,
        namespaces: &[String],
        pod_columns: &PodColumns,
        label_selector: Option<&str>,
    ) -> Result<Vec<Vec<KubeTableRow>>> {
        let insert_ns = insert_ns(namespaces);

        let name_index = pod_columns
            .specs()
            .iter()
            .position(|s| matches!(s, PodColumnSpec::Builtin(PodColumn::Name)))
            .expect("Name column must be present in pod columns");

        let status_index = pod_columns
            .specs()
            .iter()
            .position(|s| matches!(s, PodColumnSpec::Builtin(PodColumn::Status)));

        let columns: Vec<&str> = pod_columns
            .specs()
            .iter()
            .filter_map(|s| match s {
                PodColumnSpec::Builtin(c) => Some(c.as_str()),
                PodColumnSpec::Label { .. } => None,
            })
            .collect();

        let pod_columns_specs: Vec<PodColumnSpec> = pod_columns.specs().to_vec();

        let label_selector = label_selector.map(|s| s.to_string());

        try_join_all(namespaces.iter().map(|ns| {
            let pod_columns_specs = pod_columns_specs.clone();
            let base_path = Pod::url_path(&Default::default(), Some(ns));
            let path = match label_selector.as_deref().filter(|s| !s.is_empty()) {
                Some(sel) => format!("{}?labelSelector={}", base_path, sel),
                None => base_path,
            };
            get_resource_per_namespace(
                &self.kube_client,
                path,
                &columns,
                move |row: &TableRow, indexes: &[usize]| {
                    let mut row_cells = build_row_cells(&pod_columns_specs, row, indexes);

                    let name = row_cells[name_index].clone();

                    let color = if let Some(index) = status_index {
                        let status = row_cells[index].as_str();

                        self.config
                            .pod_highlight_rules
                            .iter()
                            .find(|rule| rule.status_regex.is_match(status))
                            .map(|rule| style_to_ansi(rule.style))
                    } else {
                        None
                    };

                    if insert_ns {
                        row_cells.insert(0, ns.to_string())
                    }

                    if let Some(color) = color {
                        row_cells
                            .iter_mut()
                            .for_each(|r| *r = format!("{}{}\x1b[0m", color, r))
                    }

                    KubeTableRow {
                        namespace: ns.to_string(),
                        name,
                        row: row_cells,
                        metadata: Some(BTreeMap::from([(
                            "kind".to_string(),
                            Pod::KIND.to_string(),
                        )])),
                    }
                },
            )
        }))
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kube::apis::v1_table::{Value};
    use k8s_openapi::apimachinery::pkg::runtime::RawExtension;
    use pretty_assertions::assert_eq;
    use serde_json::Value as JsonValue;

    fn make_row(cells: &[&str]) -> TableRow {
        TableRow {
            cells: cells
                .iter()
                .map(|c| Value(JsonValue::String(c.to_string())))
                .collect(),
            ..Default::default()
        }
    }

    fn make_row_with_labels(cells: &[&str], labels: &[(&str, &str)]) -> TableRow {
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

    // ── Label arm: present label ─────────────────────────────────────────────

    #[test]
    fn label_arm_returns_value_when_label_present() {
        let specs = vec![
            PodColumnSpec::Builtin(PodColumn::Name),
            PodColumnSpec::Label {
                key: "app".to_string(),
                header: "APP".to_string(),
            },
        ];
        let row = make_row_with_labels(&["my-pod"], &[("app", "nginx")]);
        // builtin_indexes: only one Builtin spec, maps to cell[0]
        let cells = build_row_cells(&specs, &row, &[0]);
        assert_eq!(cells, vec!["my-pod", "nginx"]);
    }

    // ── Label arm: label key absent from metadata.labels ────────────────────

    #[test]
    fn label_arm_returns_empty_when_label_absent() {
        let specs = vec![
            PodColumnSpec::Builtin(PodColumn::Name),
            PodColumnSpec::Label {
                key: "missing-key".to_string(),
                header: "MISSING".to_string(),
            },
        ];
        let row = make_row_with_labels(&["my-pod"], &[("app", "nginx")]);
        let cells = build_row_cells(&specs, &row, &[0]);
        assert_eq!(cells, vec!["my-pod", ""]);
    }

    // ── Label arm: metadata.labels entirely absent ───────────────────────────

    #[test]
    fn label_arm_returns_empty_when_no_metadata_labels() {
        let specs = vec![
            PodColumnSpec::Builtin(PodColumn::Name),
            PodColumnSpec::Label {
                key: "app".to_string(),
                header: "APP".to_string(),
            },
        ];
        // row with no object (no metadata at all)
        let row = make_row(&["my-pod"]);
        let cells = build_row_cells(&specs, &row, &[0]);
        assert_eq!(cells, vec!["my-pod", ""]);
    }

    // ── Label arm: object present but metadata has no labels key ─────────────

    #[test]
    fn label_arm_returns_empty_when_labels_map_absent() {
        let specs = vec![
            PodColumnSpec::Builtin(PodColumn::Name),
            PodColumnSpec::Label {
                key: "app".to_string(),
                header: "APP".to_string(),
            },
        ];
        let object = serde_json::json!({ "metadata": {} });
        let row = TableRow {
            cells: vec![Value(JsonValue::String("my-pod".to_string()))],
            object: Some(RawExtension(object)),
            ..Default::default()
        };
        let cells = build_row_cells(&specs, &row, &[0]);
        assert_eq!(cells, vec!["my-pod", ""]);
    }

    // ── Mixed builtin + label in spec order ──────────────────────────────────

    #[test]
    fn mixed_builtin_and_label_cells_are_in_spec_order() {
        let specs = vec![
            PodColumnSpec::Builtin(PodColumn::Name),
            PodColumnSpec::Label {
                key: "env".to_string(),
                header: "ENV".to_string(),
            },
            PodColumnSpec::Builtin(PodColumn::Status),
            PodColumnSpec::Label {
                key: "team".to_string(),
                header: "TEAM".to_string(),
            },
        ];
        // API table returns Name and Status cells (builtin only)
        // builtin_indexes: Name→cell[0], Status→cell[1]
        let row = make_row_with_labels(
            &["web-pod", "Running"],
            &[("env", "prod"), ("team", "platform")],
        );
        let cells = build_row_cells(&specs, &row, &[0, 1]);
        assert_eq!(cells, vec!["web-pod", "prod", "Running", "platform"]);
    }
}
