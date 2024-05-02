mod v1;
mod v1beta1;

use anyhow::{Context as _, Result};
use kube::{Api, Client};

use crate::{
    features::{
        api_resources::kube::SharedApiResources, network::message::NetworkRequestTargetParams,
    },
    kube::{
        apis::networking::gateway::{self},
        KubeClientRequest,
    },
};

use super::{Fetch, FetchedData};

pub(super) struct GatewayDescriptionWorker<'a, C>
where
    C: KubeClientRequest,
{
    client: &'a C,
    namespace: String,
    name: String,
    version: String,
}

#[async_trait::async_trait]
impl<'a, C> Fetch<'a, C> for GatewayDescriptionWorker<'a, C>
where
    C: KubeClientRequest,
{
    fn new(client: &'a C, params: NetworkRequestTargetParams, _: SharedApiResources) -> Self {
        let NetworkRequestTargetParams {
            namespace,
            name,
            version,
        } = params;

        Self {
            client,
            namespace,
            name,
            version,
        }
    }

    async fn fetch(&self) -> Result<FetchedData> {
        match self.version.as_str() {
            "v1" => fetch_v1(self.client.client().clone(), &self.name, &self.namespace).await,

            "v1beta1" => {
                fetch_v1beta1(self.client.client().clone(), &self.name, &self.namespace).await
            }

            _ => {
                unreachable!()
            }
        }
    }
}

async fn fetch_v1(client: Client, name: &str, namespace: &str) -> Result<FetchedData> {
    let api = Api::<gateway::v1::Gateway>::namespaced(client.clone(), namespace);

    let gateway = api.get(name).await.context(format!(
        "Failed to fetch Gateway: namespace={}, name={}",
        namespace, name
    ))?;

    let description = v1::Description::new(gateway.clone());

    let related_resources =
        v1::discover_releated_resources(client, name, namespace, &gateway).await?;

    let mut yaml = serde_yaml::to_string(&description)?
        .lines()
        .map(ToString::to_string)
        .collect::<Vec<String>>();

    let mut related_resources_yaml = serde_yaml::to_string(&related_resources)?
        .lines()
        .map(ToString::to_string)
        .collect::<Vec<String>>();

    if !related_resources_yaml.is_empty() {
        yaml.push("".into());

        yaml.append(&mut related_resources_yaml);
    }

    Ok(yaml)
}

async fn fetch_v1beta1(client: Client, name: &str, namespace: &str) -> Result<FetchedData> {
    let api = Api::<gateway::v1beta1::Gateway>::namespaced(client.clone(), namespace);

    let gateway = api.get(name).await.context(format!(
        "Failed to fetch Gateway: namespace={}, name={}",
        namespace, name
    ))?;

    let description = v1beta1::Description::new(gateway.clone());

    let related_resources =
        v1beta1::discover_releated_resources(client, name, namespace, &gateway).await?;

    let mut yaml = serde_yaml::to_string(&description)?
        .lines()
        .map(ToString::to_string)
        .collect::<Vec<String>>();

    let mut related_resources_yaml = serde_yaml::to_string(&related_resources)?
        .lines()
        .map(ToString::to_string)
        .collect::<Vec<String>>();

    if !related_resources_yaml.is_empty() {
        yaml.push("".into());

        yaml.append(&mut related_resources_yaml);
    }

    Ok(yaml)
}
