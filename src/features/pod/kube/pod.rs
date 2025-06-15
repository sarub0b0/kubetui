use std::collections::BTreeMap;

use anyhow::Result;
use async_trait::async_trait;
use crossbeam::channel::Sender;
use futures::future::try_join_all;
use k8s_openapi::{api::core::v1::Pod, Resource as _};
use ratatui::style::{Color, Style};
use regex::Regex;

use crate::{
    features::pod::{message::PodMessage, PodColumn, PodColumns},
    kube::{
        apis::v1_table::TableRow,
        table::{get_resource_per_namespace, insert_ns, KubeTable, KubeTableRow},
        KubeClient,
    },
    message::Message,
    ui::widget::ansi_color::style_to_ansi,
    workers::kube::{SharedPodColumns, SharedTargetNamespaces, Worker, WorkerResult},
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
    kube_client: KubeClient,
    config: PodConfig,
}

impl PodPoller {
    pub fn new(
        tx: Sender<Message>,
        shared_target_namespaces: SharedTargetNamespaces,
        shared_pod_columns: SharedPodColumns,
        kube_client: KubeClient,
        config: PodConfig,
    ) -> Self {
        Self {
            tx,
            shared_target_namespaces,
            shared_pod_columns,
            kube_client,
            config,
        }
    }
}

#[async_trait]
impl Worker for PodPoller {
    type Output = WorkerResult;

    async fn run(&self) -> Self::Output {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));

        let Self { tx, .. } = self;

        loop {
            interval.tick().await;

            let pod_info = self.get_pod_info().await;

            tx.send(PodMessage::Poll(pod_info).into())
                .expect("Failed to Kube::Pod");
        }
    }
}

impl PodPoller {
    async fn get_pod_info(&self) -> Result<KubeTable> {
        let namespaces = self.shared_target_namespaces.read().await;
        let pod_columns = self.shared_pod_columns.read().await;

        let jobs = self.get_pods_per_namespace(&namespaces, &pod_columns).await;

        let ok_only: Vec<KubeTableRow> = jobs?.into_iter().flatten().collect();

        let mut display_columns: Vec<String> = pod_columns
            .columns
            .iter()
            .map(|col| col.display().to_string())
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
    ) -> Result<Vec<Vec<KubeTableRow>>> {
        let insert_ns = insert_ns(namespaces);

        let name_index = pod_columns
            .columns
            .iter()
            .position(|&col| col == PodColumn::Name)
            .expect("Name column must be present in pod columns");

        let status_index = pod_columns
            .columns
            .iter()
            .position(|&col| col == PodColumn::Status);

        let columns = pod_columns
            .columns
            .iter()
            .map(|col| col.as_str())
            .collect::<Vec<_>>();

        try_join_all(namespaces.iter().map(|ns| {
            get_resource_per_namespace(
                &self.kube_client,
                format!("api/v1/namespaces/{}/{}", ns, "pods"),
                &columns,
                move |row: &TableRow, indexes: &[usize]| {
                    let mut row: Vec<String> =
                        indexes.iter().map(|i| row.cells[*i].to_string()).collect();

                    let name = row[name_index].clone();

                    let color = if let Some(index) = status_index {
                        let status = row[index].as_str();

                        self.config
                            .pod_highlight_rules
                            .iter()
                            .find(|rule| rule.status_regex.is_match(status))
                            .map(|rule| style_to_ansi(rule.style))
                    } else {
                        None
                    };

                    if insert_ns {
                        row.insert(0, ns.to_string())
                    }

                    if let Some(color) = color {
                        row.iter_mut()
                            .for_each(|r| *r = format!("{}{}\x1b[0m", color, r))
                    }

                    KubeTableRow {
                        namespace: ns.to_string(),
                        name,
                        row,
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
