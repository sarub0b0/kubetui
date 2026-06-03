use std::{collections::BTreeMap, time};

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
use kube::Resource as _;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

use crate::{
    features::{
        api_resources::kube::{ApiResource, ApiResources, SharedApiResources},
        network::{
            message::{GatewayVersion, HTTPRouteVersion, NetworkResponse},
            NetworkColumn,
            NetworkColumnSpec,
            NetworkColumns,
        },
    },
    kube::{
        apis::{
            networking::gateway::{v1, v1beta1},
            v1_table::Table,
        },
        table::{insert_ns, KubeTable, KubeTableRow},
        KubeClient,
        KubeClientRequest,
    },
    logger,
    message::Message,
    workers::kube::{
        InfiniteWorker,
        SharedNetworkColumns,
        SharedNetworkFilter,
        SharedTargetNamespaces,
    },
};

#[derive(Debug, Default, Clone)]
struct NetworkTableRow {
    namespace: String,
    kind: String,
    version: String,
    name: String,
    cells: Vec<String>,
}

impl NetworkTableRow {
    fn to_kube_table_row(&self, is_insert_ns: bool) -> KubeTableRow {
        let mut row = self.cells.clone();
        if is_insert_ns {
            row.insert(0, self.namespace.clone());
        }
        KubeTableRow {
            namespace: self.namespace.clone(),
            name: self.name.clone(),
            metadata: Some(BTreeMap::from([
                ("kind".to_string(), self.kind.clone()),
                ("version".to_string(), self.version.clone()),
            ])),
            row,
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
            Self::Gateway(version) => {
                match version {
                    GatewayVersion::V1 => v1::Gateway::KIND,
                    GatewayVersion::V1Beta1 => v1beta1::Gateway::KIND,
                }
            }
            Self::HTTPRoute(version) => {
                match version {
                    HTTPRouteVersion::V1 => v1::HTTPRoute::KIND,
                    HTTPRouteVersion::V1Beta1 => v1beta1::HTTPRoute::KIND,
                }
            }
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

    async fn fetch_table(
        &self,
        client: &KubeClient,
        ns: &str,
        label_selector: Option<&str>,
    ) -> Result<Table> {
        let base_path = match self {
            Self::Ingress => Ingress::url_path(&Default::default(), Some(ns)),
            Self::Service => Service::url_path(&Default::default(), Some(ns)),
            Self::Pod => Pod::url_path(&Default::default(), Some(ns)),
            Self::NetworkPolicy => NetworkPolicy::url_path(&Default::default(), Some(ns)),
            Self::Gateway(GatewayVersion::V1) => {
                v1::Gateway::url_path(&Default::default(), Some(ns))
            }
            Self::Gateway(GatewayVersion::V1Beta1) => {
                v1beta1::Gateway::url_path(&Default::default(), Some(ns))
            }
            Self::HTTPRoute(HTTPRouteVersion::V1) => {
                v1::HTTPRoute::url_path(&Default::default(), Some(ns))
            }
            Self::HTTPRoute(HTTPRouteVersion::V1Beta1) => {
                v1beta1::HTTPRoute::url_path(&Default::default(), Some(ns))
            }
        };

        let path = match label_selector.filter(|s| !s.is_empty()) {
            Some(sel) => {
                format!(
                    "{}?labelSelector={}",
                    base_path,
                    utf8_percent_encode(sel, NON_ALPHANUMERIC)
                )
            }
            None => base_path,
        };

        client.request_table(&path).await.with_context(|| {
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
    tx: Sender<Message>,
    shared_target_namespaces: SharedTargetNamespaces,
    shared_network_columns: SharedNetworkColumns,
    shared_network_filter: SharedNetworkFilter,
    kube_client: KubeClient,
    api_resources: SharedApiResources,
}

impl NetworkPoller {
    pub fn new(
        tx: Sender<Message>,
        shared_target_namespaces: SharedTargetNamespaces,
        shared_network_columns: SharedNetworkColumns,
        shared_network_filter: SharedNetworkFilter,
        kube_client: KubeClient,
        api_resources: SharedApiResources,
    ) -> Self {
        Self {
            tx,
            shared_target_namespaces,
            shared_network_columns,
            shared_network_filter,
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
impl InfiniteWorker for NetworkPoller {
    async fn run(&self) {
        let mut interval = tokio::time::interval(time::Duration::from_secs(1));

        let tx = &self.tx;

        loop {
            interval.tick().await;

            let target_resources = {
                let apis = self.api_resources.read().await;
                target_resources(&apis)
            };

            let columns = self.shared_network_columns.read().await.clone();
            let label_selector = self.shared_network_filter.read().await.clone();

            let table = self
                .polling(&target_resources, &columns, label_selector.as_deref())
                .await;

            if let Err(e) = tx.send(NetworkResponse::List(table).into()) {
                logger!(error, "Failed to send NetworkResponse::List: {}", e);
                return;
            }
        }
    }
}

impl NetworkPoller {
    async fn polling(
        &self,
        target_resources: &[TargetResource],
        columns: &NetworkColumns,
        label_selector: Option<&str>,
    ) -> Result<KubeTable> {
        let target_namespaces = self.shared_target_namespaces.read().await;
        let specs = columns.specs();

        // Build target_columns dynamically from specs (skip KIND — supplied by
        // each resource's TargetResource::as_str() — and Label, which comes
        // from row.object.metadata.labels).
        let target_columns: Vec<&str> = specs
            .iter()
            .filter_map(|s| {
                match s {
                    NetworkColumnSpec::Builtin(NetworkColumn::Kind) => None,
                    NetworkColumnSpec::Builtin(c) => Some(c.as_str()),
                    NetworkColumnSpec::Label { .. } => None,
                }
            })
            .collect();

        let rows: Vec<_> = join_all(target_resources.iter().map(|kind| {
            self.fetch_resource(
                kind,
                &target_namespaces,
                specs,
                &target_columns,
                label_selector,
            )
        }))
        .await
        .into_iter()
        .inspect(|res| {
            if let Err(e) = res {
                logger!(error, "Failed to fetch resource: {:?}", e);
            }
        })
        .filter_map(|res| res.ok())
        .collect();

        let is_insert_ns = insert_ns(&target_namespaces);

        let mut header: Vec<String> = specs.iter().map(|s| s.header()).collect();
        if is_insert_ns {
            header.insert(0, "NAMESPACE".to_string());
        }

        let kube_rows: Vec<KubeTableRow> = rows
            .into_iter()
            .flatten()
            .map(|r| r.to_kube_table_row(is_insert_ns))
            .collect();

        Ok(KubeTable {
            header,
            rows: kube_rows,
        })
    }

    async fn fetch_resource(
        &self,
        kind: &TargetResource,
        namespaces: &[String],
        specs: &[NetworkColumnSpec],
        target_columns: &[&str],
        label_selector: Option<&str>,
    ) -> Result<Vec<NetworkTableRow>> {
        let client = &self.kube_client;

        let jobs = try_join_all(namespaces.iter().map(|ns| {
            fetch_resource_per_namespace(client, kind, ns, specs, target_columns, label_selector)
        }))
        .await?;

        Ok(jobs.into_iter().flatten().collect())
    }
}

/// Build the per-row cell vector from a spec list, the resource's kind name,
/// and a k8s API `TableRow`.
///
/// `builtin_indexes` are the positional indexes into `row.cells` for the
/// non-KIND builtin columns (NAME / AGE, in the order specified by the
/// fetch's `target_columns`).
pub(crate) fn build_network_row_cells(
    specs: &[NetworkColumnSpec],
    kind: &str,
    row: &crate::kube::apis::v1_table::TableRow,
    builtin_indexes: &[usize],
) -> Vec<String> {
    let mut builtin_iter = builtin_indexes.iter();
    specs
        .iter()
        .map(|s| {
            match s {
                NetworkColumnSpec::Builtin(NetworkColumn::Kind) => kind.to_string(),
                NetworkColumnSpec::Builtin(_) => {
                    let i = builtin_iter.next().expect("builtin index available");
                    row.cells[*i].to_string()
                }
                NetworkColumnSpec::Label { key, .. } => {
                    row.object
                        .as_ref()
                        .and_then(|o| o.0.get("metadata"))
                        .and_then(|m| m.get("labels"))
                        .and_then(|l| l.get(key))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string()
                }
            }
        })
        .collect()
}

async fn fetch_resource_per_namespace(
    client: &KubeClient,
    kind: &TargetResource,
    ns: &str,
    specs: &[NetworkColumnSpec],
    target_columns: &[&str],
    label_selector: Option<&str>,
) -> Result<Vec<NetworkTableRow>> {
    let table = kind.fetch_table(client, ns, label_selector).await?;

    let indexes = table.find_indexes(target_columns)?;
    let name_pos_in_specs = specs
        .iter()
        .position(|s| matches!(s, NetworkColumnSpec::Builtin(NetworkColumn::Name)))
        .expect("Name column must be present in network columns");

    let rows = table
        .rows
        .iter()
        .map(|row| {
            let cells = build_network_row_cells(specs, &kind.to_string(), row, &indexes);
            let name = cells[name_pos_in_specs].clone();
            NetworkTableRow {
                namespace: ns.to_string(),
                kind: kind.to_string(),
                version: kind.version().to_string(),
                name,
                cells,
            }
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

#[cfg(test)]
mod build_network_row_cells_tests {
    use super::*;
    use crate::kube::apis::v1_table::{TableRow, Value};
    use k8s_openapi::apimachinery::pkg::runtime::RawExtension;
    use pretty_assertions::assert_eq;
    use serde_json::Value as JsonValue;

    fn make_row(cells: &[&str]) -> TableRow {
        TableRow {
            cells: cells
                .iter()
                .map(|c| Value(JsonValue::String(c.to_string())))
                .collect(),
            ..Default::default()
        }
    }

    fn make_row_with_labels(cells: &[&str], labels: &[(&str, &str)]) -> TableRow {
        let labels_json: serde_json::Map<String, JsonValue> = labels
            .iter()
            .map(|(k, v)| (k.to_string(), JsonValue::String(v.to_string())))
            .collect();
        let object = serde_json::json!({ "metadata": { "labels": labels_json } });
        let mut row = make_row(cells);
        row.object = Some(RawExtension(object));
        row
    }

    #[test]
    fn builtin_only_cells_in_spec_order_with_kind_from_argument() {
        let specs = vec![
            NetworkColumnSpec::Builtin(NetworkColumn::Kind),
            NetworkColumnSpec::Builtin(NetworkColumn::Name),
            NetworkColumnSpec::Builtin(NetworkColumn::Age),
        ];
        let row = make_row(&["my-svc", "3h"]);
        let cells = build_network_row_cells(&specs, "Service", &row, &[0, 1]);
        assert_eq!(cells, vec!["Service", "my-svc", "3h"]);
    }

    #[test]
    fn label_arm_returns_value_when_label_present() {
        let specs = vec![
            NetworkColumnSpec::Builtin(NetworkColumn::Name),
            NetworkColumnSpec::Label {
                key: "app".to_string(),
                header: "APP".to_string(),
            },
        ];
        let row = make_row_with_labels(&["my-svc"], &[("app", "nginx")]);
        let cells = build_network_row_cells(&specs, "Service", &row, &[0]);
        assert_eq!(cells, vec!["my-svc", "nginx"]);
    }

    #[test]
    fn label_arm_returns_empty_when_label_absent() {
        let specs = vec![
            NetworkColumnSpec::Builtin(NetworkColumn::Name),
            NetworkColumnSpec::Label {
                key: "app".to_string(),
                header: "APP".to_string(),
            },
        ];
        let row = make_row_with_labels(&["my-svc"], &[("other", "x")]);
        let cells = build_network_row_cells(&specs, "Service", &row, &[0]);
        assert_eq!(cells, vec!["my-svc", ""]);
    }

    #[test]
    fn label_arm_returns_empty_when_no_object() {
        let specs = vec![
            NetworkColumnSpec::Builtin(NetworkColumn::Name),
            NetworkColumnSpec::Label {
                key: "app".to_string(),
                header: "APP".to_string(),
            },
        ];
        let row = make_row(&["my-svc"]);
        let cells = build_network_row_cells(&specs, "Service", &row, &[0]);
        assert_eq!(cells, vec!["my-svc", ""]);
    }

    #[test]
    fn mixed_builtin_and_label_in_spec_order() {
        let specs = vec![
            NetworkColumnSpec::Builtin(NetworkColumn::Kind),
            NetworkColumnSpec::Label {
                key: "env".to_string(),
                header: "ENV".to_string(),
            },
            NetworkColumnSpec::Builtin(NetworkColumn::Name),
            NetworkColumnSpec::Builtin(NetworkColumn::Age),
        ];
        // builtin order from target_columns derived from spec: Name (0), Age (1).
        let row = make_row_with_labels(&["my-svc", "3h"], &[("env", "prod")]);
        let cells = build_network_row_cells(&specs, "Ingress", &row, &[0, 1]);
        assert_eq!(cells, vec!["Ingress", "prod", "my-svc", "3h"]);
    }
}
