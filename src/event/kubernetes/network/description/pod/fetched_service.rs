use k8s_openapi::{
    api::core::v1::{LoadBalancerIngress, LoadBalancerStatus, Service, ServiceSpec},
    apimachinery::pkg::apis::meta::v1::Condition,
    List,
};

pub type FetchedServiceList = List<Service>;

pub struct FetchedService(pub Service);

impl FetchedService {
    pub fn to_string_vec(&self) -> Vec<String> {
        let mut ret = vec!["service:".to_string()];

        if let Some(name) = &self.0.metadata.name {
            ret.push(format!("  name: {}", name));
        }

        if let Some(spec) = &self.0.spec {
            Self::spec(spec, &mut ret);
        }

        if let Some(status) = &self.0.status {
            Self::load_balancer(&status.load_balancer, &mut ret);
            Self::conditions(&status.conditions, &mut ret);
        }

        ret
    }

    fn spec(spec: &ServiceSpec, vec: &mut Vec<String>) {
        if let Some(cluster_ip) = &spec.cluster_ip {
            vec.push(format!("  cluster_ip: {}", cluster_ip));
        }

        if let Some(cluster_ips) = &spec.cluster_ips {
            // 縦に長くなりがちのためカンマくぎりで表示
            let ips = cluster_ips.join(", ");

            if !ips.is_empty() {
                vec.push(format!("  clusterIPs: {}", ips));
            }
        }

        if let Some(external_ips) = &spec.external_ips {
            vec.push(format!("  externalIPs: {:?}", external_ips));
        }

        if let Some(external_name) = &spec.external_name {
            vec.push(format!("  externalName: {}", external_name));
        }

        if let Some(health_check_node_port) = &spec.health_check_node_port {
            vec.push(format!("  healthCheckNodePort: {}", health_check_node_port));
        }

        if let Some(load_balancer_ip) = &spec.load_balancer_ip {
            vec.push(format!("  load_balancer_ip: {}", load_balancer_ip));
        }

        if let Some(ports) = &spec.ports {
            if let Ok(yaml) = serde_yaml::to_string(&ports) {
                let v: Vec<String> = yaml.lines().skip(1).map(|y| format!("    {}", y)).collect();

                if !v.is_empty() {
                    vec.push("  ports:".to_string());
                    vec.extend(v);
                }
            }
        }

        if let Some(type_) = &spec.type_ {
            vec.push(format!("  type: {}", type_));
        }
    }

    fn load_balancer(load_balancer: &Option<LoadBalancerStatus>, vec: &mut Vec<String>) {
        if let Some(load_balancer) = load_balancer {
            if let Some(ingresses) = &load_balancer.ingress {
                let ingresses_vec: Vec<String> = Self::ingresses(ingresses);

                if !ingresses_vec.is_empty() {
                    vec.push("  loadBalancer:".to_string());
                    vec.push("    ingress:".to_string());
                    vec.extend(ingresses_vec);
                }
            }
        }
    }

    fn ingresses(ingresses: &[LoadBalancerIngress]) -> Vec<String> {
        ingresses
            .iter()
            .flat_map(|ingress| {
                let mut v = vec![];

                let mut has_value = false;

                let hyphen_or_space = |has_value: &mut bool| {
                    if *has_value {
                        " "
                    } else {
                        *has_value = true;
                        "-"
                    }
                };

                if let Some(ip) = &ingress.ip {
                    v.push(format!(
                        "      {} ip: {}",
                        hyphen_or_space(&mut has_value),
                        ip
                    ));
                }

                if let Some(hostname) = &ingress.hostname {
                    v.push(format!(
                        "      {} hostname: {}",
                        hyphen_or_space(&mut has_value),
                        hostname
                    ));
                }

                if let Some(ports) = &ingress.ports {
                    v.push(format!("      {} ports:", hyphen_or_space(&mut has_value)));

                    ports.iter().for_each(|port_status| {
                        v.push(format!("          - port: {}", port_status.port));
                        v.push(format!("            protocol: {}", port_status.protocol));
                        if let Some(error) = &port_status.error {
                            v.push(format!("            error: {}", error));
                        }
                    })
                }

                v
            })
            .collect()
    }

