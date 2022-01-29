use super::{
    v1_table::*,
    worker::{PollWorker, Worker},
    Event, Kube, KubeClient, KubeTable, WorkerResult,
};

use std::{collections::BTreeMap, time};

use futures::future::try_join_all;
use k8s_openapi::api::core::v1::ConfigMap;

use kube::{api::ObjectMeta, Api};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::{anyhow, Error, Result};

#[derive(Clone)]
pub struct ConfigsPollWorker {
    inner: PollWorker,
}

impl ConfigsPollWorker {
    pub fn new(inner: PollWorker) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl Worker for ConfigsPollWorker {
    type Output = Result<WorkerResult>;

    async fn run(&self) -> Self::Output {
        let mut interval = tokio::time::interval(time::Duration::from_secs(1));

        let Self {
            inner:
                PollWorker {
                    is_terminated,
                    tx,
                    namespaces,
                    kube_client,
                },
        } = self;

        while !is_terminated.load(std::sync::atomic::Ordering::Relaxed) {
            interval.tick().await;

            let namespaces = namespaces.read().await;

            let table = fetch_configs(kube_client, &namespaces).await;

            tx.send(Event::Kube(Kube::Configs(table)))?;
        }
        Ok(WorkerResult::Terminated)
    }
}

#[derive(Clone, Copy)]
enum Configs {
    ConfigMap,
    Secret,
}

impl Configs {
    fn kind(&self) -> &'static str {
        match self {
            Self::ConfigMap => "configmaps",
            Self::Secret => "secrets",
        }
    }

    fn resource(&self) -> &'static str {
        match self {
            Self::ConfigMap => "ConfigMap",
            Self::Secret => "Secret",
        }
    }
}

async fn fetch_configs_per_namespace(
    client: &KubeClient,
    namespaces: &[String],
    ty: Configs,
) -> Result<Vec<Vec<String>>> {
    let insert_ns = insert_ns(namespaces);
    let jobs = try_join_all(namespaces.iter().map(|ns| {
        get_resource_per_namespace(
            client,
            format!("api/v1/namespaces/{}/{}", ns, ty.kind()),
            &["Name", "Data", "Age"],
            move |row: &TableRow, indexes: &[usize]| {
                let mut cells = vec![
                    ty.resource().to_string(),
                    row.cells[indexes[0]].to_string(),
                    row.cells[indexes[1]].to_string(),
                    row.cells[indexes[2]].to_string(),
                ];

                if insert_ns {
                    cells.insert(0, ns.to_string())
                }

                cells
            },
        )
    }))
    .await?;

    Ok(jobs.into_iter().flatten().collect())
}

async fn fetch_configs(client: &KubeClient, namespaces: &[String]) -> Result<KubeTable> {
    let mut table = KubeTable {
        header: if namespaces.len() == 1 {
            ["KIND", "NAME", "DATA", "AGE"]
                .iter()
                .map(ToString::to_string)
                .collect()
        } else {
            ["NAMESPACE", "KIND", "NAME", "DATA", "AGE"]
                .iter()
                .map(ToString::to_string)
                .collect()
        },
        ..Default::default()
    };

    let jobs = try_join_all([
        fetch_configs_per_namespace(client, namespaces, Configs::ConfigMap),
        fetch_configs_per_namespace(client, namespaces, Configs::Secret),
    ])
    .await?;

    table.update_rows(jobs.into_iter().flatten().collect());

    Ok(table)
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct Secret {
    metadata: ObjectMeta,
    data: Option<BTreeMap<String, String>>,
}

impl kube::Resource for Secret {
    type DynamicType = ();

    fn kind(_: &Self::DynamicType) -> std::borrow::Cow<'_, str> {
        "Secret".into()
    }

    fn group(_: &Self::DynamicType) -> std::borrow::Cow<'_, str> {
        "".into()
    }

    fn version(_: &Self::DynamicType) -> std::borrow::Cow<'_, str> {
        "v1".into()
    }

    fn plural(_: &Self::DynamicType) -> std::borrow::Cow<'_, str> {
        "secrets".into()
    }

    fn meta(&self) -> &ObjectMeta {
        &self.metadata
    }

    fn meta_mut(&mut self) -> &mut ObjectMeta {
        &mut self.metadata
    }
}
pub async fn get_config(
    client: KubeClient,
    ns: &str,
    kind: &str,
    name: &str,
) -> Result<Vec<String>> {
    match kind {
        "ConfigMap" => {
            let cms: Api<ConfigMap> = Api::namespaced(client.client_clone(), ns);
            let cm = cms.get(name).await?;
            if let Some(data) = cm.data {
                Ok(data.iter().map(|(k, v)| format!("{}: {}", k, v)).collect())
            } else {
                Err(anyhow!(Error::NoneParameter("configmap.data")))
            }
        }
        "Secret" => {
            let secs: Api<Secret> = Api::namespaced(client.client_clone(), ns);
            let sec = secs.get(name).await?;

            if let Some(data) = sec.data {
                Ok(data
                    .iter()
                    .map(|(k, v)| {
                        let decode = match base64::decode(v) {
                            Ok(decoded_data) => {
                                if let Ok(utf8_data) = String::from_utf8(decoded_data) {
                                    utf8_data
                                } else {
                                    format!("Can't output a non-UTF8 value. [base64-encoded] {}", v)
                                }
                            }
                            Err(err) => err.to_string(),
                        };

                        format!("{}: {}", k, decode)
                    })
                    .collect())
            } else {
                Err(anyhow!(Error::NoneParameter("secret.data")))
            }
        }
        _ => Err(anyhow!(Error::Raw(format!(
            "Invalid kind [{}]. Set kind ConfigMap or Secret",
            kind
        )))),
    }
}
