use super::request::get_table_request;
use super::v1_table::*;
use super::{Event, Kube};
use crate::kubernetes::Handlers;
use crate::util::*;

use chrono::{DateTime, Duration, Utc};

use futures::{StreamExt, TryStreamExt};

use std::sync::Arc;
use std::time;

use tokio::sync::RwLock;

use crossbeam::channel::Sender;

use k8s_openapi::api::core::v1::Event as KEvent;

use kube::{
    api::{ListParams, Resource},
    Api, Client,
};

use kube_runtime::{utils::try_flatten_applied, watcher};

pub async fn event_loop(
    tx: Sender<Event>,
    client: Client,
    namespace: Arc<RwLock<String>>,
    server_url: String,
) {
    let mut interval = tokio::time::interval(time::Duration::from_millis(1000));
    loop {
        interval.tick().await;
        let ns = namespace.read().await;

        let event_list = get_event_list(client.clone(), &ns, &server_url).await;

        tx.send(Event::Kube(Kube::Event(event_list))).unwrap();
    }
}

const TARGET_LEN: usize = 4;
const TARGET: [&str; TARGET_LEN] = ["Last Seen", "Object", "Reason", "Message"];

async fn get_event_list(client: Client, ns: &str, server_url: &str) -> Vec<String> {
    let table: Result<Table, kube::Error> = client
        .request(get_table_request(server_url, ns, "events").unwrap())
        .await;

    match table {
        Ok(t) => {
            let vec: Vec<Vec<&str>> = t
                .rows
                .iter()
                .map(|row| {
                    t.find_indexes(&TARGET)
                        .iter()
                        .filter_map(|i| row.cells[*i].as_str())
                        .collect::<Vec<&str>>()
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

#[allow(dead_code)]
async fn watch(tx: Sender<Event>, client: Client, ns: String) -> Handlers {
    let events: Api<KEvent> = Api::namespaced(client, &ns);
    let lp = ListParams::default();

    let buf = Arc::new(RwLock::new(Vec::new()));

    let buf_clone = Arc::clone(&buf);
    let watch_handle = tokio::spawn(async move {
        // タイムアウト時に再接続を試みる
        let current_datetime: DateTime<Utc> = Utc::now();
        let mut ew = try_flatten_applied(watcher(events, lp)).boxed();
        while let Some(event) = ew.try_next().await.unwrap() {
            let mut buf = buf_clone.write().await;

            let meta = event.meta();

            let creation_timestamp: DateTime<Utc> = match &meta.creation_timestamp {
                Some(ref time) => time.0,
                None => current_datetime,
            };
            let duration: Duration = current_datetime - creation_timestamp;

            buf.push(format!("{:4} {}", age(duration), event.message.unwrap()));
        }
    });

    let buf_clone = Arc::clone(&buf);
    let event_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(time::Duration::from_millis(500));
        loop {
            interval.tick().await;
            let mut buf = buf_clone.write().await;
            if !buf.is_empty() {
                tx.send(Event::Kube(Kube::Event(buf.clone()))).unwrap();

                buf.clear();
            }
        }
    });

    Handlers(vec![watch_handle, event_handle])
}

#[allow(dead_code)]
pub async fn event_watch(
    tx: Sender<Event>,
    client: Client,
    ns: String,
    object_name: impl Into<String>,
    kind: impl Into<String>,
) -> Handlers {
    let events: Api<KEvent> = Api::namespaced(client, &ns);
    let lp = ListParams::default().fields(&format!(
        "involvedObject.kind={},involvedObject.name={}",
        kind.into(),
        object_name.into()
    ));

    let buf = Arc::new(RwLock::new(Vec::new()));

    let buf_clone = Arc::clone(&buf);
    let watch_handle = tokio::spawn(async move {
        let mut ew = try_flatten_applied(watcher(events, lp)).boxed();
        while let Some(event) = ew.try_next().await.unwrap() {
            let mut buf = buf_clone.write().await;
            buf.push(format!(
                "{} {} {}",
                event.type_.unwrap(),
                event.reason.unwrap(),
                event.message.unwrap()
            ));
        }
    });

    let buf_clone = Arc::clone(&buf);
    let event_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(time::Duration::from_millis(500));
        loop {
            interval.tick().await;
            let mut buf = buf_clone.write().await;
            if !buf.is_empty() {
                tx.send(Event::Kube(Kube::LogStreamResponse(buf.clone())))
                    .unwrap();

                buf.clear();
            }
        }
    });

    Handlers(vec![watch_handle, event_handle])
}
