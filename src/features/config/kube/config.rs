use std::{collections::BTreeMap, time};

use crate::{
    features::config::{message::ConfigResponse, ConfigColumn, ConfigColumnSpec, ConfigColumns},
    kube::{
        apis::v1_table::TableRow,
        table::{get_resource_per_namespace, insert_ns, KubeTable, KubeTableRow},
        KubeClient,
    },
    logger,
    message::Message,
    workers::kube::{
        InfiniteWorker,
        SharedConfigColumns,
        SharedConfigFilter,
        SharedTargetNamespaces,
    },
};

use anyhow::Result;
use async_trait::async_trait;
use crossbeam::channel::Sender;
use futures::future::try_join_all;
use k8s_openapi::api::core::v1::{ConfigMap, Secret};
use kube::Resource;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

#[derive(Clone)]
pub struct ConfigPoller {
    tx: Sender<Message>,
    shared_target_namespaces: SharedTargetNamespaces,
    shared_config_columns: SharedConfigColumns,
    shared_config_filter: SharedConfigFilter,
    kube_client: KubeClient,
}

impl ConfigPoller {
    pub fn new(
        tx: Sender<Message>,
        shared_target_namespaces: SharedTargetNamespaces,
        shared_config_columns: SharedConfigColumns,
        shared_config_filter: SharedConfigFilter,
        kube_client: KubeClient,
    ) -> Self {
        Self {
            tx,
            shared_target_namespaces,
            shared_config_columns,
            shared_config_filter,
            kube_client,
        }
    }
}

#[async_trait]
impl InfiniteWorker for ConfigPoller {
    async fn run(&self) {
        let mut interval = tokio::time::interval(time::Duration::from_secs(1));

        let Self {
            tx,
            shared_target_namespaces,
            shared_config_columns,
            shared_config_filter,
            kube_client,
        } = self;

        loop {
            interval.tick().await;

            let target_namespaces = shared_target_namespaces.read().await;
            let columns = shared_config_columns.read().await.clone();
            let label_selector = shared_config_filter.read().await.clone();

            let table = fetch_configs(
                kube_client,
                &target_namespaces,
                &columns,
                label_selector.as_deref(),
            )
            .await;

            if let Err(e) = tx.send(ConfigResponse::Table(table).into()) {
                logger!(error, "Failed to send ConfigResponse::Table: {}", e);
                return;
            }
        }
    }
}

#[derive(Clone, Copy)]
enum Configs {
    ConfigMap,
    Secret,
}

impl Configs {
    fn resource(&self) -> &'static str {
        match self {
            Self::ConfigMap => "ConfigMap",
            Self::Secret => "Secret",
        }
    }

    fn url_path(&self, namespace: &str) -> String {
        match self {
            Self::ConfigMap => ConfigMap::url_path(&Default::default(), Some(namespace)),
            Self::Secret => Secret::url_path(&Default::default(), Some(namespace)),
        }
    }
}

