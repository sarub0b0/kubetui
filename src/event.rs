use super::util::*;

#[allow(unused_imports)]
use chrono::{DateTime, Duration, Utc};

#[allow(unused_imports)]
use std::sync::{
    mpsc::{self, Receiver, Sender},
    Arc, RwLock,
};
use std::thread;
use std::time;

#[allow(unused_imports)]
use tokio::runtime::Runtime;

#[allow(unused_imports)]
use std::{
    error::Error,
    io::{self, stdout, Write},
};

#[allow(unused_imports)]
use crossterm::{
    event::{
        self, poll, read, DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyCode,
        KeyEvent, KeyModifiers,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

#[allow(unused_imports)]
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Corner, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets, Frame, Terminal,
};

#[allow(unused_imports)]
use k8s_openapi::{
    api::core::v1::{Namespace, Pod, PodStatus},
    apimachinery::pkg::apis::meta::v1::Time,
};
use kube::{
    api::{ListParams, LogParams, Meta},
    config::Kubeconfig,
    Api, Client,
};

pub enum Event {
    Input(KeyEvent),
    Kube(Kube),
    Tick,
    Resize,
    Mouse,
}

pub enum Kube {
    Pod(Vec<String>),
    Namespace(Option<Vec<String>>),
    LogRequest(String),
    LogResponse(Option<Vec<String>>),
}

pub struct PodInfo {
    name: String,
    phase: String,
    age: String,
}

impl PodInfo {
    fn new(name: String, phase: String, age: String) -> Self {
        Self { name, phase, age }
    }

    fn to_string(&self, width: usize) -> String {
        format!(
            "{:width$} {}    {}",
            self.name,
            self.phase,
            self.age,
            width = width + 4
        )
    }
}

async fn get_pod_info(client: Client, namespace: &str) -> Vec<String> {
    let pods: Api<Pod> = Api::namespaced(client, namespace);
    let lp = ListParams::default();

    let pods_list = pods.list(&lp).await.unwrap();

    let max_name_len = pods_list
        .iter()
        .max_by(|r, l| r.name().len().cmp(&l.name().len()))
        .unwrap()
        .name()
        .len();

    let current_datetime: DateTime<Utc> = Utc::now();

    let mut ret: Vec<String> = Vec::new();
    for p in pods_list {
        let meta = Meta::meta(&p);
        let status = &p.status;
        let name = meta.name.clone().unwrap();

        let phase = match status {
            Some(s) => s.phase.clone().unwrap(),
            None => "Unknown".to_string(),
        };
        let creation_timestamp: DateTime<Utc> = match &meta.creation_timestamp {
            Some(ref time) => time.0,
            None => current_datetime,
        };
        let duration: Duration = current_datetime - creation_timestamp;

        ret.push(PodInfo::new(name, phase, age(&duration)).to_string(max_name_len));
    }
    ret
}

pub fn read_key(tx: Sender<Event>) {
    loop {
        match read().unwrap() {
            CEvent::Key(ev) => tx.send(Event::Input(ev)).unwrap(),
            CEvent::Mouse(_) => tx.send(Event::Mouse).unwrap(),
            CEvent::Resize(_, _) => tx.send(Event::Resize).unwrap(),
        }
    }
}

// TODO: spawnを削除する <20-02-21, yourname> //
fn get_namespace_list() -> Option<Vec<String>> {
    let th = thread::spawn(move || {
        let rt = Runtime::new().unwrap();
        rt.block_on(async move {
            let client = Client::try_default().await.unwrap();
            let namespaces: Api<Namespace> = Api::all(client);

            let lp = ListParams::default();

            let ns_list = namespaces.list(&lp).await.unwrap();

            ns_list.iter().map(|ns| ns.name()).collect::<Vec<String>>()
        })
    });

    Some(th.join().unwrap())
}

async fn get_logs(client: Client, namespace: &str, pod_name: &str) -> Option<Vec<String>> {
    let pod: Api<Pod> = Api::namespaced(client, namespace);
    let lp = LogParams::default();
    let logs = pod.logs(pod_name, &lp).await;

    match logs {
        Ok(logs) => Some(logs.lines().map(String::from).collect()),
        Err(_) => None,
    }
}

pub fn kube_process(tx: Sender<Event>, rx: Receiver<Event>) {
    let rt = Runtime::new().unwrap();

    rt.block_on(async move {
        let kubeconfig = Kubeconfig::read().unwrap();
        let current_context = kubeconfig.current_context.unwrap();

        let current_context = kubeconfig
            .contexts
            .iter()
            .find(|n| n.name == current_context);

        let namespace = Arc::new(RwLock::new(
            current_context.unwrap().clone().context.namespace.unwrap(),
        ));

        let tx_pod = tx.clone();
        let tx_log = tx.clone();
        let tx_ns = tx.clone();

        let namespace_event_loop = Arc::clone(&namespace);
        let namespace_pod_loop = Arc::clone(&namespace);

        let event_loop = tokio::spawn(async move {
            let client = Client::try_default().await.unwrap();
            loop {
                let ev = rx.recv().unwrap();
                match ev {
                    Event::Kube(ev) => match ev {
                        Kube::Namespace(_) => tx_ns
                            .send(Event::Kube(Kube::Namespace(get_namespace_list())))
                            .unwrap(),

                        Kube::LogRequest(pod_name) => {
                            let client_clone = client.clone();
                            let namespace = namespace_event_loop.read().unwrap().clone();
                            let logs = get_logs(client_clone, &namespace, &pod_name).await;

                            tx_log.send(Event::Kube(Kube::LogResponse(logs))).unwrap();
                        }
                        _ => {
                            unreachable!()
                        }
                    },
                    _ => {
                        unreachable!()
                    }
                }
            }
        });

        let pod_loop = tokio::spawn(async move {
            let mut interval = tokio::time::interval(time::Duration::from_secs(1));
            let client = Client::try_default().await.unwrap();
            loop {
                interval.tick().await;
                let namespace = namespace_pod_loop.read().unwrap().clone();
                let pod_info = get_pod_info(client.clone(), &namespace).await;
                tx_pod.send(Event::Kube(Kube::Pod(pod_info))).unwrap();
            }
        });

        event_loop.await.unwrap();
        pod_loop.await.unwrap();
    });
}

pub fn tick(tx: Sender<Event>, rate: time::Duration) {
    let rt = Runtime::new().unwrap();

    rt.block_on(async move {
        let mut interval = tokio::time::interval(rate);
        loop {
            interval.tick().await;

            tx.send(Event::Tick).unwrap();
        }
    });
}
