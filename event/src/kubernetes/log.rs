use super::{Event, Kube};
use crate::kubernetes::Handlers;

use futures::{StreamExt, TryStreamExt};

use std::sync::{Arc, RwLock};
use std::time;

use crossbeam::channel::Sender;

use k8s_openapi::api::core::v1::{Event as KEvent, Pod};

use kube::{
    api::{ListParams, LogParams},
    Api, Client,
};
use kube_runtime::{utils::try_flatten_applied, watcher};

pub async fn log_stream(
    tx: Sender<Event>,
    client: Client,
    ns: String,
    pod_name: String,
) -> Handlers {
    let pod: Api<Pod> = Api::namespaced(client.clone(), &ns);
    let mut lp = LogParams::default();

    lp.follow = true;

    let mut logs = pod.log_stream(&pod_name, &lp).await.unwrap().boxed();
    // バッチでログストリームを渡す
    let buf = Arc::new(RwLock::new(Vec::new()));

    let buf_clone = Arc::clone(&buf);
    let stream_handler = tokio::spawn(async move {
        while let Some(line) = logs.try_next().await.unwrap() {
            let mut buf = buf_clone.write().unwrap();
            buf.push(String::from_utf8_lossy(&line).to_string());
        }
    });

    let buf_clone = Arc::clone(&buf);
    let event_handler = tokio::spawn(async move {
        let mut interval = tokio::time::interval(time::Duration::from_millis(200));
        loop {
            interval.tick().await;
            let mut buf = buf_clone.write().unwrap();
            if !buf.is_empty() {
                tx.send(Event::Kube(Kube::LogStreamResponse(buf.clone())))
                    .unwrap();

                buf.clear();
            }
        }
    });

    Handlers(stream_handler, event_handler)
}

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
            let mut buf = buf_clone.write().unwrap();
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
            let mut buf = buf_clone.write().unwrap();
            if !buf.is_empty() {
                tx.send(Event::Kube(Kube::LogStreamResponse(buf.clone())))
                    .unwrap();

                buf.clear();
            }
        }
    });

    Handlers(watch_handle, event_handle)
}