/// Build the per-row cell vector from a spec list, the resource's kind name,
/// and a k8s API `TableRow`.
///
/// `builtin_indexes` are the positional indexes into `row.cells` for the
/// non-KIND builtin columns (NAME / DATA / AGE, in the order specified by
/// the fetch's `target_columns`).
pub(crate) fn build_config_row_cells(
    specs: &[ConfigColumnSpec],
    kind: &str,
    row: &TableRow,
    builtin_indexes: &[usize],
) -> Vec<String> {
    let mut builtin_iter = builtin_indexes.iter();
    specs
        .iter()
        .map(|s| {
            match s {
                ConfigColumnSpec::Builtin(ConfigColumn::Kind) => kind.to_string(),
                ConfigColumnSpec::Builtin(_) => {
                    let i = builtin_iter.next().expect("builtin index available");
                    row.cells[*i].to_string()
                }
                ConfigColumnSpec::Label { key, .. } => {
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
        .collect()
}

async fn fetch_configs(
    client: &KubeClient,
    namespaces: &[String],
    columns: &ConfigColumns,
    label_selector: Option<&str>,
) -> Result<KubeTable> {
    let specs = columns.specs();

    let mut header: Vec<String> = specs.iter().map(|s| s.header()).collect();
    if namespaces.len() != 1 {
        header.insert(0, "NAMESPACE".to_string());
    }

    let jobs = try_join_all([
        fetch_configs_per_namespace(
            client,
            namespaces,
            Configs::ConfigMap,
            specs,
            label_selector,
        ),
        fetch_configs_per_namespace(client, namespaces, Configs::Secret, specs, label_selector),
    ])
    .await?;

    let mut table = KubeTable {
        header,
        ..Default::default()
    };
    table.update_rows(jobs.into_iter().flatten().collect());

    Ok(table)
}

async fn fetch_configs_per_namespace(
    client: &KubeClient,
    namespaces: &[String],
    ty: Configs,
    specs: &[ConfigColumnSpec],
    label_selector: Option<&str>,
) -> Result<Vec<KubeTableRow>> {
    let insert_ns = insert_ns(namespaces);
    let label_selector = label_selector.map(|s| s.to_string());

    // Build target_columns dynamically from specs (skip KIND — supplied by
    // ty.resource()), so the API only fetches what the user currently wants.
    // This keeps `builtin_indexes` aligned with the non-KIND Builtin entries
    // in spec order, even if the user toggles DATA off.
    let target_columns: Vec<&str> = specs
        .iter()
        .filter_map(|s| {
            match s {
                ConfigColumnSpec::Builtin(ConfigColumn::Kind) => None,
                ConfigColumnSpec::Builtin(c) => Some(c.as_str()),
                ConfigColumnSpec::Label { .. } => None,
            }
        })
        .collect();

    // Captures specs for use inside the per-namespace closures.
    let specs_owned: Vec<ConfigColumnSpec> = specs.to_vec();

    let jobs = try_join_all(namespaces.iter().map(|ns| {
        let specs_for_ns = specs_owned.clone();
        let base_path = ty.url_path(ns);
        let path = match label_selector.as_deref().filter(|s| !s.is_empty()) {
            Some(sel) => {
                format!(
                    "{}?labelSelector={}",
                    base_path,
                    utf8_percent_encode(sel, NON_ALPHANUMERIC)
                )
            }
            None => base_path,
        };
        get_resource_per_namespace(
            client,
            path,
            &target_columns,
            move |row: &TableRow, indexes: &[usize]| {
                let mut row_cells =
                    build_config_row_cells(&specs_for_ns, ty.resource(), row, indexes);

                let name_pos = specs_for_ns
                    .iter()
                    .position(|s| matches!(s, ConfigColumnSpec::Builtin(ConfigColumn::Name)))
                    .expect("Name column must be present in config columns");
                let name = row_cells[name_pos].clone();

                if insert_ns {
                    row_cells.insert(0, ns.to_string());
                }

                KubeTableRow {
                    namespace: ns.to_string(),
                    name,
                    row: row_cells,
                    metadata: Some(BTreeMap::from([(
                        "kind".to_string(),
                        ty.resource().to_string(),
                    )])),
                }
            },
        )
    }))
    .await?;

    Ok(jobs.into_iter().flatten().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kube::apis::v1_table::Value;
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
        let mut row = make_row(cells);
        row.object = Some(RawExtension(object));
        row
    }

    #[test]
    fn builtin_only_cells_in_spec_order_with_kind_from_argument() {
        let specs = vec![
            ConfigColumnSpec::Builtin(ConfigColumn::Kind),
            ConfigColumnSpec::Builtin(ConfigColumn::Name),
            ConfigColumnSpec::Builtin(ConfigColumn::Data),
            ConfigColumnSpec::Builtin(ConfigColumn::Age),
        ];
        let row = make_row(&["my-cm", "5", "3h"]);
        let cells = build_config_row_cells(&specs, "ConfigMap", &row, &[0, 1, 2]);
        assert_eq!(cells, vec!["ConfigMap", "my-cm", "5", "3h"]);
    }

    #[test]
    fn label_arm_returns_value_when_label_present() {
        let specs = vec![
            ConfigColumnSpec::Builtin(ConfigColumn::Name),
            ConfigColumnSpec::Label {
                key: "app".to_string(),
                header: "APP".to_string(),
            },
        ];
        let row = make_row_with_labels(&["my-cm"], &[("app", "datadog")]);
        let cells = build_config_row_cells(&specs, "ConfigMap", &row, &[0]);
        assert_eq!(cells, vec!["my-cm", "datadog"]);
    }

    #[test]
    fn label_arm_returns_empty_when_label_absent() {
        let specs = vec![
            ConfigColumnSpec::Builtin(ConfigColumn::Name),
            ConfigColumnSpec::Label {
                key: "app".to_string(),
                header: "APP".to_string(),
            },
        ];
        let row = make_row_with_labels(&["my-cm"], &[("other", "x")]);
        let cells = build_config_row_cells(&specs, "ConfigMap", &row, &[0]);
        assert_eq!(cells, vec!["my-cm", ""]);
    }

    #[test]
    fn label_arm_returns_empty_when_no_object() {
        let specs = vec![
            ConfigColumnSpec::Builtin(ConfigColumn::Name),
            ConfigColumnSpec::Label {
                key: "app".to_string(),
                header: "APP".to_string(),
            },
        ];
        let row = make_row(&["my-cm"]);
        let cells = build_config_row_cells(&specs, "ConfigMap", &row, &[0]);
        assert_eq!(cells, vec!["my-cm", ""]);
    }

    #[test]
    fn mixed_builtin_and_label_in_spec_order() {
        let specs = vec![
            ConfigColumnSpec::Builtin(ConfigColumn::Kind),
            ConfigColumnSpec::Label {
                key: "env".to_string(),
                header: "ENV".to_string(),
            },
            ConfigColumnSpec::Builtin(ConfigColumn::Name),
            ConfigColumnSpec::Label {
                key: "team".to_string(),
                header: "TEAM".to_string(),
            },
            ConfigColumnSpec::Builtin(ConfigColumn::Age),
        ];
        // builtin order in row.cells (from target_columns derived from spec):
        // Name (0), Age (1). Data is skipped because spec doesn't include it.
        let row = make_row_with_labels(&["my-cm", "3h"], &[("env", "prod"), ("team", "platform")]);
        let cells = build_config_row_cells(&specs, "Secret", &row, &[0, 1]);
        assert_eq!(cells, vec!["Secret", "prod", "my-cm", "platform", "3h"]);
    }
}
