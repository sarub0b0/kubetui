use k8s_openapi::api::core::v1::Pod;

use super::*;

#[derive(Deserialize, Serialize, Debug)]
pub struct FetchedPod(pub Pod);

impl FetchedPod {
    pub fn to_string_vec(&self) -> Vec<String> {
        let mut ret = vec!["pod:".to_string()];

        if let Some(labels) = &self.0.metadata.labels {
            let labels = labels
                .iter()
                .map(|(k, v)| format!("    {}: {}", k, v))
                .collect::<Vec<String>>();

            ret.push("  labels:".to_string());

            ret.extend(labels);
        }

        if let Some(spec) = &self.0.spec {
            ret.push("  containers:".to_string());

            let containers: Vec<String> = spec
                .containers
                .iter()
                .flat_map(|c| {
                    let mut vec = vec![format!("    - name: {}", c.name)];

                    if let Some(image) = &c.image {
                        vec.push(format!("      image: {}", image));
                    }

                    if let Some(ports) = &c.ports {
                        vec.push("      ports:".to_string());

                        ports.iter().for_each(|port| {
                            vec.push(format!("        containerPort: {}", port.container_port));

                            if let Some(host_ip) = &port.host_ip {
                                vec.push(format!("        hostIP: {}", host_ip));
                            }

                            if let Some(host_port) = &port.host_port {
                                vec.push(format!("        hostPort: {}", host_port));
                            }

                            if let Some(name) = &port.name {
                                vec.push(format!("        name: {}", name));
                            }

                            if let Some(protocol) = &port.protocol {
                                vec.push(format!("        protocol: {}", protocol));
                            }
                        })
                    }

                    vec
                })
                .collect();

            ret.extend(containers);
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
                ret.push("  ip:".to_string());

                if let Some(host_ip) = &status.host_ip {
                    ret.push(format!("    hostIP: {}", host_ip));
                }

                if let Some(pod_ip) = &status.pod_ip {
                    ret.push(format!("    podIP: {}", pod_ip));
                }

                if !pod_ips.is_empty() {
                    ret.push("    podIPs:".to_string());

                    ret.extend(pod_ips);
                }
            }

            if let Some(phase) = &status.phase {
                ret.push(format!("  phase: {}", phase));
            }
        }

        ret
    }
}
