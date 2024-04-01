use std::{collections::BTreeMap, time};

use anyhow::Result;
use async_trait::async_trait;
use futures::future::try_join_all;
use k8s_openapi::{
    api::{
        core::v1::{Pod, Service},
        networking::v1::{Ingress, NetworkPolicy},
    },
    Resource as _,
};
use strum::{EnumIter, IntoEnumIterator};

use crate::{
    features::network::message::NetworkResponse,
    kube::{
        apis::{
            networking::gateway::v1::{Gateway, HTTPRoute},
            v1_table::Table,
        },
        table::{insert_ns, KubeTable, KubeTableRow},
        KubeClient, KubeClientRequest,
    },
    workers::kube::{PollerBase, Worker, WorkerResult},
};

#[derive(Debug, Default, Clone)]
pub struct NetworkTableRow {
    namespace: String,
    kind: String,
    name: String,
    age: String,
}

impl NetworkTableRow {
    fn to_kube_table_row(&self, is_insert_ns: bool) -> KubeTableRow {
        let row = if is_insert_ns {
            [&self.namespace, &self.kind, &self.name, &self.age]
                .iter()
                .map(ToString::to_string)
                .collect()
        } else {
            [&self.kind, &self.name, &self.age]
                .iter()
                .map(ToString::to_string)
                .collect()
        };

        KubeTableRow {
            namespace: self.namespace.to_string(),
            name: self.name.to_string(),
            metadata: Some(BTreeMap::from([(
                "kind".to_string(),
                self.kind.to_string(),
            )])),
            row,
        }
    }
}

#[derive(Debug, Default)]
pub struct NetworkTable {
    is_include_namespace: bool,
    rows: Vec<NetworkTableRow>,
}

impl NetworkTable {
    fn new(is_include_namespace: bool, rows: Vec<NetworkTableRow>) -> Self {
        Self {
            is_include_namespace,
            rows,
        }
    }

    fn header(&self) -> Vec<String> {
        if self.is_include_namespace {
            ["NAMESPACE", "KIND", "NAME", "AGE"]
                .iter()
                .map(ToString::to_string)
                .collect()
        } else {
            ["KIND", "NAME", "AGE"]
                .iter()
                .map(ToString::to_string)
                .collect()
        }
    }

    fn to_kube_table_rows(&self) -> Vec<KubeTableRow> {
        self.rows
            .iter()
            .map(|row| row.to_kube_table_row(self.is_include_namespace))
            .collect()
    }

    fn to_kube_table(&self) -> KubeTable {
        KubeTable {
            header: self.header(),
            rows: self.to_kube_table_rows(),
        }
    }
}

#[derive(EnumIter)]
enum TargetResource {
    Ingress,
    Service,
    Pod,
    NetworkPolicy,
    Gateway,
    HTTPRoute,
}

impl TargetResource {
    fn to_str(&self) -> &'static str {
        match self {
            Self::Ingress => Ingress::KIND,
            Self::Service => Service::KIND,
            Self::Pod => Pod::KIND,
            Self::NetworkPolicy => NetworkPolicy::KIND,
            Self::Gateway => Gateway::KIND,
            Self::HTTPRoute => HTTPRoute::KIND,
        }
    }

    async fn fetch_table(&self, client: &KubeClient, ns: &str) -> Result<Table> {
        match self {
            Self::Ingress => client.table_namespaced::<Ingress>(ns).await,
            Self::Service => client.table_namespaced::<Service>(ns).await,
            Self::Pod => client.table_namespaced::<Pod>(ns).await,
            Self::NetworkPolicy => client.table_namespaced::<NetworkPolicy>(ns).await,
            Self::Gateway => client.table_namespaced::<Gateway>(ns).await,
            Self::HTTPRoute => client.table_namespaced::<HTTPRoute>(ns).await,
        }
    }
}

impl std::fmt::Display for TargetResource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_str())
    }
}

#[derive(Clone)]
pub struct NetworkPoller {
    base: PollerBase,
}

impl NetworkPoller {
    pub fn new(base: PollerBase) -> Self {
        Self { base }
    }
}

#[async_trait()]
impl Worker for NetworkPoller {
    type Output = WorkerResult;

    async fn run(&self) -> Self::Output {
        let mut interval = tokio::time::interval(time::Duration::from_secs(1));

        let is_terminated = &self.base.is_terminated;
        let tx = &self.base.tx;

        while !is_terminated.load(std::sync::atomic::Ordering::Relaxed) {
            interval.tick().await;

            let table = self.polling().await;

            tx.send(NetworkResponse::List(table).into())
                .expect("Failed to send NetworkResponse::List");
        }

        WorkerResult::Terminated
    }
}

const TARGET_COLUMNS: [&str; 2] = ["Name", "Age"];

impl NetworkPoller {
    async fn polling(&self) -> Result<KubeTable> {
        let target_namespaces = self.base.shared_target_namespaces.read().await;

        let rows = try_join_all(
            TargetResource::iter().map(|kind| self.fetch_resource(kind, &target_namespaces)),
        )
        .await?;

        let table = NetworkTable::new(
            insert_ns(&target_namespaces),
            rows.into_iter().flatten().collect(),
        );

        Ok(table.to_kube_table())
    }

    async fn fetch_resource(
        &self,
        kind: TargetResource,
        namespaces: &[String],
    ) -> Result<Vec<NetworkTableRow>> {
        let client = &self.base.kube_client;

        let jobs = try_join_all(
            namespaces
                .iter()
                .map(|ns| fetch_resource_per_namespace(client, &kind, ns, &TARGET_COLUMNS)),
        )
        .await?;

        Ok(jobs.into_iter().flatten().collect())
    }
}

async fn fetch_resource_per_namespace(
    client: &KubeClient,
    kind: &TargetResource,
    ns: &str,
    target_columns: &[&str],
) -> Result<Vec<NetworkTableRow>> {
    let table = kind.fetch_table(client, ns).await?;

    let indexes = table.find_indexes(target_columns);

    let rows = table
        .rows
        .iter()
        .map(|row| NetworkTableRow {
            namespace: ns.to_string(),
            kind: kind.to_string(),
            name: row.cells[indexes[0]].to_string(),
            age: row.cells[indexes[1]].to_string(),
        })
        .collect();

    Ok(rows)
}
