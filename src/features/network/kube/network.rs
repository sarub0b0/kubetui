use std::{
    collections::BTreeMap,
    sync::{atomic::AtomicBool, Arc},
    time,
};

use anyhow::{Context, Result};
use async_trait::async_trait;
use crossbeam::channel::Sender;
use futures::future::{join_all, try_join_all};
use k8s_openapi::{
    api::{
        core::v1::{Pod, Service},
        networking::v1::{Ingress, NetworkPolicy},
    },
    Resource,
};

use crate::{
    features::{
        api_resources::kube::{ApiResource, ApiResources, SharedApiResources},
        network::message::{GatewayVersion, HTTPRouteVersion, NetworkResponse},
    },
    kube::{
        apis::{
            networking::gateway::{v1, v1beta1},
            v1_table::Table,
        },
        table::{insert_ns, KubeTable, KubeTableRow},
        KubeClient, KubeClientRequest,
    },
    logger,
    message::Message,
    workers::kube::{SharedTargetNamespaces, Worker, WorkerResult},
};

#[derive(Debug, Default, Clone)]
pub struct NetworkTableRow {
    namespace: String,
    kind: String,
    version: String,
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
            metadata: Some(BTreeMap::from([
                ("kind".to_string(), self.kind.to_string()),
                ("version".to_string(), self.version.to_string()),
            ])),
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

#[derive(Copy, Clone)]
enum TargetResource {
    Ingress,
    Service,
    Pod,
    NetworkPolicy,
    Gateway(GatewayVersion),
    HTTPRoute(HTTPRouteVersion),
}

impl TargetResource {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Ingress => Ingress::KIND,
            Self::Service => Service::KIND,
            Self::Pod => Pod::KIND,
            Self::NetworkPolicy => NetworkPolicy::KIND,
            Self::Gateway(version) => match version {
                GatewayVersion::V1 => v1::Gateway::KIND,
                GatewayVersion::V1Beta1 => v1beta1::Gateway::KIND,
            },
            Self::HTTPRoute(version) => match version {
                HTTPRouteVersion::V1 => v1::HTTPRoute::KIND,
                HTTPRouteVersion::V1Beta1 => v1beta1::HTTPRoute::KIND,
            },
        }
    }

    fn version(&self) -> &'static str {
        match self {
            Self::Ingress => Ingress::VERSION,
            Self::Service => Service::VERSION,
            Self::Pod => Pod::VERSION,
            Self::NetworkPolicy => NetworkPolicy::VERSION,
            Self::Gateway(GatewayVersion::V1) => v1::Gateway::VERSION,
            Self::Gateway(GatewayVersion::V1Beta1) => v1beta1::Gateway::VERSION,
            Self::HTTPRoute(HTTPRouteVersion::V1) => v1::HTTPRoute::VERSION,
            Self::HTTPRoute(HTTPRouteVersion::V1Beta1) => v1beta1::HTTPRoute::VERSION,
        }
    }

    async fn fetch_table(&self, client: &KubeClient, ns: &str) -> Result<Table> {
        match self {
            Self::Ingress => client.table_namespaced::<Ingress>(ns).await,
            Self::Service => client.table_namespaced::<Service>(ns).await,
            Self::Pod => client.table_namespaced::<Pod>(ns).await,
            Self::NetworkPolicy => client.table_namespaced::<NetworkPolicy>(ns).await,
            Self::Gateway(GatewayVersion::V1) => client.table_namespaced::<v1::Gateway>(ns).await,
            Self::Gateway(GatewayVersion::V1Beta1) => {
                client.table_namespaced::<v1beta1::Gateway>(ns).await
            }
            Self::HTTPRoute(HTTPRouteVersion::V1) => {
                client.table_namespaced::<v1::HTTPRoute>(ns).await
            }
            Self::HTTPRoute(HTTPRouteVersion::V1Beta1) => {
                client.table_namespaced::<v1beta1::HTTPRoute>(ns).await
            }
        }
        .with_context(|| {
            format!(
                "Failed to fetch table: kind={} ({}) namespace={}",
                self.as_str(),
                self.version(),
                ns
            )
        })
    }
}

impl std::fmt::Display for TargetResource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Clone)]
pub struct NetworkPoller {
    is_terminated: Arc<AtomicBool>,
    tx: Sender<Message>,
    shared_target_namespaces: SharedTargetNamespaces,
    kube_client: KubeClient,
    api_resources: SharedApiResources,
}

impl NetworkPoller {
    pub fn new(
        is_terminated: Arc<AtomicBool>,
        tx: Sender<Message>,
        shared_target_namespaces: SharedTargetNamespaces,
        kube_client: KubeClient,
        api_resources: SharedApiResources,
    ) -> Self {
        Self {
            is_terminated,
            tx,
            shared_target_namespaces,
            kube_client,
            api_resources,
        }
    }
}

fn target_resources(api_resources: &ApiResources) -> Vec<TargetResource> {
    let mut targets = vec![
        TargetResource::Ingress,
        TargetResource::Service,
        TargetResource::Pod,
        TargetResource::NetworkPolicy,
    ];

    match find_api_resource(
        api_resources,
        v1::Gateway::GROUP,
        v1::Gateway::URL_PATH_SEGMENT,
    )
    .map(|api| api.version())
    {
        Some("v1") => {
            targets.push(TargetResource::Gateway(GatewayVersion::V1));
        }
        Some("v1beta1") => {
            targets.push(TargetResource::Gateway(GatewayVersion::V1Beta1));
        }
        Some(v) => {
            logger!(warn, "Gateway is not support: {}", v);
        }
        None => {
            logger!(warn, "Gateway is not found.");
        }
    }

    match find_api_resource(
        api_resources,
        v1::HTTPRoute::GROUP,
        v1::HTTPRoute::URL_PATH_SEGMENT,
    )
    .map(|api| api.version())
    {
        Some("v1") => {
            targets.push(TargetResource::HTTPRoute(HTTPRouteVersion::V1));
        }
        Some("v1beta1") => {
            targets.push(TargetResource::HTTPRoute(HTTPRouteVersion::V1Beta1));
        }
        Some(v) => {
            logger!(warn, "HTTPRoute is not support: {}", v);
        }
        None => {
            logger!(warn, "HTTPRoute is not found.");
        }
    }

    targets
}

