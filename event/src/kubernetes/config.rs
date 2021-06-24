use super::{v1_table::*, Event, Kube, KubeArgs, KubeTable, Namespaces};

use std::{sync::Arc, time};

use crossbeam::channel::Sender;

use futures::future::try_join_all;
use k8s_openapi::api::core::v1::{ConfigMap, Secret};

use kube::{Api, Client};

use crate::error::{Error, Result};

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
    client: &Client,
    server_url: &str,
    namespaces: &[String],
    ty: Configs,
) -> Result<Vec<Vec<String>>> {
    let insert_ns = insert_ns(&namespaces);
    let jobs = try_join_all(namespaces.iter().map(|ns| {
        get_resourse_per_namespace(
            client,
            server_url,
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

async fn fetch_configs(namespaces: &[String], args: &KubeArgs) -> Result<KubeTable> {
    let KubeArgs {
        client, server_url, ..
    } = args;

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

    // TODO Github Actionsでrust 1.53.0が使えるようになったら配列に変更する
    let jobs = try_join_all(vec![
        fetch_configs_per_namespace(client, server_url, namespaces, Configs::ConfigMap),
        fetch_configs_per_namespace(client, server_url, namespaces, Configs::Secret),
    ])
    .await?;

    table.update_rows(jobs.into_iter().flatten().collect());

    Ok(table)
}

pub async fn configs_loop(tx: Sender<Event>, namespaces: Namespaces, args: Arc<KubeArgs>) {
    let mut interval = tokio::time::interval(time::Duration::from_secs(1));

    while !args
        .is_terminated
        .load(std::sync::atomic::Ordering::Relaxed)
    {
        interval.tick().await;

        let namespaces = namespaces.read().await;

        let table = fetch_configs(&namespaces, &args).await;

        tx.send(Event::Kube(Kube::Configs(table))).unwrap();
    }
}

pub async fn get_config(client: Client, ns: &str, kind: &str, name: &str) -> Result<Vec<String>> {
    match kind {
        "ConfigMap" => {
            let cms: Api<ConfigMap> = Api::namespaced(client, &ns);
            let cm = cms.get(name).await?;
            Ok(match cm.data {
                Some(data) => data.iter().map(|(k, v)| format!("{}: {}", k, v)).collect(),
                None => vec!["".to_string()],
            })
        }
        "Secret" => {
            let secs: Api<Secret> = Api::namespaced(client, &ns);
            let sec = secs.get(name).await?;
            Ok(match sec.data {
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
            })
        }
        _ => Err(Error::String(format!(
            "Invalid kind [{}]. Set kind ConfigMap or Secret",
            kind
        ))),
    }
}
