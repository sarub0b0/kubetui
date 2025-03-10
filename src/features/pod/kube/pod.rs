use std::collections::BTreeMap;

use anyhow::Result;
use async_trait::async_trait;
use crossbeam::channel::Sender;
use futures::future::try_join_all;
use k8s_openapi::{api::core::v1::Pod, Resource as _};
use ratatui::style::{Color, Style};
use regex::Regex;

use crate::{
    kube::{
        apis::v1_table::TableRow,
        table::{get_resource_per_namespace, insert_ns, KubeTable, KubeTableRow},
        KubeClient,
    },
    message::Message,
    ui::widget::ansi_color::style_to_ansi,
    workers::kube::{message::Kube, SharedTargetNamespaces, Worker, WorkerResult},
};

#[derive(Debug, Clone)]
pub struct PodConfig {
    pub pod_highlight_rules: Vec<PodHighlightRule>,
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
    kube_client: KubeClient,
    config: PodConfig,
}

impl PodPoller {
    pub fn new(
        tx: Sender<Message>,
        shared_target_namespaces: SharedTargetNamespaces,
        kube_client: KubeClient,
        config: PodConfig,
    ) -> Self {
        Self {
            tx,
            shared_target_namespaces,
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

            tx.send(Message::Kube(Kube::Pod(pod_info)))
                .expect("Failed to Kube::Pod");
        }
    }
}

impl PodPoller {
    async fn get_pod_info(&self) -> Result<KubeTable> {
        let namespaces = self.shared_target_namespaces.read().await;

        let jobs = self.get_pods_per_namespace(&namespaces).await;

        let ok_only: Vec<KubeTableRow> = jobs?.into_iter().flatten().collect();

        let mut table = KubeTable {
            header: if namespaces.len() == 1 {
                ["NAME", "READY", "STATUS", "AGE"]
                    .iter()
                    .map(ToString::to_string)
                    .collect()
            } else {
                ["NAMESPACE", "NAME", "READY", "STATUS", "AGE"]
                    .iter()
                    .map(ToString::to_string)
                    .collect()
            },
            ..Default::default()
        };

        table.update_rows(ok_only);

        Ok(table)
    }

    async fn get_pods_per_namespace(
        &self,
        namespaces: &[String],
    ) -> Result<Vec<Vec<KubeTableRow>>> {
        let insert_ns = insert_ns(namespaces);
        try_join_all(namespaces.iter().map(|ns| {
            get_resource_per_namespace(
                &self.kube_client,
                format!("api/v1/namespaces/{}/{}", ns, "pods"),
                &["Name", "Ready", "Status", "Age"],
                move |row: &TableRow, indexes: &[usize]| {
                    let mut row: Vec<String> =
                        indexes.iter().map(|i| row.cells[*i].to_string()).collect();

                    let name = row[0].clone();

                    let status = row[2].as_str();

                    let color = self
                        .config
                        .pod_highlight_rules
                        .iter()
                        .find(|rule| rule.status_regex.is_match(status))
                        .map(|rule| style_to_ansi(rule.style));

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
