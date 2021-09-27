use super::{
    v1_table::*,
    KubeArgs, Namespaces, WorkerResult, {Event, Kube},
};

use std::{sync::Arc, time};

use crossbeam::channel::Sender;
use futures::future::try_join_all;

use kube::Client;

use crate::error::Result;

pub async fn event_loop(
    tx: Sender<Event>,
    namespaces: Namespaces,
    args: Arc<KubeArgs>,
) -> Result<WorkerResult> {
    let mut interval = tokio::time::interval(time::Duration::from_millis(1000));
    while !args
        .is_terminated
        .load(std::sync::atomic::Ordering::Relaxed)
    {
        interval.tick().await;
        let ns = namespaces.read().await;

        let event_list = get_event_table(&args.client, &args.server_url, &ns).await;

        tx.send(Event::Kube(Kube::Event(event_list))).unwrap();
    }

    Ok(WorkerResult::Terminated)
}

const TARGET_LEN: usize = 4;
const TARGET: [&str; TARGET_LEN] = ["Last Seen", "Object", "Reason", "Message"];

async fn get_event_table(
    client: &Client,
    server_url: &str,
    namespaces: &[String],
) -> Result<Vec<String>> {
    let insert_ns = insert_ns(namespaces);

    let jobs = try_join_all(namespaces.iter().map(|ns| {
        get_resource_per_namespace(
            client,
            server_url,
            format!("api/v1/namespaces/{}/{}", ns, "events"),
            &TARGET,
            move |row: &TableRow, indexes: &[usize]| {
                let mut cells: Vec<String> =
                    indexes.iter().map(|i| row.cells[*i].to_string()).collect();

                if insert_ns {
                    cells.insert(1, ns.to_string())
                }

                cells
            },
        )
    }))
    .await?;

    let mut ok_only: Vec<Vec<String>> = jobs.into_iter().flatten().collect();

    ok_only.sort_by_key(|row| row[0].to_time());

    Ok(ok_only
        .iter()
        .map(|v| {
            v.iter()
                .enumerate()
                .fold(String::new(), |mut s: String, (i, item)| -> String {
                    if i == v.len() - 1 {
                        s += &format!("\n\x1b[90m> {}\x1b[0m\n ", item);
                    } else {
                        s += &format!("{:<4}  ", item);
                    }
                    s
                })
        })
        .collect())
}
