use k8s_openapi::{
    api::core::v1::{Service, ServiceSpec, ServiceStatus},
    apimachinery::pkg::apis::meta::v1::ObjectMeta,
    List,
};
use serde_yaml::{Mapping, Sequence, Value};

pub type FetchedServiceList = List<Service>;

pub struct FetchedService(pub Vec<Service>);

impl FetchedService {
    pub fn to_vec_string(&self) -> Vec<String> {
        let mut seq = Sequence::new();

        for service in &self.0 {
            let mut map = Mapping::new();

            if let Some(Value::Mapping(value)) = Self::metadata(&service.metadata) {
                map.extend(value);
            }

            if let Some(Value::Mapping(value)) = Self::spec(&service.spec) {
                map.extend(value);
            }

            if let Some(Value::Mapping(value)) = Self::status(&service.status) {
                map.extend(value);
            }

            seq.push(map.into());
        }

        if !seq.is_empty() {
            let mut root = Mapping::new();
            root.insert("service".into(), seq.into());

            if let Ok(yaml) = serde_yaml::to_string(&root) {
                yaml.lines().skip(1).map(ToString::to_string).collect()
            } else {
                vec![]
            }
        } else {
            vec![]
        }
    }

    fn metadata(metadata: &ObjectMeta) -> Option<Value> {
        metadata.name.as_ref().map(|name| {
            let mut map = Mapping::new();
            map.insert("name".into(), name.to_string().into());
            map.into()
        })
    }

    fn spec(spec: &Option<ServiceSpec>) -> Option<Value> {
        if let Some(spec) = spec {
            let mut map = Mapping::new();

            if let Some(cluster_ip) = &spec.cluster_ip {
                map.insert("clusterIP".into(), cluster_ip.to_string().into());
            }

            if let Some(cluster_ips) = &spec.cluster_ips {
                // 縦に長くなりがちのためカンマくぎりで表示
                let ips = cluster_ips.join(", ");
                if !ips.is_empty() {
                    map.insert("clusterIPs".into(), ips.into());
                }
            }

            if let Some(external_ips) = &spec.external_ips {
                let ips = external_ips.join(", ");
                if !ips.is_empty() {
                    map.insert("externalIPs".into(), ips.to_string().into());
                }
            }

            if let Some(external_name) = &spec.external_name {
                map.insert("externalName".into(), external_name.to_string().into());
            }

            if let Some(health_check_node_port) = &spec.health_check_node_port {
                map.insert(
                    "healthCheckNodePort".into(),
                    health_check_node_port.to_string().into(),
                );
            }

            if let Some(load_balancer_ip) = &spec.load_balancer_ip {
                map.insert("loadBalancerIP".into(), load_balancer_ip.to_string().into());
            }

            if let Some(ports) = &spec.ports {
                if let Ok(value) = serde_yaml::to_value(ports) {
                    map.insert("ports".into(), value);
                }
            }

            if let Some(type_) = &spec.type_ {
                map.insert("type".into(), type_.to_string().into());
            }

            Some(map.into())
        } else {
            None
        }
    }

