use std::{collections::BTreeMap, fmt::Display};

use crossbeam::channel::Sender;
use serde::{Deserialize, Serialize};
use serde_yaml::Value;

use crate::{
    error::Result,
    event::{
        kubernetes::{client::KubeClient, network::NetworkMessage},
        Event,
    },
};

use super::DescriptionWorker;

#[derive(Deserialize, Serialize, Debug)]
struct Status {
    phase: String,
    #[serde(rename = "hostIP")]
    host_ip: String,
    #[serde(rename = "podIP")]
    pod_ip: String,
    #[serde(rename = "podIPs")]
    pod_ips: Vec<BTreeMap<String, String>>,
}

#[derive(Deserialize, Serialize, Debug)]
struct Metadata {
    labels: BTreeMap<String, String>,
}

#[derive(Deserialize, Serialize, Debug)]
struct Pod {
    metadata: Metadata,
    status: Status,
}

impl Pod {
    fn to_string_vec(&self) -> Vec<String> {
        let labels = self
            .metadata
            .labels
            .iter()
            .map(|(k, v)| format!("    {}: {}", k, v))
            .collect::<Vec<String>>();

        let pod_ips = self
            .status
            .pod_ips
            .iter()
            .flat_map(|v| {
                v.iter()
                    .map(|(_, v)| format!("      - {}", v))
                    .collect::<Vec<String>>()
            })
            .collect::<Vec<String>>();

        let mut ret = vec!["Pod:".to_string(), "  Labels:".to_string()];

        ret.extend(labels);
        ret.push("  IP:".to_string());
        ret.push(format!("    HostIP: {}", self.status.host_ip));
        ret.push(format!("    PodIP: {}", self.status.pod_ip));
        ret.push(format!("    PodIPs:"));
        ret.extend(pod_ips);

        ret
    }
}

#[derive(Deserialize, Serialize, Debug)]
struct PodDescription {
    pod: Pod,
}

pub(super) struct PodDescriptionWorker<'a> {
    client: &'a KubeClient,
    tx: &'a Sender<Event>,
    namespace: String,
    name: String,
    url: String,
}

#[async_trait::async_trait]
impl<'a> DescriptionWorker<'a> for PodDescriptionWorker<'a> {
    fn new(client: &'a KubeClient, tx: &'a Sender<Event>, namespace: String, name: String) -> Self {
        let url = format!("api/v1/namespaces/{}/pods/{}", namespace, name);

        PodDescriptionWorker {
            client,
            tx,
            namespace,
            name,
            url,
        }
    }

    // TODO 関連するService, Ingress, NetworkPolicyの情報を合わせて表示する
    async fn run(&self) -> Result<()> {
        let value = self.fetch_pod().await?;

        self.tx
            .send(NetworkMessage::Response(Ok(value.to_string_vec())).into())?;

        Ok(())
    }
}

impl<'a> PodDescriptionWorker<'a> {
    async fn fetch_pod(&self) -> Result<Pod> {
        let res = self.client.request_text(&self.url).await?;

        let value: Pod = serde_json::from_str(&res)?;
        // let value: serde_yaml::Value = serde_json::from_str(&res)?;

        Ok(value)
    }
}