#[async_trait()]
impl Worker for NetworkPoller {
    type Output = WorkerResult;

    async fn run(&self) -> Self::Output {
        let mut interval = tokio::time::interval(time::Duration::from_secs(1));

        let is_terminated = &self.is_terminated;
        let tx = &self.tx;

        while !is_terminated.load(std::sync::atomic::Ordering::Relaxed) {
            interval.tick().await;

            let target_resources = {
                let apis = self.api_resources.read().await;
                target_resources(&apis)
            };

            let table = self.polling(&target_resources).await;

            tx.send(NetworkResponse::List(table).into())
                .expect("Failed to send NetworkResponse::List");
        }

        WorkerResult::Terminated
    }
}

const TARGET_COLUMNS: [&str; 2] = ["Name", "Age"];

impl NetworkPoller {
    async fn polling(&self, target_resources: &[TargetResource]) -> Result<KubeTable> {
        let target_namespaces = self.shared_target_namespaces.read().await;

        let rows: Vec<_> = join_all(
            target_resources
                .iter()
                .map(|kind| self.fetch_resource(kind, &target_namespaces)),
        )
        .await
        .into_iter()
        .inspect(|res| {
            if let Err(e) = res {
                logger!(error, "Failed to fetch resource: {:?}", e);
            }
        })
        .filter_map(|res| res.ok())
        .collect();

        let table = NetworkTable::new(
            insert_ns(&target_namespaces),
            rows.into_iter().flatten().collect(),
        );

        Ok(table.to_kube_table())
    }

    async fn fetch_resource(
        &self,
        kind: &TargetResource,
        namespaces: &[String],
    ) -> Result<Vec<NetworkTableRow>> {
        let client = &self.kube_client;

        let jobs = try_join_all(
            namespaces
                .iter()
                .map(|ns| fetch_resource_per_namespace(client, kind, ns, &TARGET_COLUMNS)),
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
            version: kind.version().to_string(),
            name: row.cells[indexes[0]].to_string(),
            age: row.cells[indexes[1]].to_string(),
        })
        .collect();

    Ok(rows)
}

/// groupとkindが一致するAPIリソースを取得する
///   * 一致するリソースが複数ある場合は、preferredVersionを優先して取得する
///   * preferredVersionがない場合は、最初に見つかったリソースを取得する
///   * 一致するリソースがない場合はNoneを返す
fn find_api_resource<'a>(
    api_resources: &'a ApiResources,
    group: &str,
    kind: &str,
) -> Option<&'a ApiResource> {
    let mut apis_for_find = api_resources
        .iter()
        .filter(|api| api.group() == group && api.name() == kind);

    let mut apis_for_first = apis_for_find.clone();

    if let Some(api) = apis_for_find.find(|api| api.is_preferred_version()) {
        Some(api)
    } else {
        apis_for_first.next()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod find_api_resource {
        use super::*;

        use kube::discovery::Scope;
        use pretty_assertions::assert_eq;

        #[test]
        fn with_preferred_version() {
            let api_resources = ApiResources::from([
                ApiResource::Apis {
                    group: "group1".to_string(),
                    name: "kind1".to_string(),
                    version: "v1".to_string(),
                    preferred_version: false,
                    scope: Scope::Namespaced,
                },
                ApiResource::Apis {
                    group: "group1".to_string(),
                    name: "kind1".to_string(),
                    version: "v2".to_string(),
                    preferred_version: true,
                    scope: Scope::Namespaced,
                },
            ]);

            let actual = find_api_resource(&api_resources, "group1", "kind1");

            let expected = ApiResource::Apis {
                group: "group1".to_string(),
                name: "kind1".to_string(),
                version: "v2".to_string(),
                preferred_version: true,
                scope: Scope::Namespaced,
            };

            assert_eq!(actual, Some(&expected));
        }

        #[test]
        fn without_preferred_version() {
            let api_resources = ApiResources::from([
                ApiResource::Apis {
                    group: "group1".to_string(),
                    name: "kind1".to_string(),
                    version: "v2".to_string(),
                    preferred_version: false,
                    scope: Scope::Namespaced,
                },
                ApiResource::Apis {
                    group: "group1".to_string(),
                    name: "kind1".to_string(),
                    version: "v1".to_string(),
                    preferred_version: false,
                    scope: Scope::Namespaced,
                },
            ]);

            let actual = find_api_resource(&api_resources, "group1", "kind1");

            let expected = ApiResource::Apis {
                group: "group1".to_string(),
                name: "kind1".to_string(),
                version: "v2".to_string(),
                preferred_version: false,
                scope: Scope::Namespaced,
            };

            assert_eq!(actual, Some(&expected));
        }

        #[test]
        fn no_matching_resources() {
            let api_resources = ApiResources::default();

            let actual = find_api_resource(&api_resources, "group1", "kind1");

            assert_eq!(actual, None);
        }
    }
}
