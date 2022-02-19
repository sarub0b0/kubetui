use k8s_openapi::{
    api::core::v1::{Pod, PodSpec, PodStatus},
    apimachinery::pkg::apis::meta::v1::ObjectMeta,
};

use super::*;

#[derive(Deserialize, Serialize, Debug)]
pub struct FetchedPod(pub Pod);

impl FetchedPod {
    pub fn to_vec_string(&self) -> Vec<String> {
        let mut ret = vec!["pod:".to_string()];

        if let Some(value) = Self::metadata(&self.0.metadata) {
            ret.extend(value);
        }

        if let Some(value) = Self::spec(&self.0.spec) {
            ret.extend(value);
        }

        if let Some(value) = Self::status(&self.0.status) {
            ret.extend(value)
        }

        ret
    }

    fn metadata(metadata: &ObjectMeta) -> Option<Vec<String>> {
        if let Some(labels) = &metadata.labels {
            let labels = labels
                .iter()
                .map(|(k, v)| format!("    {}: {}", k, v))
                .collect::<Vec<String>>();

            if !labels.is_empty() {
                let mut ret = vec!["  labels:".to_string()];

                ret.extend(labels);

                return Some(ret);
            }
        }

        None
    }

    fn spec(spec: &Option<PodSpec>) -> Option<Vec<String>> {
        if let Some(spec) = spec {
            let containers: Vec<String> = spec
                .containers
                .iter()
                .flat_map(|c| {
                    let mut vec = vec![format!("    - name: {}", c.name)];

                    if let Some(image) = &c.image {
                        vec.push(format!("      image: {}", image));
                    }

                    // そのままserde_yaml::to_stringにいれると"~"になるため中身を取り出す処理をいれている
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

            if !containers.is_empty() {
                let mut ret = vec!["  containers:".to_string()];
                ret.extend(containers);

                return Some(ret);
            }
        }

        None
    }

    fn status(status: &Option<PodStatus>) -> Option<Vec<String>> {
        if let Some(status) = status {
            let mut ret = Vec::new();

            if let Some(host_ip) = &status.host_ip {
                ret.push(format!("  hostIP: {}", host_ip));
            }

            if let Some(pod_ip) = &status.pod_ip {
                ret.push(format!("  podIP: {}", pod_ip));
            }

            if let Some(pod_ips) = &status.pod_ips {
                let ips = pod_ips
                    .iter()
                    .cloned()
                    .filter_map(|ip| ip.ip)
                    .collect::<Vec<String>>()
                    .join(", ");

                if !ips.is_empty() {
                    ret.push(format!("  podIPs: {}", ips));
                }
            }

            if let Some(phase) = &status.phase {
                ret.push(format!("  phase: {}", phase));
            }

            if !ret.is_empty() {
                return Some(ret);
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name() {}
}
