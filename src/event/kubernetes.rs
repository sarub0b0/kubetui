use super::{Event, Kube};

use crate::util::*;

use chrono::{DateTime, Duration, Utc};

use bytes::Bytes;
use futures::{StreamExt, TryStreamExt};

use std::time;
use std::{error::Error, thread};
use std::{
    io::BufRead,
    sync::{Arc, RwLock},
};

use crossbeam::channel::{Receiver, Sender};
use tokio::{runtime::Runtime, task::JoinHandle};

use k8s_openapi::api::core::v1::{
    ContainerState, ContainerStateTerminated, ContainerStateWaiting, Namespace, Pod, PodStatus,
};
use kube::{
    api::{ListParams, LogParams, Meta},
    config::Kubeconfig,
    Api, Client, Result,
};

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

        let event_loop = tokio::spawn(event_loop(
            rx,
            tx.clone(),
            tx.clone(),
            Arc::clone(&namespace),
        ));

        let pod_loop = tokio::spawn(pod_loop(tx.clone(), Arc::clone(&namespace)));

        event_loop.await.unwrap();
        pod_loop.await.unwrap();
    });
}

async fn event_loop(
    rx: Receiver<Event>,
    tx_ns: Sender<Event>,
    tx_log: Sender<Event>,
    namespace: Arc<RwLock<String>>,
) {
    let client = Client::try_default().await.unwrap();

    loop {
        let ev = rx.recv().unwrap();
        match ev {
            Event::Kube(ev) => match ev {
                Kube::SetNamespace(ns) => {
                    let selectd_ns = ns.clone();
                    let mut ns = namespace.write().unwrap();
                    *ns = selectd_ns;
                }

                Kube::GetNamespaceRequest => tx_ns
                    .send(Event::Kube(
                        Kube::GetNamespaceResponse(get_namespace_list()),
                    ))
                    .unwrap(),

                Kube::LogRequest(pod_name) => {
                    let client_clone = client.clone();
                    let namespace = namespace.read().unwrap().clone();
                    let pod: Api<Pod> = Api::namespaced(client_clone, &namespace);
                    let lp = LogParams::default();
                    // let mut logs = pod.log_stream(&pod_name, &lp).await.unwrap();
                    let logs = pod.logs(&pod_name, &lp).await.unwrap();

                    // let mut buf: Vec<String> = Vec::with_capacity(1024);

                    let buf = logs.split("\n").map(String::from).collect();
                    // while let Some(line) = logs.try_next().await.unwrap() {
                    //     for line in line.lines() {
                    //         match line {
                    //             Ok(line) => buf.push(line),
                    //             Err(e) => buf.push(e.to_string()),
                    //         }
                    //     }
                    // }

                    tx_log.send(Event::Kube(Kube::LogResponse(buf))).unwrap();
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
}

async fn pod_loop(tx: Sender<Event>, namespace: Arc<RwLock<String>>) {
    let client = Client::try_default().await.unwrap();

    let mut interval = tokio::time::interval(time::Duration::from_secs(1));

    loop {
        interval.tick().await;
        let namespace = namespace.read().unwrap().clone();
        let pod_info = get_pod_info(client.clone(), &namespace).await;
        tx.send(Event::Kube(Kube::Pod(pod_info))).unwrap();
    }
}

async fn get_pod_info(client: Client, namespace: &str) -> Vec<String> {
    let pods: Api<Pod> = Api::namespaced(client, namespace);

    let lp = ListParams::default();

    let pods_list = pods.list(&lp).await.unwrap();

    let max_name_len = match pods_list
        .iter()
        .max_by(|r, l| r.name().len().cmp(&l.name().len()))
    {
        Some(pod) => pod.name().len(),
        None => 0,
    };

    let current_datetime: DateTime<Utc> = Utc::now();

    let mut ret: Vec<String> = Vec::new();
    for p in pods_list {
        let meta = Meta::meta(&p);
        let name = meta.name.clone().unwrap();

        let phase = get_status(p.clone());

        let creation_timestamp: DateTime<Utc> = match &meta.creation_timestamp {
            Some(ref time) => time.0,
            None => current_datetime,
        };
        let duration: Duration = current_datetime - creation_timestamp;

        ret.push(PodInfo::new(name, phase, age(duration)).to_string(max_name_len));
    }
    ret
}

// 参考：https://github.com/astefanutti/kubebox/blob/4ae0a2929a17c132a1ea61144e17b51f93eb602f/lib/kubernetes.js#L7
fn get_status(pod: Pod) -> String {
    if pod.status.is_none() {
        return "".to_string();
    }

    let status = &pod.status.clone().unwrap();
    let meta = Meta::meta(&pod);

    if meta.deletion_timestamp.is_some() {
        return "Terminating".to_string();
    }

    if let Some(reason) = &status.reason {
        if reason == "Evicted" {
            return "Evicted".to_string();
        }
    }

    let phase = status.phase.clone().or(status.reason.clone()).unwrap();

    let (initializing, mut phase) = phase_init_container(&status, phase);
    if !initializing {
        phase = phase_container(&status, phase);
    }
    return phase;
}

fn is_terminated_container(terminated: &Option<ContainerStateTerminated>) -> bool {
    if let Some(terminated) = terminated {
        if terminated.exit_code == 0 {
            return true;
        }
    }
    false
}
fn phase_init_container(status: &PodStatus, phase: impl Into<String>) -> (bool, String) {
    let mut initializing = false;
    let mut phase = phase.into();

    if let Some(cs) = &status.init_container_statuses {
        cs.iter().enumerate().all(|(i, c)| {
            let state = c.state.clone().unwrap();

            let (terminated, waiting) = (state.terminated, state.waiting);

            if is_terminated_container(&terminated) {
                return true;
            }

            initializing = true;

            match terminated {
                Some(terminated) => match &terminated.reason {
                    Some(reason) => phase = format!("Init:{}", reason),
                    None => {
                        phase = if let Some(s) = &terminated.signal {
                            format!("Init:Signal:{}", s)
                        } else {
                            format!("Init:ExitCode:{}", terminated.exit_code)
                        };
                    }
                },
                None => {
                    if let Some(waiting) = waiting {
                        if let Some(reason) = &waiting.reason {
                            if reason != "PodInitializing" {
                                phase = format!("Init:{}", reason);
                            }
                        }
                    }
                }
            }
            phase = format!("Init:{}/{}", i, cs.len());

            false
        });
    }

    (initializing, phase)
}

fn phase_container(status: &PodStatus, phase: impl Into<String>) -> String {
    let mut has_running = false;
    let mut phase = phase.into();

    if let Some(cs) = &status.container_statuses {
        cs.iter().for_each(|c| {
            let state = c.state.clone().unwrap();

            let (running, terminated, waiting) = (state.running, state.terminated, state.waiting);

            let mut signal = None;
            let mut exit_code = 0;

            if let Some(terminated) = &terminated {
                signal = terminated.signal;
                exit_code = terminated.exit_code;
            }

            if let Some(terminated) = &terminated {
                if let Some(reason) = &terminated.reason {
                    phase = reason.clone();
                };
            } else {
                if let Some(waiting) = &waiting {
                    phase = match &waiting.reason {
                        Some(reason) => reason.clone(),
                        None => {
                            if let Some(signal) = signal {
                                format!("Signal:{}", signal)
                            } else {
                                format!("ExitCode:{}", exit_code)
                            }
                        }
                    };
                } else if running.is_some() && c.ready {
                    has_running = true;
                }
            }
        })
    }

    if phase == "Completed" && has_running {
        phase = "Running".to_string();
    }

    return phase;
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

// fn get_logs(
//     client: Client,
//     namespace: &str,
//     pod_name: &str,
//     tx: &Sender<Event>,
// ) -> Box<dyn futures::Stream<Item = Result<Bytes>>> {
//     let pod: Api<Pod> = Api::namespaced(client, namespace);
//     let mut lp = LogParams::default();
//     lp.follow = true;
//     let mut logs = pod.log_stream(pod_name, &lp).await.unwrap();
//     // while let Some(line) = logs.try_next().await.unwrap() {
//     //     print!("{}\r", String::from_utf8_lossy(&line));
//     //     tx.send(Event::Kube(Kube::LogResponse(Some(vec![
//     //         String::from_utf8_lossy(&line).to_string(),
//     //     ]))))
//     //     .unwrap();
//     // }

//     Box::new(logs)

//     // pod.log_stream(pod_name, &lp).await.unwrap()
// }
