use std::{collections::BTreeMap, fmt::Display};

use crossbeam::channel::Sender;
use k8s_openapi::api::core::v1::ContainerPort;
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
struct Container {
    image: String,
    name: String,
    ports: Option<Vec<ContainerPort>>,
}

#[derive(Deserialize, Serialize, Debug)]
struct Spec {
    containers: Vec<Container>,
}

#[derive(Deserialize, Serialize, Debug)]
struct Metadata {
    labels: BTreeMap<String, String>,
}

#[derive(Deserialize, Serialize, Debug)]
struct Pod {
    metadata: Metadata,
    status: Status,
    spec: Spec,
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
        ret.push("  Containers:".to_string());

        let containers: Vec<String> = self
            .spec
            .containers
            .iter()
            .flat_map(|c| {
                let mut ret = vec![format!("    - Image: {}", c.image)];

                if let Some(ports) = &c.ports {
                    ret.push(format!("      Ports:"));

                    ports.iter().for_each(|port| {
                        ret.push(format!("        ContainerPort: {}", port.container_port));

                        if let Some(host_ip) = &port.host_ip {
                            ret.push(format!("        HostIP: {}", host_ip));
                        }

                        if let Some(host_port) = &port.host_port {
                            ret.push(format!("        HostPort: {}", host_port));
                        }

                        if let Some(name) = &port.name {
                            ret.push(format!("        Name: {}", name));
                        }

                        if let Some(protocol) = &port.protocol {
                            ret.push(format!("        Protocol: {}", protocol));
                        }
                    })
                }

                ret
            })
            .collect();

        ret.extend(containers);

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
