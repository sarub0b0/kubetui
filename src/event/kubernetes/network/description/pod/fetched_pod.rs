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

                Some(ret)
            } else {
                None
            }
        } else {
            None
        }
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

                Some(ret)
            } else {
                None
            }
        } else {
            None
        }
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
                Some(ret)
            } else {
                None
            }
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod metadata {
        use super::*;

        use indoc::indoc;
        use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
        use pretty_assertions::assert_eq;

        #[test]
        fn labels() {
            let labels = vec![
                ("foo".to_string(), "bar".to_string()),
                ("hoge".to_string(), "fuga".to_string()),
            ];

            let actual = Pod {
                metadata: ObjectMeta {
                    labels: Some(labels.into_iter().collect()),
                    name: Some("test".to_string()),
                    ..Default::default()
                },
                spec: None,
                status: None,
            };

            let expected = indoc!(
                "
                pod:
                  labels:
                    foo: bar
                    hoge: fuga
                "
            )
            .lines()
            .map(ToString::to_string)
            .collect::<Vec<String>>();

            assert_eq!(FetchedPod(actual).to_vec_string(), expected);
        }
    }

    mod spec {
        use super::*;

        use indoc::indoc;
        use k8s_openapi::{
            api::core::v1::{Container, ContainerPort},
            apimachinery::pkg::apis::meta::v1::ObjectMeta,
        };
        use pretty_assertions::assert_eq;

        #[test]
        fn containers() {
            let actual = Pod {
                metadata: ObjectMeta {
                    labels: None,
                    name: Some("test".to_string()),
                    ..Default::default()
                },
                spec: Some(PodSpec {
                    containers: vec![
                        Container {
                            name: "test".to_string(),
                            image: Some("test".to_string()),
                            ports: None,
                            ..Default::default()
                        },
                        Container {
                            name: "test2".to_string(),
                            image: Some("test2".to_string()),
                            ports: Some(vec![
                                ContainerPort {
                                    container_port: 8080,
                                    protocol: Some("TCP".to_string()),
                                    ..Default::default()
                                },
                                ContainerPort {
                                    container_port: 8081,
                                    protocol: Some("TCP".to_string()),
                                    name: Some("test".to_string()),
                                    ..Default::default()
                                },
                            ]),
                            ..Default::default()
                        },
                    ],
                    ..Default::default()
                }),
                status: None,
            };

            let expected = indoc!(
                "
                pod:
                  containers:
                    - name: test
                      image: test
                    - name: test2
                      image: test2
                      ports:
                        - containerPort: 8080
                          protocol: TCP
                        - containerPort: 8081
                          name: test
                          protocol: TCP
                "
            )
            .lines()
            .map(ToString::to_string)
            .collect::<Vec<String>>();

            assert_eq!(FetchedPod(actual).to_vec_string(), expected);
        }
    }

    mod status {
        use super::*;

        use indoc::indoc;
        use k8s_openapi::{
            api::core::v1::{PodCondition, PodIP},
            apimachinery::pkg::apis::meta::v1::ObjectMeta,
        };
        use pretty_assertions::assert_eq;

        #[test]
        fn pod_status() {
            let actual = Pod {
                metadata: ObjectMeta {
                    labels: None,
                    name: Some("test".to_string()),
                    ..Default::default()
                },
                spec: None,
                status: Some(PodStatus {
                    host_ip: Some("test".to_string()),
                    pod_ip: Some("test".to_string()),
                    pod_ips: Some(vec![
                        PodIP {
                            ip: Some("0.0.0.0".to_string()),
                        },
                        PodIP {
                            ip: Some("0.0.0.0".to_string()),
                        },
                    ]),
                    phase: Some("test".to_string()),
                    conditions: Some(vec![
                        PodCondition {
                            type_: "test".to_string(),
                            status: "test".to_string(),
                            ..Default::default()
                        },
                        PodCondition {
                            type_: "test2".to_string(),
                            status: "test2".to_string(),
                            ..Default::default()
                        },
                    ]),
                    ..Default::default()
                }),
            };

            let expected = indoc!(
                "
                pod:
                  hostIP: test
                  podIP: test
                  podIPs: 0.0.0.0, 0.0.0.0
                  phase: test
                "
            )
            .lines()
            .map(ToString::to_string)
            .collect::<Vec<String>>();

            assert_eq!(FetchedPod(actual).to_vec_string(), expected);
        }
    }
}
