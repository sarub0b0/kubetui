use super::{
    v1_table::*,
    KubeArgs, Namespaces, {Event, Kube},
};

use std::{sync::Arc, time};

use crossbeam::channel::Sender;
use futures::future::join_all;

use kube::Client;

pub async fn event_loop(tx: Sender<Event>, namespaces: Namespaces, args: Arc<KubeArgs>) {
    let mut interval = tokio::time::interval(time::Duration::from_millis(1000));
    loop {
        interval.tick().await;
        let ns = namespaces.read().await;

        let event_list = get_event_table(&args.client, &args.server_url, &ns).await;

        tx.send(Event::Kube(Kube::Event(event_list))).unwrap();
    }
}

const TARGET_LEN: usize = 4;
const TARGET: [&str; TARGET_LEN] = ["Last Seen", "Object", "Reason", "Message"];

async fn get_event_table(client: &Client, server_url: &str, namespaces: &[String]) -> Vec<String> {
    let insert_ns = insert_ns(namespaces);

    let jobs = join_all(namespaces.iter().map(|ns| {
        get_resourse_per_namespace(
            &client,
            &server_url,
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
    }));

    let mut data: Vec<Vec<String>> = jobs.await.into_iter().flatten().collect();

    data.sort_by_key(|row| row[0].to_time());

    data.iter()
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
        .collect()
}
