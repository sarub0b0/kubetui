use super::{v1_table::*, Event, Kube, KubeArgs, KubeTable, Namespaces};

use std::{sync::Arc, time};

use crossbeam::channel::Sender;

use futures::future::join_all;
use k8s_openapi::api::core::v1::{ConfigMap, Secret};

use kube::{Api, Client};

pub async fn configs_loop(tx: Sender<Event>, namespaces: Namespaces, args: Arc<KubeArgs>) {
    let mut interval = tokio::time::interval(time::Duration::from_secs(1));

    loop {
        interval.tick().await;

        let namespaces = namespaces.read().await;

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

        let insert_ns = insert_ns(&namespaces);

        let jobs_configmap = join_all(namespaces.iter().map(|ns| {
            get_resourse_per_namespace(
                &args.client,
                &args.server_url,
                format!("api/v1/namespaces/{}/{}", ns, "configmaps"),
                &["Name", "Data", "Age"],
                move |row: &TableRow, indexes: &[usize]| {
                    let mut cells = vec![
                        "ConfigMap".to_string(),
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
        }));

        let jobs_secret = join_all(namespaces.iter().map(|ns| {
            get_resourse_per_namespace(
                &args.client,
                &args.server_url,
                format!("api/v1/namespaces/{}/{}", ns, "secrets"),
                &["Name", "Data", "Age"],
                move |row: &TableRow, indexes: &[usize]| {
                    let mut cells = vec![
                        "Secret".to_string(),
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
        }));

        let mut data: Vec<Vec<String>> = jobs_configmap.await.into_iter().flatten().collect();

        data.append(&mut jobs_secret.await.into_iter().flatten().collect());

        table.update_rows(data);

        tx.send(Event::Kube(Kube::Configs(table))).unwrap();
    }
}

pub async fn get_config(client: Client, ns: &str, kind: &str, name: &str) -> Vec<String> {
    match kind {
        "ConfigMap" => {
            let cms: Api<ConfigMap> = Api::namespaced(client, &ns);
            let cm = cms.get(name).await.unwrap();
            match cm.data {
                Some(data) => data.iter().map(|(k, v)| format!("{}: {}", k, v)).collect(),
                None => vec!["".to_string()],
            }
        }
        "Secret" => {
            let secs: Api<Secret> = Api::namespaced(client, &ns);
            let sec = secs.get(name).await.unwrap();
            match sec.data {
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
            }
        }
        _ => {
            panic!("Invalid kind [{}]. Set kind ConfigMap or Secret", kind);
        }
    }
}
