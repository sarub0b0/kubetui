use super::{Event, Kube};
use crate::kubernetes::Handlers;
use crate::util::*;

use chrono::{DateTime, Duration, Utc};

use futures::{StreamExt, TryStreamExt};

use std::sync::{Arc, RwLock};
use std::time;

use crossbeam::channel::Sender;

use k8s_openapi::api::core::v1::Event as KEvent;

use kube::{
    api::{ListParams, Meta},
    Api, Client,
};
use kube_runtime::{utils::try_flatten_applied, watcher};

pub async fn event_loop(tx: Sender<Event>, client: Client, namespace: Arc<RwLock<String>>) {
    loop {
        let ns = namespace.read().unwrap().clone();

        let handler = watch(tx.clone(), client.clone(), ns.clone()).await;

        let mut interval = tokio::time::interval(time::Duration::from_millis(500));
        let mut changed_namespace = false;
        while !changed_namespace {
            interval.tick().await;
            let new_ns = namespace.read().unwrap().clone();
            if new_ns != ns {
                changed_namespace = true;
            }
        }

        handler.abort();
    }
}

async fn watch(tx: Sender<Event>, client: Client, ns: String) -> Handlers {
    let events: Api<KEvent> = Api::namespaced(client, &ns);
    let lp = ListParams::default();

    let buf = Arc::new(RwLock::new(Vec::new()));

    let buf_clone = Arc::clone(&buf);
    let watch_handle = tokio::spawn(async move {
        let current_datetime: DateTime<Utc> = Utc::now();
        let mut ew = try_flatten_applied(watcher(events, lp)).boxed();
        while let Some(event) = ew.try_next().await.unwrap() {
            let mut buf = buf_clone.write().unwrap();

            let meta = Meta::meta(&event);
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
            let mut buf = buf_clone.write().unwrap();
            if !buf.is_empty() {
                tx.send(Event::Kube(Kube::Event(buf.clone()))).unwrap();

                buf.clear();
            }
        }
    });

    Handlers(watch_handle, event_handle)
}

pub async fn _event_watch(
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
