use anyhow::Result;
use async_trait::async_trait;
use futures::future::try_join_all;

use crate::{
    kube::{
        apis::v1_table::TableRow,
        table::{get_resource_per_namespace, insert_ns, KubeTable, KubeTableRow},
    },
    message::Message,
    workers::kube::{
        message::Kube,
        worker::{PollerBase, Worker},
        WorkerResult,
    },
};

#[derive(Clone)]
pub struct PodPoller {
    base: PollerBase,
}

impl PodPoller {
    pub fn new(base: PollerBase) -> Self {
        Self { base }
    }
}

#[async_trait]
impl Worker for PodPoller {
    type Output = WorkerResult;

    async fn run(&self) -> Self::Output {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));

        let Self {
            base: PollerBase {
                is_terminated, tx, ..
            },
        } = self;

        while !is_terminated.load(std::sync::atomic::Ordering::Relaxed) {
            interval.tick().await;

            let pod_info = self.get_pod_info().await;

            tx.send(Message::Kube(Kube::Pod(pod_info)))
                .expect("Failed to Kube::Pod");
        }

        WorkerResult::Terminated
    }
}

impl PodPoller {
    async fn get_pod_info(&self) -> Result<KubeTable> {
        let namespaces = self.base.shared_target_namespaces.read().await;

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
                &self.base.kube_client,
                format!("api/v1/namespaces/{}/{}", ns, "pods"),
                &["Name", "Ready", "Status", "Age"],
                move |row: &TableRow, indexes: &[usize]| {
                    let mut row: Vec<String> =
                        indexes.iter().map(|i| row.cells[*i].to_string()).collect();

                    let name = row[0].clone();

                    let color = match row[2].as_str() {
                        s if s == "Completed" || s.contains("Evicted") => Some(90),
                        s if s.contains("BackOff")
                            || s.contains("Err")
                            || s.contains("Unknown") =>
                        {
                            Some(31)
                        }
                        _ => None,
                    };

                    if insert_ns {
                        row.insert(0, ns.to_string())
                    }

                    if let Some(color) = color {
                        row.iter_mut()
                            .for_each(|r| *r = format!("\x1b[{}m{}\x1b[0m", color, r))
                    }

                    KubeTableRow {
                        namespace: ns.to_string(),
                        name,
                        row,
                        ..Default::default()
                    }
                },
            )
        }))
        .await
    }
}
