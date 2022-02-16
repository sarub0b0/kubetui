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

                    // そのままserde_yaml::to_stringにいれると"~"になるため中身を取り出す処理をいれ
                    // ている
                    if let Some(ports) = &c.ports {
                        if let Ok(ports) = serde_yaml::to_string(ports) {
                            let v = ports
                                .lines()
                                .skip(1)
                                .map(|p| format!("        {}", p))
                                .collect::<Vec<String>>();

                            if !v.is_empty() {
                                vec.push("      ports:".to_string());
                                vec.extend(v);
                            }
                        }
                    }

                    vec
                })
                .collect();

            ret.extend(containers);
        }

        if let Some(status) = &self.0.status {
            if let Some(host_ip) = &status.host_ip {
                ret.push(format!("  hostIP: {}", host_ip));
            }

            if let Some(pod_ip) = &status.pod_ip {
                ret.push(format!("  podIP: {}", pod_ip));
            }

            if let Some(pod_ips) = &status.pod_ips {
                let ips: Vec<&String> = pod_ips.iter().filter_map(|ip| ip.ip.as_ref()).collect();

                if let Ok(yaml) = serde_yaml::to_string(&ips) {
                    let v: Vec<String> = yaml
                        .lines()
                        .skip(1)
                        .map(|ip| format!("    {}", ip))
                        .collect();

                    if !v.is_empty() {
                        ret.push("  podIPs:".to_string());
                        ret.extend(v);
                    }
                }
            }

            if let Some(phase) = &status.phase {
                ret.push(format!("  phase: {}", phase));
            }
        }

        ret
    }
}
