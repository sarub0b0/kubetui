use std::sync::{atomic::AtomicBool, Arc};

use anyhow::{anyhow, Result};
use crossbeam::channel::Sender;
use serde_yaml::Value;

use crate::{
    features::{
        api_resources::kube::{ApiResource, ApiResources, SharedApiResources},
        yaml::message::YamlResponse,
    },
    logger,
    message::Message,
    workers::kube::{client::KubeClientRequest, worker::AbortWorker},
};

use super::SelectedYaml;

#[derive(Debug, Clone)]
pub struct SelectedYamlWorker<C>
where
    C: KubeClientRequest,
{
    is_terminated: Arc<AtomicBool>,
    tx: Sender<Message>,
    client: C,
    req: SelectedYaml,
    shared_api_resources: SharedApiResources,
}

impl<C: KubeClientRequest> SelectedYamlWorker<C> {
    pub fn new(
        is_terminated: Arc<AtomicBool>,
        tx: Sender<Message>,
        client: C,
        shared_api_resources: SharedApiResources,
        req: SelectedYaml,
    ) -> Self {
        Self {
            is_terminated,
            tx,
            client,
            req,
            shared_api_resources,
        }
    }
}

#[async_trait::async_trait]
impl<C: KubeClientRequest> AbortWorker for SelectedYamlWorker<C> {
    async fn run(&self) {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(3));

        let SelectedYaml {
            kind,
            name,
            namespace,
        } = &self.req;

        while !self
            .is_terminated
            .load(std::sync::atomic::Ordering::Relaxed)
        {
            interval.tick().await;

            let api_resources = self.shared_api_resources.read().await;

            let fetched_data = fetch_resource_yaml(
                &self.client,
                &api_resources,
                kind,
                name.to_string(),
                namespace.to_string(),
            )
            .await;

            self.tx
                .send(YamlResponse::SelectedYaml(fetched_data).into())
                .expect("Failed to send YamlResponse::Yaml");
        }
    }
}

/// 選択されているリソースのyamlを取得する
pub async fn fetch_resource_yaml<C: KubeClientRequest>(
    client: &C,
    api_resources: &ApiResources,
    kind: &ApiResource,
    name: String,
    ns: String,
) -> Result<Vec<String>> {
    logger!(
        info,
        "Fetching resource target [kind={} ns={} name={}]",
        kind,
        ns,
        name
    );

    let api = api_resources
        .get(kind)
        .ok_or_else(|| anyhow!("Can't get {} from API resource", kind))?;
    // json string data
    let kind = api.name();
    let path = if api.is_namespaced() {
        format!(
            "{}/namespaces/{}/{}/{}",
            api.group_version_url(),
            ns,
            kind,
            name
        )
    } else {
        format!("{}/{}/{}", api.group_version_url(), kind, name)
    };

    logger!(info, "Fetching resource [{}]", path);

    let res = client.request_text(&path).await?;

    logger!(info, "Fetched resource - {}", res);

    // yaml dataに変換
    let mut yaml_data: serde_yaml::Value = serde_json::from_str(&res)?;

    if let Some(Value::Mapping(md)) = yaml_data.get_mut("metadata") {
        md.remove("managedFields");
    }

    let yaml_string = serde_yaml::to_string(&yaml_data)?
        .lines()
        .map(ToString::to_string)
        .collect();

    Ok(yaml_string)
}
