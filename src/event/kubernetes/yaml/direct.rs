use std::sync::{atomic::AtomicBool, Arc};

use anyhow::Result;
use crossbeam::channel::Sender;
use k8s_openapi::{
    api::{
        core::v1::{ConfigMap, Pod, Secret, Service},
        networking::v1::{Ingress, NetworkPolicy},
    },
    NamespaceResourceScope,
};
use kube::{Api, Resource};
use serde::{de::DeserializeOwned, Serialize};

use crate::{
    event::{
        kubernetes::{client::KubeClient, worker::AbortWorker, yaml::YamlResponse},
        Event,
    },
    logger,
};

#[derive(Debug, Clone)]
pub struct DirectedYaml {
    pub name: String,
    pub namespace: String,
    pub kind: DirectedYamlKind,
}

#[derive(Debug, Clone)]
pub enum DirectedYamlKind {
    Pod,
    ConfigMap,
    Secret,
    Ingress,
    Service,
    NetworkPolicy,
}

impl std::fmt::Display for DirectedYamlKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DirectedYamlKind::Pod => write!(f, "pods"),
            DirectedYamlKind::ConfigMap => write!(f, "configmaps"),
            DirectedYamlKind::Secret => write!(f, "secrets"),
            DirectedYamlKind::Ingress => write!(f, "ingresses"),
            DirectedYamlKind::Service => write!(f, "services"),
            DirectedYamlKind::NetworkPolicy => write!(f, "networkpolicies"),
        }
    }
}

#[derive(Clone)]
pub struct DirectedYamlWorker {
    is_terminated: Arc<AtomicBool>,
    tx: Sender<Event>,
    client: KubeClient,
    req: DirectedYaml,
}

impl DirectedYamlWorker {
    pub fn new(
        is_terminated: Arc<AtomicBool>,
        tx: Sender<Event>,
        client: KubeClient,
        req: DirectedYaml,
    ) -> Self {
        Self {
            is_terminated,
            tx,
            client,
            req,
        }
    }
}

#[async_trait::async_trait]
impl AbortWorker for DirectedYamlWorker {
    async fn run(&self) {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(3));

        let DirectedYaml {
            kind,
            name,
            namespace,
        } = &self.req;

        while !self
            .is_terminated
            .load(std::sync::atomic::Ordering::Relaxed)
        {
            interval.tick().await;

            let yaml = match kind {
                DirectedYamlKind::Pod => {
                    fetch_resource_yaml::<Pod>(&self.client, name, namespace).await
                }
                DirectedYamlKind::ConfigMap => {
                    fetch_resource_yaml::<ConfigMap>(&self.client, name, namespace).await
                }
                DirectedYamlKind::Secret => {
                    fetch_resource_yaml::<Secret>(&self.client, name, namespace).await
                }
                DirectedYamlKind::Ingress => {
                    fetch_resource_yaml::<Ingress>(&self.client, name, namespace).await
                }
                DirectedYamlKind::Service => {
                    fetch_resource_yaml::<Service>(&self.client, name, namespace).await
                }
                DirectedYamlKind::NetworkPolicy => {
                    fetch_resource_yaml::<NetworkPolicy>(&self.client, name, namespace).await
                }
            };

            self.tx
                .send(
                    YamlResponse::DirectedYaml {
                        yaml,
                        kind: kind.to_string(),
                        name: name.to_string(),
                    }
                    .into(),
                )
                .expect("Failed to send YamlResponse::Yaml");
        }
    }
}

/// 選択されているリソースのyamlを取得する
pub async fn fetch_resource_yaml<K>(
    client: &KubeClient,
    name: &str,
    ns: &str,
) -> Result<Vec<String>>
where
    K: Resource<Scope = NamespaceResourceScope> + k8s_openapi::Resource,
    <K as kube::Resource>::DynamicType: Default,
    K: DeserializeOwned + Clone + std::fmt::Debug,
    K: Serialize,
{
    logger!(
        info,
        "Fetching resource target [kind={} ns={} name={}]",
        K::KIND,
        ns,
        name
    );

    let api: Api<K> = Api::namespaced(client.to_client(), ns);

    let mut data = api.get(name).await?;

    let metadata = data.meta_mut();
    metadata.managed_fields = None;

    let yaml_string = serde_yaml::to_string(&data)?
        .lines()
        .map(ToString::to_string)
        .collect();

    Ok(yaml_string)
}
