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
                    let mut builtin_iter = indexes.iter();
                    let mut row_cells: Vec<String> = pod_columns_specs
                        .iter()
                        .map(|s| match s {
                            PodColumnSpec::Builtin(_) => {
                                let i = builtin_iter.next().expect("builtin index available");
                                row.cells[*i].to_string()
                            }
                            PodColumnSpec::Label { .. } => String::new(),
                        })
                        .collect();

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
