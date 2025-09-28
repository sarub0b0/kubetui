use std::{collections::BTreeMap, time};

use crate::{
    features::config::message::ConfigResponse,
    kube::{
        apis::v1_table::TableRow,
        table::{get_resource_per_namespace, insert_ns, KubeTable, KubeTableRow},
        KubeClient,
    },
    message::Message,
    workers::kube::{SharedTargetNamespaces, Worker, WorkerResult},
};

use anyhow::Result;
use async_trait::async_trait;
use crossbeam::channel::Sender;
use futures::future::try_join_all;
use k8s_openapi::api::core::v1::{ConfigMap, Secret};
use kube::Resource;

#[derive(Clone)]
pub struct ConfigPoller {
    tx: Sender<Message>,
    shared_target_namespaces: SharedTargetNamespaces,
    kube_client: KubeClient,
}

impl ConfigPoller {
    pub fn new(
        tx: Sender<Message>,
        shared_target_namespaces: SharedTargetNamespaces,
        kube_client: KubeClient,
    ) -> Self {
        Self {
            tx,
            shared_target_namespaces,
            kube_client,
        }
    }
}

#[async_trait]
impl Worker for ConfigPoller {
    type Output = WorkerResult;

    async fn run(&self) -> Self::Output {
        let mut interval = tokio::time::interval(time::Duration::from_secs(1));

        let Self {
            tx,
            shared_target_namespaces,
            kube_client,
        } = self;

        loop {
            interval.tick().await;

            let target_namespaces = shared_target_namespaces.read().await;

            let table = fetch_configs(kube_client, &target_namespaces).await;

            tx.send(ConfigResponse::Table(table).into())
                .expect("Failed to send ConfigResponse::Table");
        }
    }
}

#[derive(Clone, Copy)]
enum Configs {
    ConfigMap,
    Secret,
}

impl Configs {
    fn resource(&self) -> &'static str {
        match self {
            Self::ConfigMap => "ConfigMap",
            Self::Secret => "Secret",
        }
    }

    fn url_path(&self, namespace: &str) -> String {
        match self {
            Self::ConfigMap => ConfigMap::url_path(&Default::default(), Some(namespace)),
            Self::Secret => Secret::url_path(&Default::default(), Some(namespace)),
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
            ty.url_path(ns),
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
