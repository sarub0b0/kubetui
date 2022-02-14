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
use pod::*;

mod pod {
    use k8s_openapi::api::core::v1::Pod;

    use super::*;

    #[derive(Deserialize, Serialize, Debug)]
    pub struct FetchedPod(pub Pod);

    impl FetchedPod {
        pub fn to_string_vec(&self) -> Vec<String> {
            let mut ret = vec!["Pod:".to_string()];

            if let Some(labels) = &self.0.metadata.labels {
                let labels = labels
                    .iter()
                    .map(|(k, v)| format!("    {}: {}", k, v))
                    .collect::<Vec<String>>();

                ret.push("  Labels:".to_string());

                ret.extend(labels);
            }

            if let Some(status) = &self.0.status {
                let pod_ips = status
                    .pod_ips
                    .iter()
                    .flat_map(|v| {
                        v.iter()
                            .filter_map(|ip| ip.ip.as_ref().map(|ip| format!("      - {}", ip)))
                            .collect::<Vec<String>>()
                    })
                    .collect::<Vec<String>>();

                if status.host_ip.is_some() || status.pod_ip.is_some() || !pod_ips.is_empty() {
                    ret.push("  IP:".to_string());

                    if let Some(host_ip) = &status.host_ip {
                        ret.push(format!("    HostIP: {}", host_ip));
                    }

                    if let Some(pod_ip) = &status.pod_ip {
                        ret.push(format!("    PodIP: {}", pod_ip));
                    }

                    if !pod_ips.is_empty() {
                        ret.push("    PodIPs:".to_string());

                        ret.extend(pod_ips);
                    }
                }
            }

            if let Some(spec) = &self.0.spec {
                ret.push("  Containers:".to_string());

                let containers: Vec<String> = spec
                    .containers
                    .iter()
                    .flat_map(|c| {
                        let mut vec = vec![format!("    - Name: {}", c.name)];

                        if let Some(image) = &c.image {
                            vec.push(format!("      Image: {}", image));
                        }

                        if let Some(ports) = &c.ports {
                            vec.push("      Ports:".to_string());

                            ports.iter().for_each(|port| {
                                vec.push(format!("        ContainerPort: {}", port.container_port));

                                if let Some(host_ip) = &port.host_ip {
                                    vec.push(format!("        HostIP: {}", host_ip));
                                }

                                if let Some(host_port) = &port.host_port {
                                    vec.push(format!("        HostPort: {}", host_port));
                                }

                                if let Some(name) = &port.name {
                                    vec.push(format!("        Name: {}", name));
                                }

                                if let Some(protocol) = &port.protocol {
                                    vec.push(format!("        Protocol: {}", protocol));
                                }
                            })
                        }

                        vec
                    })
                    .collect();

                ret.extend(containers);
            }

            ret
        }
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
        let res = self.client.request_text(&self.url).await?;
    async fn fetch_pod(&self) -> Result<FetchedPod> {

        let value: FetchedPod = serde_json::from_str(&res)?;

        Ok(value)
    }
}
