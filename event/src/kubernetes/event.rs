use super::{
    request::get_table_request,
    v1_table::*,
    KubeArgs, Namespaces, {Event, Kube},
};

use std::{sync::Arc, time};

use crossbeam::channel::Sender;

use kube::Client;

pub async fn event_loop(tx: Sender<Event>, namespaces: Namespaces, args: Arc<KubeArgs>) {
    let mut interval = tokio::time::interval(time::Duration::from_millis(1000));
    loop {
        interval.tick().await;
        let ns = namespaces.read().await;

        let event_list = get_event_list(args.client.clone(), &ns[0], &args.server_url).await;

        tx.send(Event::Kube(Kube::Event(event_list))).unwrap();
    }
}

const TARGET_LEN: usize = 4;
const TARGET: [&str; TARGET_LEN] = ["Last Seen", "Object", "Reason", "Message"];

async fn get_event_list(client: Client, ns: &str, server_url: &str) -> Vec<String> {
    let table: Result<Table, kube::Error> = client
        .request(
            get_table_request(
                server_url,
                &format!("api/v1/namespaces/{}/{}", ns, "events"),
            )
            .unwrap(),
        )
        .await;

    match table {
        Ok(mut t) => {
            t.sort_rows_by_time(t.find_index(TARGET[0]).unwrap());

            let vec: Vec<Vec<String>> = t
                .rows
                .iter()
                .map(|row| {
                    t.find_indexes(&TARGET)
                        .iter()
                        .map(|i| row.cells[*i].to_string())
                        .collect::<Vec<String>>()
                })
                .collect();

            vec.iter()
                .map(|v| {
                    v.iter()
                        .enumerate()
                        .fold(String::new(), |mut s: String, (i, item)| -> String {
                            if i == TARGET_LEN - 1 {
                                s += &format!("\n\x1b[90m> {}\x1b[0m\n ", item)
                            } else {
                                s += &format!("{:<4}  ", item)
                            }
                            s
                        })
                })
                .collect()
        }

        Err(e) => return vec![format!("{}", e)],
    }
}
