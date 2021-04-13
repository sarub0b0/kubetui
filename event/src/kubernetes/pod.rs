use super::{Event, Kube};

use crate::util::*;

use chrono::{DateTime, Duration, Utc};

use std::sync::Arc;
use std::time;

use tokio::sync::RwLock;

use crossbeam::channel::Sender;

use k8s_openapi::api::core::v1::{ContainerStateTerminated, Pod, PodStatus};

use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;

use kube::{
    api::{ListParams, Meta},
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

pub async fn pod_loop(tx: Sender<Event>, client: Client, namespace: Arc<RwLock<String>>) {
    let mut interval = tokio::time::interval(time::Duration::from_secs(1));

    loop {
        interval.tick().await;
        let namespace = namespace.read().await;
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
pub fn get_status(pod: Pod) -> String {
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
