use k8s_openapi::{
    api::core::v1::{LoadBalancerStatus, Service, ServiceSpec, ServiceStatus},
    apimachinery::pkg::apis::meta::v1::{Condition, ObjectMeta},
    List,
};

pub type FetchedServiceList = List<Service>;

pub struct FetchedService(pub Service);

impl FetchedService {
    pub fn to_vec_string(&self) -> Vec<String> {
        let mut ret = vec!["service:".to_string()];

        if let Some(value) = Self::metadata(&self.0.metadata) {
            ret.extend(value);
        }

        if let Some(value) = Self::spec(&self.0.spec) {
            ret.extend(value);
        }

        if let Some(value) = Self::status(&self.0.status) {
            ret.extend(value);
        }

        ret
    }

    fn metadata(metadata: &ObjectMeta) -> Option<Vec<String>> {
        if let Some(name) = &metadata.name {
            Some(vec![format!("  name: {}", name)])
        } else {
            None
        }
    }

    fn spec(spec: &Option<ServiceSpec>) -> Option<Vec<String>> {
        if let Some(spec) = spec {
            let mut ret = Vec::new();

            if let Some(cluster_ip) = &spec.cluster_ip {
                ret.push(format!("  clusterIP: {}", cluster_ip));
            }

            if let Some(cluster_ips) = &spec.cluster_ips {
                // 縦に長くなりがちのためカンマくぎりで表示
                let ips = cluster_ips.join(", ");

                if !ips.is_empty() {
                    ret.push(format!("  clusterIPs: {}", ips));
                }
            }

            if let Some(external_ips) = &spec.external_ips {
                ret.push(format!("  externalIPs: {:?}", external_ips));
            }

            if let Some(external_name) = &spec.external_name {
                ret.push(format!("  externalName: {}", external_name));
            }

            if let Some(health_check_node_port) = &spec.health_check_node_port {
                ret.push(format!("  healthCheckNodePort: {}", health_check_node_port));
            }

            if let Some(load_balancer_ip) = &spec.load_balancer_ip {
                ret.push(format!("  loadBalancerIP: {}", load_balancer_ip));
            }

            if let Some(ports) = &spec.ports {
                if let Ok(yaml) = serde_yaml::to_string(&ports) {
                    let v: Vec<String> =
                        yaml.lines().skip(1).map(|y| format!("    {}", y)).collect();

                    if !v.is_empty() {
                        ret.push("  ports:".to_string());
                        ret.extend(v);
                    }
                }
            }

            if let Some(type_) = &spec.type_ {
                ret.push(format!("  type: {}", type_));
            }

            if !ret.is_empty() {
                return Some(ret);
            }
        }

        None
    }

    fn status(status: &Option<ServiceStatus>) -> Option<Vec<String>> {
        if let Some(status) = status {
            let mut ret: Vec<String> = Vec::new();

            if let Some(load_balancer) = &status.load_balancer {
                if let Some(value) = Self::load_balancer(&load_balancer) {
                    ret.extend(value);
                }
            }

            if let Some(conditions) = &status.conditions {
                if let Some(value) = Self::conditions(&conditions) {
                    ret.extend(value);
                }
            }

            if !ret.is_empty() {
                return Some(ret);
            }
        }
        None
    }

    fn load_balancer(load_balancer: &LoadBalancerStatus) -> Option<Vec<String>> {
        if let Some(ingresses) = &load_balancer.ingress {
            if !ingresses.is_empty() {
                if let Ok(yaml) = serde_yaml::to_string(ingresses) {
                    let v: Vec<String> = yaml
                        .lines()
                        .skip(1)
                        .map(|y| format!("      {}", y))
                        .collect();

                    if !v.is_empty() {
                        let mut ret =
                            vec!["  loadBalancer:".to_string(), "    ingress:".to_string()];
                        ret.extend(v);

                        return Some(ret);
                    }
                }
            }
        }
        None
    }

    fn conditions(conditions: &[Condition]) -> Option<Vec<String>> {
        let conditions_vec: Vec<String> = conditions
            .iter()
            .flat_map(|condition| {
                let mut v = vec![format!("    - message: {}", condition.message)];

                v.push(format!(
                    "      lastTransitionTime: {}",
                    condition.last_transition_time.0.to_rfc3339()
                ));

                if let Some(observed_generation) = &condition.observed_generation {
                    v.push(format!("      observedGeneration: {}", observed_generation));
                }

                v.push(format!("      reason: {}", condition.reason));
                v.push(format!("      status: {}", condition.status));
                v.push(format!("      type: {}", condition.type_));

                v
            })
            .collect();

        if !conditions_vec.is_empty() {
            let mut ret = vec!["  conditions:".to_string()];
            ret.extend(conditions_vec);

            Some(ret)
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

                assert_eq!(FetchedService(actual).to_vec_string(), expected);
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

                    assert_eq!(FetchedService(actual).to_vec_string(), expected);
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

                    assert_eq!(FetchedService(actual).to_vec_string(), expected);
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

                    assert_eq!(FetchedService(actual).to_vec_string(), expected);
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

                    assert_eq!(FetchedService(actual).to_vec_string(), expected);
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

                    assert_eq!(FetchedService(actual).to_vec_string(), expected);
                }
            }
        }
    }
}
