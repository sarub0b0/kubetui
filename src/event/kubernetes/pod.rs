use super::{
    v1_table::*,
    worker::{PollWorker, Worker},
    KubeClient, KubeTable, KubeTableRow, WorkerResult, {Event, Kube},
};

use crate::error::Result;

use async_trait::async_trait;

use futures::future::try_join_all;

use std::time;

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

#[derive(Clone)]
pub struct PodPollWorker {
    inner: PollWorker,
}

impl PodPollWorker {
    pub fn new(inner: PollWorker) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl Worker for PodPollWorker {
    type Output = Result<WorkerResult>;

    async fn run(&self) -> Self::Output {
        let mut interval = tokio::time::interval(time::Duration::from_secs(1));

        let Self {
            inner:
                PollWorker {
                    is_terminated,
                    tx,
                    shared_target_namespaces,
                    kube_client,
                },
        } = self;

        while !is_terminated.load(std::sync::atomic::Ordering::Relaxed) {
            interval.tick().await;
            let target_namespaces = shared_target_namespaces.read().await;

            let pod_info = get_pod_info(kube_client, &target_namespaces).await;

            tx.send(Event::Kube(Kube::Pod(pod_info))).unwrap();
        }
        Ok(WorkerResult::Terminated)
    }
}

async fn get_pods_per_namespace(
    client: &KubeClient,
    namespaces: &[String],
) -> Result<Vec<Vec<KubeTableRow>>> {
    let insert_ns = insert_ns(namespaces);
    try_join_all(namespaces.iter().map(|ns| {
        get_resource_per_namespace(
            client,
            format!("api/v1/namespaces/{}/{}", ns, "pods"),
            &["Name", "Ready", "Status", "Age"],
            move |row: &TableRow, indexes: &[usize]| {
                let mut row: Vec<String> =
                    indexes.iter().map(|i| row.cells[*i].to_string()).collect();

                let name = row[0].clone();

                let color = match row[2].as_str() {
                    s if s == "Completed" || s.contains("Evicted") => Some(90),
                    s if s.contains("BackOff") || s.contains("Err") || s.contains("Unknown") => {
                        Some(31)
                    }
                    _ => None,
                };

                if insert_ns {
                    row.insert(0, ns.to_string())
                }

                if let Some(color) = color {
                    row.iter_mut()
                        .for_each(|r| *r = format!("\x1b[{}m{}\x1b[0m", color, r))
                }

                KubeTableRow {
                    namespace: ns.to_string(),
                    name,
                    row,
                    ..Default::default()
                }
            },
        )
    }))
    .await
}

async fn get_pod_info(client: &KubeClient, namespaces: &[String]) -> Result<KubeTable> {
    let jobs = get_pods_per_namespace(client, namespaces).await;

    let ok_only: Vec<KubeTableRow> = jobs?.into_iter().flatten().collect();

    let mut table = KubeTable {
        header: if namespaces.len() == 1 {
            ["NAME", "READY", "STATUS", "AGE"]
                .iter()
                .map(ToString::to_string)
                .collect()
        } else {
            ["NAMESPACE", "NAME", "READY", "STATUS", "AGE"]
                .iter()
                .map(ToString::to_string)
                .collect()
        },
        ..Default::default()
    };

    table.update_rows(ok_only);

    Ok(table)
}

// 参考：https://github.com/astefanutti/kubebox/blob/4ae0a2929a17c132a1ea61144e17b51f93eb602f/lib/kubernetes.js#L7
// #[allow(dead_code)]
// pub fn get_status(pod: Pod) -> String {
//     let status: PodStatus;
//     let meta: &ObjectMeta = pod.meta();

//     match &pod.status {
//         Some(s) => {
//             status = s.clone();
//         }
//         None => return "".to_string(),
//     }

//     if meta.deletion_timestamp.is_some() {
//         return "Terminating".to_string();
//     }

//     if let Some(reason) = &status.reason {
//         if reason == "Evicted" {
//             return "Evicted".to_string();
//         }
//     }

//     let mut phase = status
//         .phase
//         .clone()
//         .or_else(|| status.reason.clone())
//         .unwrap();

//     let mut initializing = false;

//     let cs = &status.init_container_statuses;

//     let find_terminated = cs.iter().enumerate().find(|(_, c)| {
//         let state = c.state.clone().unwrap();
//         let terminated = state.terminated;

//         !is_terminated_container(&terminated)
//     });

//     if let Some((i, c)) = find_terminated {
//         let state = c.state.clone().unwrap();
//         let (terminated, waiting) = (state.terminated, state.waiting);

//         initializing = true;

//         phase = match terminated {
//             Some(terminated) => match terminated.reason {
//                 Some(reason) => format!("Init:{}", reason),
//                 None => {
//                     if let Some(s) = &terminated.signal {
//                         format!("Init:Signal:{}", s)
//                     } else {
//                         format!("Init:ExitCode:{}", terminated.exit_code)
//                     }
//                 }
//             },
//             None => {
//                 if let Some(waiting) = waiting {
//                     if let Some(reason) = &waiting.reason {
//                         if reason != "PodInitializing" {
//                             return format!("Init:{}", reason);
//                         }
//                     }
//                 }
//                 format!("Init:{}/{}", i, cs.len())
//             }
//         };
//     }

//     if !initializing {
//         let mut has_running = false;

//         let cs = &status.container_statuses;
//         cs.iter().for_each(|c| {
//             let state = c.state.clone().unwrap();

//             let (running, terminated, waiting) = (state.running, state.terminated, state.waiting);

//             let mut signal = None;
//             let mut exit_code = 0;

//             if let Some(terminated) = &terminated {
//                 signal = terminated.signal;
//                 exit_code = terminated.exit_code;
//             }

//             match &terminated {
//                 Some(terminated) => {
//                     if let Some(reason) = &terminated.reason {
//                         phase = reason.clone();
//                     };
//                 }
//                 None => match &waiting {
//                     Some(waiting) => {
//                         phase = match &waiting.reason {
//                             Some(reason) => reason.clone(),
//                             None => {
//                                 if let Some(signal) = signal {
//                                     format!("Signal:{}", signal)
//                                 } else {
//                                     format!("ExitCode:{}", exit_code)
//                                 }
//                             }
//                         };
//                     }
//                     None => {
//                         if running.is_some() && c.ready {
//                             has_running = true;
//                         }
//                     }
//                 },
//             }
//         });

//         if phase == "Completed" && has_running {
//             phase = "Running".to_string();
//         }
//     }

//     phase
// }

// fn is_terminated_container(terminated: &Option<ContainerStateTerminated>) -> bool {
//     if let Some(terminated) = terminated {
//         if terminated.exit_code == 0 {
//             return true;
//         }
//     }
//     false
// }
