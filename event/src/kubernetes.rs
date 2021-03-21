use super::{Event, Kube};

use crate::util::*;

use chrono::{DateTime, Duration, Utc};

use futures::{StreamExt, TryStreamExt};

use std::sync::{Arc, RwLock};
use std::thread;
use std::{time, vec};

use crossbeam::channel::{Receiver, Sender};
use tokio::{
    runtime::Runtime,
    task::{self, JoinHandle},
};

use k8s_openapi::api::core::v1::{
    ConfigMap, ContainerStateTerminated, Namespace, Pod, PodStatus, Secret,
};

use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;

use kube::{
    api::{ListParams, LogParams, Meta},
    config::Kubeconfig,
    Api, Client,
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

    fn to_string(&self, width_name: usize, width_phase: usize) -> String {
        format!(
            "{:width_name$}{:width_phase$}{}",
            self.name,
            self.phase,
            self.age,
            width_name = width_name + 2,
            width_phase = width_phase + 2,
        )
    }
}

struct LogStreamHandler(JoinHandle<()>, JoinHandle<()>);

pub fn kube_process(tx: Sender<Event>, rx: Receiver<Event>) {
    let rt = Runtime::new().unwrap();

    rt.block_on(async move {
        let kubeconfig = Kubeconfig::read().unwrap();
        let current_context = kubeconfig.current_context.unwrap();

        let named_context = kubeconfig
            .contexts
            .iter()
            .find(|n| n.name == current_context);

        let namespace = Arc::new(RwLock::new(
            named_context.unwrap().clone().context.namespace.unwrap(),
        ));

        let event_loop = tokio::spawn(event_loop(
            rx,
            tx.clone(),
            Arc::clone(&namespace),
            current_context,
        ));

        let pod_loop = tokio::spawn(pod_loop(tx.clone(), Arc::clone(&namespace)));

        let configs_loop = tokio::spawn(configs_loop(tx.clone(), Arc::clone(&namespace)));
        event_loop.await.unwrap();
        pod_loop.await.unwrap();
        configs_loop.await.unwrap();
    });
}

async fn event_loop(
    rx: Receiver<Event>,
    tx: Sender<Event>,
    namespace: Arc<RwLock<String>>,
    current_context: String,
) {
    let client = Client::try_default().await.unwrap();
    let tx_ns = tx.clone();
    let tx_config = tx.clone();
    let tx_ctx = tx.clone();

    let mut log_stream_handler: Option<JoinHandle<LogStreamHandler>> = None;
    loop {
        let ev = rx.recv().unwrap();
        match ev {
            Event::Kube(ev) => match ev {
                Kube::SetNamespace(ns) => {
                    let selectd_ns = ns.clone();
                    let mut ns = namespace.write().unwrap();
                    *ns = selectd_ns;

                    tx_ctx
                        .send(Event::Kube(Kube::CurrentContextResponse(
                            current_context.to_string(),
                            ns.clone(),
                        )))
                        .unwrap();
                }

                Kube::GetNamespacesRequest => tx_ns
                    .send(Event::Kube(Kube::GetNamespacesResponse(
                        get_namespace_list(),
                    )))
                    .unwrap(),

                Kube::LogStreamRequest(pod_name) => {
                    if let Some(handler) = log_stream_handler {
                        if let Ok(h) = handler.await {
                            h.0.abort();
                            h.1.abort();
                        }
                    }

                    let ns = namespace.read().unwrap().clone();
                    let tx = tx.clone();
                    let client = client.clone();

                    log_stream_handler = Some(tokio::spawn(log_stream(tx, client, ns, pod_name)));

                    task::yield_now().await;
                }

                Kube::ConfigRequest(config) => {
                    let client_clone = client.clone();
                    let namespace = namespace.read().unwrap().clone();

                    let split: Vec<&str> = config.split(' ').collect();

                    let ty = split[0];
                    let name = split[2];

                    let ret: Vec<String> = match ty {
                        "C" => {
                            let cms: Api<ConfigMap> = Api::namespaced(client_clone, &namespace);
                            let cm = cms.get(name).await.unwrap();
                            match cm.data {
                                Some(data) => {
                                    data.iter().map(|(k, v)| format!("{}: {}", k, v)).collect()
                                }
                                None => vec!["".to_string()],
                            }
                        }
                        "S" => {
                            let secs: Api<Secret> = Api::namespaced(client_clone, &namespace);
                            let sec = secs.get(name).await.unwrap();
                            match sec.data {
                                Some(data) => data
                                    .iter()
                                    .map(|(k, v)| {
                                        let decode = if let Ok(b) = std::str::from_utf8(&v.0) {
                                            b
                                        } else {
                                            unsafe { std::str::from_utf8_unchecked(&v.0) }
                                        };

                                        format!("{}: {}", k, decode)
                                    })
                                    .collect(),
                                None => vec!["".to_string()],
                            }
                        }
                        _ => {
                            unreachable!()
                        }
                    };

                    tx_config
                        .send(Event::Kube(Kube::ConfigResponse(ret)))
                        .unwrap();
                }

                Kube::CurrentContextRequest => {
                    let ns = namespace.read().unwrap().clone();
                    tx_ctx
                        .send(Event::Kube(Kube::CurrentContextResponse(
                            current_context.to_string(),
                            ns,
                        )))
                        .unwrap();
                }
                Kube::GetNamespacesResponse(_) => {}
                Kube::Pod(_) => {}
                Kube::LogStreamResponse(_) => {}
                Kube::Configs(_) => {}
                Kube::ConfigResponse(_) => {}
                Kube::CurrentContextResponse(_, _) => {}
            },
            _ => {}
        }
    }
}