    fn status(status: &Option<ServiceStatus>) -> Option<Value> {
        if let Some(status) = status {
            let mut map = Mapping::new();

            if let Some(load_balancer) = &status.load_balancer {
                if let Some(ingresses) = &load_balancer.ingress {
                    if !ingresses.is_empty() {
                        if let Ok(value) = serde_yaml::to_value(load_balancer) {
                            map.insert("loadBalancer".into(), value);
                        }
                    }
                }
            }

            if let Some(conditions) = &status.conditions {
                if !conditions.is_empty() {
                    if let Ok(value) = serde_yaml::to_value(conditions) {
                        map.insert("conditions".into(), value);
                    }
                }
            }

            if !map.is_empty() {
                Some(map.into())
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

    mod to_vec_string {
        use super::*;

        use chrono::{DateTime, NaiveDate, Utc};
        use indoc::indoc;
        use k8s_openapi::apimachinery::pkg::apis::meta::v1::{Condition, ObjectMeta, Time};

        fn test_time() -> Time {
            Time(DateTime::<Utc>::from_utc(
                NaiveDate::from_ymd(2019, 1, 1).and_hms(0, 0, 0),
                Utc,
            ))
        }

        mod multiple {
            use super::*;
            use k8s_openapi::{
                api::core::v1::ServicePort, apimachinery::pkg::util::intstr::IntOrString,
            };
            use pretty_assertions::assert_eq;

            #[test]
            fn multiple() {
                let actual = vec![
                    Service {
                        metadata: ObjectMeta {
                            name: Some("test0".into()),
                            ..Default::default()
                        },
                        spec: Some(ServiceSpec {
                            cluster_ip: Some("0.0.0.0".to_string()),
                            cluster_ips: Some(vec!["0.0.0.0".to_string(), "0.0.0.0".to_string()]),
                            ports: Some(vec![ServicePort {
                                port: 80,
                                protocol: Some("TCP".to_string()),
                                target_port: Some(IntOrString::Int(80)),
                                ..Default::default()
                            }]),
                            type_: Some("ClusterIP".to_string()),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                    Service {
                        metadata: ObjectMeta {
                            name: Some("test1".into()),
                            ..Default::default()
                        },
                        spec: Some(ServiceSpec {
                            cluster_ip: Some("0.0.0.0".to_string()),
                            cluster_ips: Some(vec!["0.0.0.0".to_string(), "0.0.0.0".to_string()]),
                            ports: Some(vec![ServicePort {
                                port: 80,
                                protocol: Some("TCP".to_string()),
                                target_port: Some(IntOrString::Int(80)),
                                ..Default::default()
                            }]),
                            type_: Some("ClusterIP".to_string()),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                ];

                let expected = indoc! {
                    "
                    service:
                      - name: test0
                        clusterIP: 0.0.0.0
                        clusterIPs: \"0.0.0.0, 0.0.0.0\"
                        ports:
                          - port: 80
                            protocol: TCP
                            targetPort: 80
                        type: ClusterIP
                      - name: test1
                        clusterIP: 0.0.0.0
                        clusterIPs: \"0.0.0.0, 0.0.0.0\"
                        ports:
                          - port: 80
                            protocol: TCP
                            targetPort: 80
                        type: ClusterIP
                    "
                }
                .lines()
                .map(ToString::to_string)
                .collect::<Vec<_>>();

                assert_eq!(FetchedService(actual).to_vec_string(), expected);
            }
        }

        mod metadata {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn name() {
                let actual = vec![Service {
                    metadata: ObjectMeta {
                        name: Some("test".to_string()),
                        ..Default::default()
                    },
                    ..Default::default()
                }];

                let expected = indoc! { "
                service:
                  - name: test
                " }
                .lines()
                .map(ToString::to_string)
                .collect::<Vec<_>>();

                assert_eq!(FetchedService(actual).to_vec_string(), expected);
            }
        }

        mod spec {
            use super::*;

            use k8s_openapi::{
                api::core::v1::ServicePort, apimachinery::pkg::util::intstr::IntOrString,
            };
            use pretty_assertions::assert_eq;

            #[test]
            fn spec() {
                let actual = vec![Service {
                    spec: Some(ServiceSpec {
                        cluster_ip: Some("0.0.0.0".to_string()),
                        cluster_ips: Some(vec!["0.0.0.0".to_string(), "0.0.0.0".to_string()]),
                        ports: Some(vec![ServicePort {
                            port: 80,
                            protocol: Some("TCP".to_string()),
                            target_port: Some(IntOrString::Int(80)),
                            ..Default::default()
                        }]),
                        type_: Some("ClusterIP".to_string()),
                        ..Default::default()
                    }),
                    ..Default::default()
                }];

                let expected = indoc! {
                    "
                    service:
                      - clusterIP: 0.0.0.0
                        clusterIPs: \"0.0.0.0, 0.0.0.0\"
                        ports:
                          - port: 80
                            protocol: TCP
                            targetPort: 80
                        type: ClusterIP
                    "

                }
                .lines()
                .map(ToString::to_string)
                .collect::<Vec<_>>();

                assert_eq!(FetchedService(actual).to_vec_string(), expected);
            }
        }

        mod status {
            use super::*;

            mod load_balancer {
                use super::*;
                use k8s_openapi::api::core::v1::{
                    LoadBalancerIngress, LoadBalancerStatus, PortStatus, ServiceStatus,
                };
                use pretty_assertions::assert_eq;

                #[test]
                fn 値をもつとき出力する() {
                    let actual = vec![Service {
                        metadata: ObjectMeta {
                            name: Some("test".to_string()),
                            ..Default::default()
                        },
                        spec: None,
                        status: Some(ServiceStatus {
                            conditions: None,
                            load_balancer: Some(LoadBalancerStatus {
                                ingress: Some(vec![
                                    LoadBalancerIngress {
                                        hostname: Some("hostname".to_string()),
                                        ip: Some("0.0.0.0".to_string()),
                                        ports: Some(vec![
                                            PortStatus {
                                                error: Some("test".to_string()),
                                                port: 0,
                                                protocol: "TCP".to_string(),
                                            },
                                            PortStatus {
                                                error: Some("test".to_string()),
                                                port: 0,
                                                protocol: "TCP".to_string(),
                                            },
                                        ]),
                                    },
                                    LoadBalancerIngress {
                                        hostname: Some("hostname".to_string()),
                                        ip: None,
                                        ports: Some(vec![PortStatus {
                                            error: None,
                                            port: 0,
                                            protocol: "TCP".to_string(),
                                        }]),
                                    },
                                ]),
                            }),
                        }),
                    }];

                    let expected = indoc! { "
                    service:
                      - name: test
                        loadBalancer:
                          ingress:
                            - hostname: hostname
                              ip: 0.0.0.0
                              ports:
                                - error: test
                                  port: 0
                                  protocol: TCP
                                - error: test
                                  port: 0
                                  protocol: TCP
                            - hostname: hostname
                              ports:
                                - port: 0
                                  protocol: TCP
                    "}
                    .lines()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>();

                    assert_eq!(FetchedService(actual).to_vec_string(), expected);
                }

                #[test]
                fn someでかつ空のとき出力しない() {
                    let actual = vec![Service {
                        metadata: ObjectMeta {
                            name: Some("test".to_string()),
                            ..Default::default()
                        },
                        spec: None,
                        status: Some(ServiceStatus {
                            conditions: None,
                            load_balancer: Some(LoadBalancerStatus {
                                ingress: Some(vec![]),
                            }),
                        }),
                    }];

                    let expected = indoc! { "
                    service:
                      - name: test
                    "}
                    .lines()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>();

                    assert_eq!(FetchedService(actual).to_vec_string(), expected);
                }

                #[test]
                fn noneのとき出力しない() {
                    let actual = vec![Service {
                        metadata: ObjectMeta {
                            name: Some("test".to_string()),
                            ..Default::default()
                        },
                        spec: None,
                        status: Some(ServiceStatus {
                            conditions: None,
                            load_balancer: Some(LoadBalancerStatus { ingress: None }),
                        }),
                    }];

                    let expected = indoc! { "
                    service:
                      - name: test
                    "}
                    .lines()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>();

                    assert_eq!(FetchedService(actual).to_vec_string(), expected);
                }
            }

            mod conditions {
                use super::*;
                use k8s_openapi::api::core::v1::ServiceStatus;
                use pretty_assertions::assert_eq;

                #[test]
                fn noneのとき出力しない() {
                    let actual = vec![Service {
                        metadata: ObjectMeta {
                            name: Some("test".to_string()),
                            ..Default::default()
                        },
                        spec: Some(ServiceSpec {
                            ..Default::default()
                        }),
                        status: Some(ServiceStatus {
                            load_balancer: None,
                            conditions: None,
                        }),
                    }];

                    let expected = indoc! { "
                    service:
                      - name: test
                    " }
                    .lines()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>();

                    assert_eq!(FetchedService(actual).to_vec_string(), expected);
                }

                #[test]
                fn someでかつ空のとき出力しない() {
                    let actual = vec![Service {
                        metadata: ObjectMeta {
                            name: Some("test".to_string()),
                            ..Default::default()
                        },
                        spec: Some(ServiceSpec {
                            ..Default::default()
                        }),
                        status: Some(ServiceStatus {
                            load_balancer: None,
                            conditions: Some(vec![]),
                        }),
                    }];

                    let expected = indoc! { "
                    service:
                      - name: test
                    " }
                    .lines()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>();

                    assert_eq!(FetchedService(actual).to_vec_string(), expected);
                }

                #[test]
                fn 値をもつとき出力する() {
                    let actual = vec![Service {
                        metadata: ObjectMeta {
                            name: Some("test".to_string()),
                            ..Default::default()
                        },
                        spec: Some(ServiceSpec {
                            ..Default::default()
                        }),
                        status: Some(ServiceStatus {
                            load_balancer: None,
                            conditions: Some(vec![
                                Condition {
                                    last_transition_time: test_time(),
                                    message: "test".to_string(),
                                    observed_generation: Some(0),
                                    reason: "test".to_string(),
                                    status: "test".to_string(),
                                    type_: "test".to_string(),
                                },
                                Condition {
                                    last_transition_time: test_time(),
                                    message: "test".to_string(),
                                    observed_generation: None,
                                    reason: "test".to_string(),
                                    status: "test".to_string(),
                                    type_: "test".to_string(),
                                },
                            ]),
                        }),
                    }];

                    let expected = indoc! { "
                    service:
                      - name: test
                        conditions:
                          - lastTransitionTime: \"2019-01-01T00:00:00Z\"
                            message: test
                            observedGeneration: 0
                            reason: test
                            status: test
                            type: test
                          - lastTransitionTime: \"2019-01-01T00:00:00Z\"
                            message: test
                            reason: test
                            status: test
                            type: test
                    " }
                    .lines()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>();

                    assert_eq!(FetchedService(actual).to_vec_string(), expected);
                }
            }
        }
    }
}
