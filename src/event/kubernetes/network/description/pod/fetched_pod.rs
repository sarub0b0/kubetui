use k8s_openapi::{
    api::core::v1::{Pod, PodSpec, PodStatus},
    apimachinery::pkg::apis::meta::v1::ObjectMeta,
};
use serde_yaml::{Mapping, Value};

use super::*;

#[derive(Deserialize, Serialize, Debug)]
pub struct FetchedPod(pub Pod);

impl FetchedPod {
    pub fn to_vec_string(&self) -> Vec<String> {
        let mut map = Mapping::new();

        if let Some(Value::Mapping(value)) = Self::metadata(&self.0.metadata) {
            map.extend(value);
        }

        if let Some(Value::Mapping(value)) = Self::spec(&self.0.spec) {
            map.extend(value);
        }

        if let Some(Value::Mapping(value)) = Self::status(&self.0.status) {
            map.extend(value);
        }

        if !map.is_empty() {
            let mut root = Mapping::new();
            root.insert("pod".into(), map.into());

            if let Ok(yaml) = serde_yaml::to_string(&root) {
                return yaml.lines().skip(1).map(ToString::to_string).collect();
            }
        }

        vec![]
    }

    fn metadata(metadata: &ObjectMeta) -> Option<Value> {
        if let Some(labels) = &metadata.labels {
            if !labels.is_empty() {
                if let Ok(value) = serde_yaml::to_value(labels) {
                    let mut map = Mapping::new();
                    map.insert("labels".into(), value);

                    return Some(map.into());
                }
            }
        }

        None
    }

    fn spec(spec: &Option<PodSpec>) -> Option<Value> {
        if let Some(spec) = spec {
            let values: Vec<Value> = spec
                .containers
                .iter()
                .map(|c| {
                    let mut map = Mapping::new();

                    map.insert("name".into(), c.name.to_string().into());

                    if let Some(image) = &c.image {
                        map.insert("image".into(), image.to_string().into());
                    }

                    if let Some(ports) = &c.ports {
                        if let Ok(value) = serde_yaml::to_value(ports) {
                            map.insert("ports".into(), value);
                        }
                    }

                    map.into()
                })
                .collect();

            if !values.is_empty() {
                let mut map = Mapping::new();
                map.insert("containers".into(), values.into());

                return Some(map.into());
            }
        }

        None
    }

    fn status(status: &Option<PodStatus>) -> Option<Value> {
        if let Some(status) = status {
            let mut map = Mapping::new();

            if let Some(host_ip) = &status.host_ip {
                map.insert("hostIP".into(), host_ip.to_string().into());
            }

            if let Some(pod_ip) = &status.pod_ip {
                map.insert("podIP".into(), pod_ip.to_string().into());
            }

            if let Some(pod_ips) = &status.pod_ips {
                let ips = pod_ips
                    .iter()
                    .cloned()
                    .filter_map(|ip| ip.ip)
                    .collect::<Vec<String>>()
                    .join(", ");

                if !ips.is_empty() {
                    map.insert("podIPs".into(), ips.into());
                }
            }

            if let Some(phase) = &status.phase {
                map.insert("phase".into(), phase.to_string().into());
            }

            if !map.is_empty() {
                return Some(map.into());
            }
        }

        None
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
                  podIPs: \"0.0.0.0, 0.0.0.0\"
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
