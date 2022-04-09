use crate::event::kubernetes::KubeTableRow;

use super::*;

#[derive(Clone)]
pub struct NetworkPollWorker {
    inner: PollWorker,
}

impl NetworkPollWorker {
    pub fn new(inner: PollWorker) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl Worker for NetworkPollWorker {
    type Output = Result<WorkerResult>;

    async fn run(&self) -> Self::Output {
        let mut interval = tokio::time::interval(time::Duration::from_secs(1));

        let is_terminated = &self.inner.is_terminated;
        let tx = &self.inner.tx;

        while !is_terminated.load(std::sync::atomic::Ordering::Relaxed) {
            interval.tick().await;

            let table = self.polling().await;

            tx.send(NetworkMessage::Poll(table).into())?;
        }

        Ok(WorkerResult::Terminated)
    }
}

const POLLING_RESOURCES: [[&str; 3]; 4] = [
    ["Ingress", "ingresses", "apis/networking.k8s.io/v1"],
    ["Service", "services", "api/v1"],
    ["Pod", "pods", "api/v1"],
    [
        "NetworkPolicy",
        "networkpolicies",
        "apis/networking.k8s.io/v1",
    ],
];

impl NetworkPollWorker {
    async fn polling(&self) -> Result<KubeTable> {
        let namespaces = self.inner.namespaces.read().await;
        let mut table = KubeTable {
            header: if namespaces.len() == 1 {
                ["KIND", "NAME", "AGE"]
                    .iter()
                    .map(ToString::to_string)
                    .collect()
            } else {
                ["NAMESPACE", "KIND", "NAME", "AGE"]
                    .iter()
                    .map(ToString::to_string)
                    .collect()
            },
            ..Default::default()
        };

        let jobs =
            try_join_all(POLLING_RESOURCES.iter().map(|&[kind, plural, api]| {
                self.fetch_per_namespace(&namespaces, kind, api, plural)
            }))
            .await?;

        table.update_rows(jobs.into_iter().flatten().collect());

        Ok(table)
    }

    async fn fetch_per_namespace(
        &self,
        namespaces: &[String],
        kind: &str,
        url: &str,
        plural: &str,
    ) -> Result<Vec<KubeTableRow>> {
        let client = &self.inner.kube_client;
        let insert_ns = insert_ns(namespaces);
        let jobs = try_join_all(namespaces.iter().map(|ns| {
            get_resource_per_namespace(
                client,
                format!("{}/namespaces/{}/{}", url, ns, plural),
                &["Name", "Age"],
                move |row: &TableRow, indexes: &[usize]| {
                    let mut row = vec![
                        kind.to_string(),
                        row.cells[indexes[0]].to_string(),
                        row.cells[indexes[1]].to_string(),
                    ];

                    let name = row[1].clone();

                    if insert_ns {
                        row.insert(0, ns.to_string())
                    }

                    KubeTableRow {
                        namespace: ns.to_string(),
                        name,
                        row,
                    }
                },
            )
        }))
        .await?;

        Ok(jobs.into_iter().flatten().collect())
    }
}
