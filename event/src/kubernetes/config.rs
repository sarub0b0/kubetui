use super::{Event, Kube};

use std::sync::{Arc, RwLock};
use std::time;

use crossbeam::channel::Sender;

use k8s_openapi::api::core::v1::{ConfigMap, Secret};

use kube::{
    api::{ListParams, Meta},
    Api, Client,
};

pub async fn configs_loop(tx: Sender<Event>, client: Client, namespace: Arc<RwLock<String>>) {
    let mut interval = tokio::time::interval(time::Duration::from_secs(1));

    loop {
        interval.tick().await;
        let client = client.clone();
        let namespace = namespace.read().unwrap().clone();

        let configmaps: Api<ConfigMap> = Api::namespaced(client.clone(), &namespace);

        let lp = ListParams::default();

        let configmap_list = configmaps.list(&lp).await.unwrap();

        let mut ret = Vec::new();

        for cm in configmap_list {
            let meta = Meta::meta(&cm);
            let name = meta.name.clone().unwrap();

            ret.push(format!("C │ {}", name));
        }

        let secrets: Api<Secret> = Api::namespaced(client, &namespace);

        let lp = ListParams::default();

        let secret_list = secrets.list(&lp).await.unwrap();

        for secret in secret_list {
            let meta = Meta::meta(&secret);
            let name = meta.name.clone().unwrap();

            ret.push(format!("S │ {}", name));
        }

        tx.send(Event::Kube(Kube::Configs(ret))).unwrap();
    }
}

pub async fn get_config(client: Client, ns: &str, config: &str) -> Vec<String> {
    let client_clone = client.clone();

    let split: Vec<&str> = config.split(' ').collect();

    let ty = split[0];
    let name = split[2];

    match ty {
        "C" => {
            let cms: Api<ConfigMap> = Api::namespaced(client_clone, &ns);
            let cm = cms.get(name).await.unwrap();
            match cm.data {
                Some(data) => data.iter().map(|(k, v)| format!("{}: {}", k, v)).collect(),
                None => vec!["".to_string()],
            }
        }
        "S" => {
            let secs: Api<Secret> = Api::namespaced(client_clone, &ns);
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
            unreachable!()
        }
    }
}