    fn conditions(conditions: &Option<Vec<Condition>>, vec: &mut Vec<String>) {
        if let Some(conditions) = &conditions {
            let conditions_vec: Vec<String> = conditions
                .iter()
                .flat_map(|condition| {
                    let mut v = vec![format!("      - message: {}", condition.message)];

                    v.push(format!(
                        "        lastTransitionTime: {}",
                        condition.last_transition_time.0.to_rfc3339()
                    ));

                    if let Some(observed_generation) = &condition.observed_generation {
                        v.push(format!(
                            "        observedGeneration: {}",
                            observed_generation
                        ));
                    }

                    v.push(format!("        reason: {}", condition.reason));
                    v.push(format!("        status: {}", condition.status));
                    v.push(format!("        type: {}", condition.type_));

                    v
                })
                .collect();

            if !conditions_vec.is_empty() {
                vec.push("    conditions:".to_string());
                vec.extend(conditions_vec)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod to_string_vec {
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

        mod metadata {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn name() {
                let actual = Service {
                    metadata: ObjectMeta {
                        name: Some("test".to_string()),
                        ..Default::default()
                    },
                    ..Default::default()
                };

                let expected = indoc! { "
                service:
                  name: test
                " }
                .lines()
                .map(ToString::to_string)
                .collect::<Vec<_>>();

                assert_eq!(FetchedService(actual).to_string_vec(), expected);
            }
        }

        mod spec {
            #[test]
            fn feature() {}
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
                    let actual = Service {
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
                    };

                    let expected = indoc! { "
                    service:
                      name: test
                      loadBalancer:
                        ingress:
                          - ip: 0.0.0.0
                            hostname: hostname
                            ports:
                              - port: 0
                                protocol: TCP
                                error: test
                              - port: 0
                                protocol: TCP
                                error: test
                          - hostname: hostname
                            ports:
                              - port: 0
                                protocol: TCP
                    "}
                    .lines()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>();

                    assert_eq!(FetchedService(actual).to_string_vec(), expected);
                }

                #[test]
                fn someでかつ空のとき出力しない() {
                    let actual = Service {
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
                    };

                    let expected = indoc! { "
                    service:
                      name: test
                    "}
                    .lines()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>();

                    assert_eq!(FetchedService(actual).to_string_vec(), expected);
                }

                #[test]
                fn noneのとき出力しない() {
                    let actual = Service {
                        metadata: ObjectMeta {
                            name: Some("test".to_string()),
                            ..Default::default()
                        },
                        spec: None,
                        status: Some(ServiceStatus {
                            conditions: None,
                            load_balancer: Some(LoadBalancerStatus { ingress: None }),
                        }),
                    };

                    let expected = indoc! { "
                    service:
                      name: test
                    "}
                    .lines()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>();

                    assert_eq!(FetchedService(actual).to_string_vec(), expected);
                }
            }

            mod conditions {
                use super::*;
                use k8s_openapi::api::core::v1::ServiceStatus;
                use pretty_assertions::assert_eq;

                #[test]
                fn noneのとき出力しない() {
                    let actual = Service {
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
                    };

                    let expected = indoc! { "
                    service:
                      name: test
                    " }
                    .lines()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>();

                    assert_eq!(FetchedService(actual).to_string_vec(), expected);
                }

                #[test]
                fn someでかつ空のとき出力しない() {
                    let actual = Service {
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
                    };

                    let expected = indoc! { "
                    service:
                      name: test
                    " }
                    .lines()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>();

                    assert_eq!(FetchedService(actual).to_string_vec(), expected);
                }

                #[test]
                fn 値をもつとき出力する() {
                    let actual = Service {
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
                    };

                    let expected = indoc! { "
                    service:
                      name: test
                        conditions:
                          - message: test
                            lastTransitionTime: 2019-01-01T00:00:00+00:00
                            observedGeneration: 0
                            reason: test
                            status: test
                            type: test
                          - message: test
                            lastTransitionTime: 2019-01-01T00:00:00+00:00
                            reason: test
                            status: test
                            type: test
                    " }
                    .lines()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>();

                    assert_eq!(FetchedService(actual).to_string_vec(), expected);
                }
            }
        }
    }
}
