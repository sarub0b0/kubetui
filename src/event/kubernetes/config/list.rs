use std::{collections::BTreeMap, time};

use crate::{
    error::Result,
    event::kubernetes::{
        client::KubeClient,
        v1_table::{get_resource_per_namespace, insert_ns, TableRow},
        worker::{PollWorker, Worker},
        KubeTable, KubeTableRow, WorkerResult,
    },
};
use async_trait::async_trait;
use futures::future::try_join_all;

use super::ConfigResponse;

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
    type Output = WorkerResult;

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

            let table = fetch_configs(kube_client, &target_namespaces).await;

            tx.send(ConfigResponse::Table(table).into())
                .expect("Failed to send ConfigResponse::Table");
        }

        WorkerResult::Terminated
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
) -> Result<Vec<KubeTableRow>> {
    let insert_ns = insert_ns(namespaces);
    let jobs = try_join_all(namespaces.iter().map(|ns| {
        get_resource_per_namespace(
            client,
            format!("api/v1/namespaces/{}/{}", ns, ty.kind()),
            &["Name", r#"Data"#, "Age"],
            move |row: &TableRow, indexes: &[usize]| {
                let mut row = vec![
                    ty.resource().to_string(),
                    row.cells[indexes[0]].to_string(),
                    row.cells[indexes[1]].to_string(),
                    row.cells[indexes[2]].to_string(),
                ];

                let kind = row[0].clone();
                let name = row[1].clone();

                if insert_ns {
                    row.insert(0, ns.to_string())
                }

                KubeTableRow {
                    namespace: ns.to_string(),
                    name,
                    row,
                    metadata: Some(BTreeMap::from([("kind".to_string(), kind)])),
                }
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
