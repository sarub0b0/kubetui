use super::{
    v1_table::*,
    worker::{PollWorker, Worker},
    KubeClient, KubeTableRow, WorkerResult, {Event, Kube},
};

use std::time;

use futures::future::try_join_all;

use async_trait::async_trait;

use crate::error::Result;

#[derive(Clone)]
pub struct EventPollWorker {
    inner: PollWorker,
}

impl EventPollWorker {
    pub fn new(inner: PollWorker) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl Worker for EventPollWorker {
    type Output = Result<WorkerResult>;
    async fn run(&self) -> Self::Output {
        let Self {
            inner:
                PollWorker {
                    is_terminated,
                    tx,
                    shared_target_namespaces,
                    kube_client,
                },
        } = self;

        let mut interval = tokio::time::interval(time::Duration::from_millis(1000));
        while !is_terminated.load(std::sync::atomic::Ordering::Relaxed) {
            interval.tick().await;
            let target_namespaces = shared_target_namespaces.read().await;

            let event_list = get_event_table(kube_client, &target_namespaces).await;

            tx.send(Event::Kube(Kube::Event(event_list)))
                .expect("Failed to send Kube::Event");
        }

        Ok(WorkerResult::Terminated)
    }
}

const TARGET_LEN: usize = 4;
const TARGET: [&str; TARGET_LEN] = ["Last Seen", "Object", "Reason", "Message"];

async fn get_event_table(client: &KubeClient, namespaces: &[String]) -> Result<Vec<String>> {
    let insert_ns = insert_ns(namespaces);

    let jobs = try_join_all(namespaces.iter().map(|ns| {
        get_resource_per_namespace(
            client,
            format!("api/v1/namespaces/{}/{}", ns, "events"),
            &TARGET,
            move |row: &TableRow, indexes: &[usize]| {
                let mut row: Vec<String> =
                    indexes.iter().map(|i| row.cells[*i].to_string()).collect();

                let name = row[0].clone();

                if insert_ns {
                    row.insert(1, ns.to_string())
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
    .await?;

    let mut ok_only: Vec<KubeTableRow> = jobs.into_iter().flatten().collect();

    ok_only.sort_by_key(|row| row.row[0].to_time());

    Ok(ok_only
        .iter()
        .flat_map(|v| {
            v.row
                .iter()
                .enumerate()
                .fold(String::new(), |mut s: String, (i, item)| -> String {
                    if i == v.row.len() - 1 {
                        s += &format!("\n\x1b[90m> {}\x1b[0m\n ", item);
                    } else {
                        s += &format!("{:<4}  ", item);
                    }
                    s
                })
                .lines()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .collect())
}
