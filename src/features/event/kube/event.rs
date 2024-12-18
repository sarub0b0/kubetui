use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time,
};

use anyhow::Result;
use async_trait::async_trait;
use crossbeam::channel::Sender;
use futures::future::try_join_all;

use crate::{
    kube::{
        apis::v1_table::{TableRow, ToTime as _},
        table::{get_resource_per_namespace, insert_ns, KubeTableRow},
        KubeClient,
    },
    message::Message,
    workers::kube::{message::Kube, SharedTargetNamespaces, Worker, WorkerResult},
};

#[derive(Clone)]
pub struct EventPoller {
    is_terminated: Arc<AtomicBool>,
    tx: Sender<Message>,
    shared_target_namespaces: SharedTargetNamespaces,
    kube_client: KubeClient,
}

impl EventPoller {
    pub fn new(
        is_terminated: Arc<AtomicBool>,
        tx: Sender<Message>,
        shared_target_namespaces: SharedTargetNamespaces,
        kube_client: KubeClient,
    ) -> Self {
        Self {
            is_terminated,
            tx,
            shared_target_namespaces,
            kube_client,
        }
    }
}

#[async_trait]
impl Worker for EventPoller {
    type Output = WorkerResult;
    async fn run(&self) -> Self::Output {
        let Self {
            is_terminated,
            tx,
            shared_target_namespaces,
            kube_client,
        } = self;

        let mut interval = tokio::time::interval(time::Duration::from_millis(1000));
        while !is_terminated.load(Ordering::Relaxed) {
            interval.tick().await;
            let target_namespaces = shared_target_namespaces.read().await;

            let event_list = get_event_table(kube_client, &target_namespaces).await;

            tx.send(Message::Kube(Kube::Event(event_list)))
                .expect("Failed to send Kube::Event");
        }

        WorkerResult::Terminated
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
                        item.lines()
                            .for_each(|i| s += &format!("\n\x1b[90m> {}", i));

                        s += "\x1b[0m\n ";
                        // s += &format!("\n\x1b[90m> {}\x1b[0m\n ", item);
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