async fn log_stream(
    tx: Sender<Event>,
    client: Client,
    ns: String,
    pod_name: String,
) -> LogStreamHandler {
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

    LogStreamHandler(stream_handler, event_handler)
}

async fn configs_loop(tx: Sender<Event>, namespace: Arc<RwLock<String>>) {
    let client = Client::try_default().await.unwrap();

    let mut interval = tokio::time::interval(time::Duration::from_secs(1));

    loop {
        interval.tick().await;
        let namespace = namespace.read().unwrap().clone();
        let configs = get_configs(client.clone(), &namespace).await;
        tx.send(Event::Kube(Kube::Configs(configs))).unwrap();
    }
}

async fn get_configs(client: Client, namespace: &str) -> Vec<String> {
    let configmaps: Api<ConfigMap> = Api::namespaced(client.clone(), namespace);

    let lp = ListParams::default();

    let configmap_list = configmaps.list(&lp).await.unwrap();

    let mut ret = Vec::new();

    for cm in configmap_list {
        let meta = Meta::meta(&cm);
        let name = meta.name.clone().unwrap();

        ret.push(format!("C │ {}", name));
    }

    let secrets: Api<Secret> = Api::namespaced(client, namespace);

    let lp = ListParams::default();

    let secret_list = secrets.list(&lp).await.unwrap();

    for secret in secret_list {
        let meta = Meta::meta(&secret);
        let name = meta.name.clone().unwrap();

        ret.push(format!("S │ {}", name));
    }

    ret
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

    let current_datetime: DateTime<Utc> = Utc::now();

    let mut max_name_len = 0;
    let mut max_phase_len = 0;

    let mut pod_infos = Vec::new();
    for p in &pods_list {
        let meta = Meta::meta(p);
        let name = meta.name.clone().unwrap();

        let phase = get_status(p.clone());

        let creation_timestamp: DateTime<Utc> = match &meta.creation_timestamp {
            Some(ref time) => time.0,
            None => current_datetime,
        };
        let duration: Duration = current_datetime - creation_timestamp;

        if max_name_len < name.len() {
            max_name_len = name.len();
        }

        if max_phase_len < phase.len() {
            max_phase_len = phase.len();
        }
        pod_infos.push(PodInfo::new(name, phase, age(duration)));
    }

    pod_infos
        .iter()
        .map(|p| p.to_string(max_name_len, max_phase_len))
        .collect()
}

// 参考：https://github.com/astefanutti/kubebox/blob/4ae0a2929a17c132a1ea61144e17b51f93eb602f/lib/kubernetes.js#L7
fn get_status(pod: Pod) -> String {
    let status: PodStatus;
    let meta: &ObjectMeta;

    match &pod.status {
        Some(s) => {
            status = s.clone();
            meta = Meta::meta(&pod);
        }
        None => return "".to_string(),
    }

    if meta.deletion_timestamp.is_some() {
        return "Terminating".to_string();
    }

    if let Some(reason) = &status.reason {
        if reason == "Evicted" {
            return "Evicted".to_string();
        }
    }

    let mut phase = status.phase.clone().or(status.reason.clone()).unwrap();

    let mut initializing = false;

    if let Some(cs) = &status.init_container_statuses {
        let find_terminated = cs.iter().enumerate().find(|(_, c)| {
            let state = c.state.clone().unwrap();
            let terminated = state.terminated;

            !is_terminated_container(&terminated)
        });

        if let Some((i, c)) = find_terminated {
            let state = c.state.clone().unwrap();
            let (terminated, waiting) = (state.terminated, state.waiting);

            initializing = true;

            phase = match terminated {
                Some(terminated) => match terminated.reason {
                    Some(reason) => format!("Init:{}", reason),
                    None => {
                        if let Some(s) = &terminated.signal {
                            format!("Init:Signal:{}", s)
                        } else {
                            format!("Init:ExitCode:{}", terminated.exit_code)
                        }
                    }
                },
                None => {
                    if let Some(waiting) = waiting {
                        if let Some(reason) = &waiting.reason {
                            if reason != "PodInitializing" {
                                return format!("Init:{}", reason);
                            }
                        }
                    }
                    format!("Init:{}/{}", i, cs.len())
                }
            };
        }
    }

    if !initializing {
        let mut has_running = false;

        if let Some(cs) = &status.container_statuses {
            cs.iter().for_each(|c| {
                let state = c.state.clone().unwrap();

                let (running, terminated, waiting) =
                    (state.running, state.terminated, state.waiting);

                let mut signal = None;
                let mut exit_code = 0;

                if let Some(terminated) = &terminated {
                    signal = terminated.signal;
                    exit_code = terminated.exit_code;
                }

                match &terminated {
                    Some(terminated) => {
                        if let Some(reason) = &terminated.reason {
                            phase = reason.clone();
                        };
                    }
                    None => match &waiting {
                        Some(waiting) => {
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
                        }
                        None => {
                            if running.is_some() && c.ready {
                                has_running = true;
                            }
                        }
                    },
                }
            })
        }

        if phase == "Completed" && has_running {
            phase = "Running".to_string();
        }
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
