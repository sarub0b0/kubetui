use super::request::get_resource_request;
use super::v1_table::*;
use super::{Event, Kube};

use std::sync::Arc;
use std::time;

use tokio::sync::RwLock;

use crossbeam::channel::Sender;

use k8s_openapi::api::core::v1::{ContainerStateTerminated, Pod, PodStatus};

use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;

use kube::{api::Resource, Client};

#[allow(dead_code)]
pub struct PodInfo {
    name: String,
    ready: String,
    status: String,
    age: String,
}

#[allow(dead_code)]
impl PodInfo {
    fn new(
        name: impl Into<String>,
        ready: impl Into<String>,
        status: impl Into<String>,
        age: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            ready: ready.into(),
            status: status.into(),
            age: age.into(),
        }
    }
}

pub async fn pod_loop(
    tx: Sender<Event>,
    client: Client,
    namespace: Arc<RwLock<String>>,
    server_url: String,
) {
    let mut interval = tokio::time::interval(time::Duration::from_secs(1));

    loop {
        interval.tick().await;
        let namespace = namespace.read().await;
        let pod_info = get_pod_info(client.clone(), &namespace, &server_url).await;
        tx.send(Event::Kube(Kube::Pod(pod_info))).unwrap();
    }
}

async fn get_pod_info(client: Client, namespace: &str, server_url: &str) -> Vec<Vec<String>> {
    let table: Result<Table, kube::Error> = client
        .request(get_resource_request(server_url, namespace, "pods").unwrap())
        .await;

    let mut ret = Vec::new();

    match table {
        Ok(t) => {
            let name_idx = t
                .column_definitions
                .iter()
                .position(|cd| cd.name == "Name")
                .unwrap_or_default();

            let ready_idx = t
                .column_definitions
                .iter()
                .position(|cd| cd.name == "Ready")
                .unwrap_or_default();

            let status_idx = t
                .column_definitions
                .iter()
                .position(|cd| cd.name == "Status")
                .unwrap_or_default();

            let age_idx = t
                .column_definitions
                .iter()
                .position(|cd| cd.name == "Age")
                .unwrap_or_default();

            for row in t.rows.iter() {
                let (name, ready, status, age) = (
                    row.cells[name_idx].as_str().unwrap(),
                    row.cells[ready_idx].as_str().unwrap(),
                    row.cells[status_idx].as_str().unwrap(),
                    row.cells[age_idx].as_str().unwrap(),
                );

                ret.push(vec![
                    name.to_string(),
                    ready.to_string(),
                    status.to_string(),
                    age.to_string(),
                ]);
            }
        }
        Err(e) => return vec![vec![e.to_string()]],
    }

    ret
}

// 参考：https://github.com/astefanutti/kubebox/blob/4ae0a2929a17c132a1ea61144e17b51f93eb602f/lib/kubernetes.js#L7
#[allow(dead_code)]
pub fn get_status(pod: Pod) -> String {
    let status: PodStatus;
    let meta: &ObjectMeta = pod.meta();

    match &pod.status {
        Some(s) => {
            status = s.clone();
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

    let mut phase = status
        .phase
        .clone()
        .or_else(|| status.reason.clone())
        .unwrap();

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

    phase
}

fn is_terminated_container(terminated: &Option<ContainerStateTerminated>) -> bool {
    if let Some(terminated) = terminated {
        if terminated.exit_code == 0 {
            return true;
        }
    }
    false
}
