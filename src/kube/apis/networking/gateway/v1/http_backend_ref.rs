// Generated from definition io.k8s.networking.gateway.v1.HTTPBackendRef

/// HTTPBackendRef defines how a HTTPRoute forwards a HTTP request. 
///  Note that when a namespace different than the local namespace is specified, a ReferenceGrant object is required in the referent namespace to allow that namespace's owner to accept the reference. See the ReferenceGrant documentation for details. 
///  \<gateway:experimental:description\> 
///  When the BackendRef points to a Kubernetes Service, implementations SHOULD honor the appProtocol field if it is set for the target Service Port. 
///  Implementations supporting appProtocol SHOULD recognize the Kubernetes Standard Application Protocols defined in KEP-3726. 
///  If a Service appProtocol isn't specified, an implementation MAY infer the backend protocol through its own means. Implementations MAY infer the protocol from the Route type referring to the backend Service. 
///  If a Route is not able to send traffic to the backend using the specified protocol then the backend is considered invalid. Implementations MUST set the "ResolvedRefs" condition to "False" with the "UnsupportedProtocol" reason. 
///  \</gateway:experimental:description\>
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HTTPBackendRef {
    /// Filters defined at this level should be executed if and only if the request is being forwarded to the backend defined here. 
    ///  Support: Implementation-specific (For broader support of filters, use the Filters field in HTTPRouteRule.)
    pub filters: Option<Vec<crate::kube::apis::networking::gateway::v1::HTTPRouteFilter>>,

    /// Group is the group of the referent. For example, "gateway.networking.k8s.io". When unspecified or empty string, core API group is inferred.
    pub group: Option<String>,

    /// Kind is the Kubernetes resource kind of the referent. For example "Service". 
    ///  Defaults to "Service" when not specified. 
    ///  ExternalName services can refer to CNAME DNS records that may live outside of the cluster and as such are difficult to reason about in terms of conformance. They also may not be safe to forward to (see CVE-2021-25740 for more information). Implementations SHOULD NOT support ExternalName Services. 
    ///  Support: Core (Services with a type other than ExternalName) 
    ///  Support: Implementation-specific (Services with type ExternalName)
    pub kind: Option<String>,

    /// Name is the name of the referent.
    pub name: String,

    /// Namespace is the namespace of the backend. When unspecified, the local namespace is inferred. 
    ///  Note that when a namespace different than the local namespace is specified, a ReferenceGrant object is required in the referent namespace to allow that namespace's owner to accept the reference. See the ReferenceGrant documentation for details. 
    ///  Support: Core
    pub namespace: Option<String>,

    /// Port specifies the destination port number to use for this resource. Port is required when the referent is a Kubernetes Service. In this case, the port number is the service port number, not the target port. For other resources, destination port might be derived from the referent resource or this field.
    pub port: Option<i32>,

    /// Weight specifies the proportion of requests forwarded to the referenced backend. This is computed as weight/(sum of all weights in this BackendRefs list). For non-zero values, there may be some epsilon from the exact proportion defined here depending on the precision an implementation supports. Weight is not a percentage and the sum of weights does not need to equal 100. 
    ///  If only one backend is specified and it has a weight greater than 0, 100% of the traffic is forwarded to that backend. If weight is set to 0, no traffic should be forwarded for this entry. If unspecified, weight defaults to 1. 
    ///  Support for this field varies based on the context where used.
    pub weight: Option<i32>,
}

impl crate::kube::apis::DeepMerge for HTTPBackendRef {
    fn merge_from(&mut self, other: Self) {
        crate::kube::apis::merge_strategies::list::atomic(&mut self.filters, other.filters);
        crate::kube::apis::DeepMerge::merge_from(&mut self.group, other.group);
        crate::kube::apis::DeepMerge::merge_from(&mut self.kind, other.kind);
        crate::kube::apis::DeepMerge::merge_from(&mut self.name, other.name);
        crate::kube::apis::DeepMerge::merge_from(&mut self.namespace, other.namespace);
        crate::kube::apis::DeepMerge::merge_from(&mut self.port, other.port);
        crate::kube::apis::DeepMerge::merge_from(&mut self.weight, other.weight);
    }
}

impl<'de> crate::kube::apis::serde::Deserialize<'de> for HTTPBackendRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: crate::kube::apis::serde::Deserializer<'de> {
        #[allow(non_camel_case_types)]
        enum Field {
            Key_filters,
            Key_group,
            Key_kind,
            Key_name,
            Key_namespace,
            Key_port,
            Key_weight,
            Other,
        }

        impl<'de> crate::kube::apis::serde::Deserialize<'de> for Field {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: crate::kube::apis::serde::Deserializer<'de> {
                struct Visitor;

                impl<'de> crate::kube::apis::serde::de::Visitor<'de> for Visitor {
                    type Value = Field;

                    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        f.write_str("field identifier")
                    }

                    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> where E: crate::kube::apis::serde::de::Error {
                        Ok(match v {
                            "filters" => Field::Key_filters,
                            "group" => Field::Key_group,
                            "kind" => Field::Key_kind,
                            "name" => Field::Key_name,
                            "namespace" => Field::Key_namespace,
                            "port" => Field::Key_port,
                            "weight" => Field::Key_weight,
                            _ => Field::Other,
                        })
                    }
                }

                deserializer.deserialize_identifier(Visitor)
            }
        }

        struct Visitor;

        impl<'de> crate::kube::apis::serde::de::Visitor<'de> for Visitor {
            type Value = HTTPBackendRef;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("HTTPBackendRef")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error> where A: crate::kube::apis::serde::de::MapAccess<'de> {
                let mut value_filters: Option<Vec<crate::kube::apis::networking::gateway::v1::HTTPRouteFilter>> = None;
                let mut value_group: Option<String> = None;
                let mut value_kind: Option<String> = None;
                let mut value_name: Option<String> = None;
                let mut value_namespace: Option<String> = None;
                let mut value_port: Option<i32> = None;
                let mut value_weight: Option<i32> = None;

                while let Some(key) = crate::kube::apis::serde::de::MapAccess::next_key::<Field>(&mut map)? {
                    match key {
                        Field::Key_filters => value_filters = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Key_group => value_group = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Key_kind => value_kind = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Key_name => value_name = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Key_namespace => value_namespace = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Key_port => value_port = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Key_weight => value_weight = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Other => { let _: crate::kube::apis::serde::de::IgnoredAny = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?; },
                    }
                }

                Ok(HTTPBackendRef {
                    filters: value_filters,
                    group: value_group,
                    kind: value_kind,
                    name: value_name.unwrap_or_default(),
                    namespace: value_namespace,
                    port: value_port,
                    weight: value_weight,
                })
            }
        }

        deserializer.deserialize_struct(
            "HTTPBackendRef",
            &[
                "filters",
                "group",
                "kind",
                "name",
                "namespace",
                "port",
                "weight",
            ],
            Visitor,
        )
    }
}

impl crate::kube::apis::serde::Serialize for HTTPBackendRef {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: crate::kube::apis::serde::Serializer {
        let mut state = serializer.serialize_struct(
            "HTTPBackendRef",
            1 +
            self.filters.as_ref().map_or(0, |_| 1) +
            self.group.as_ref().map_or(0, |_| 1) +
            self.kind.as_ref().map_or(0, |_| 1) +
            self.namespace.as_ref().map_or(0, |_| 1) +
            self.port.as_ref().map_or(0, |_| 1) +
            self.weight.as_ref().map_or(0, |_| 1),
        )?;
        if let Some(value) = &self.filters {
            crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "filters", value)?;
        }
        if let Some(value) = &self.group {
            crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "group", value)?;
        }
        if let Some(value) = &self.kind {
            crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "kind", value)?;
        }
        crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "name", &self.name)?;
        if let Some(value) = &self.namespace {
            crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "namespace", value)?;
        }
        if let Some(value) = &self.port {
            crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "port", value)?;
        }
        if let Some(value) = &self.weight {
            crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "weight", value)?;
        }
        crate::kube::apis::serde::ser::SerializeStruct::end(state)
    }
}

#[cfg(feature = "schemars")]
impl crate::kube::apis::schemars::JsonSchema for HTTPBackendRef {
    fn schema_name() -> String {
        "io.k8s.networking.gateway.v1.HTTPBackendRef".to_owned()
    }

    fn json_schema(__gen: &mut crate::kube::apis::schemars::gen::SchemaGenerator) -> crate::kube::apis::schemars::schema::Schema {
        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                description: Some("HTTPBackendRef defines how a HTTPRoute forwards a HTTP request. \n Note that when a namespace different than the local namespace is specified, a ReferenceGrant object is required in the referent namespace to allow that namespace's owner to accept the reference. See the ReferenceGrant documentation for details. \n <gateway:experimental:description> \n When the BackendRef points to a Kubernetes Service, implementations SHOULD honor the appProtocol field if it is set for the target Service Port. \n Implementations supporting appProtocol SHOULD recognize the Kubernetes Standard Application Protocols defined in KEP-3726. \n If a Service appProtocol isn't specified, an implementation MAY infer the backend protocol through its own means. Implementations MAY infer the protocol from the Route type referring to the backend Service. \n If a Route is not able to send traffic to the backend using the specified protocol then the backend is considered invalid. Implementations MUST set the \"ResolvedRefs\" condition to \"False\" with the \"UnsupportedProtocol\" reason. \n </gateway:experimental:description>".to_owned()),
                ..Default::default()
            })),
            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::Object))),
            object: Some(Box::new(crate::kube::apis::schemars::schema::ObjectValidation {
                properties: [
                    (
                        "filters".to_owned(),
                        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
                            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                                description: Some("Filters defined at this level should be executed if and only if the request is being forwarded to the backend defined here. \n Support: Implementation-specific (For broader support of filters, use the Filters field in HTTPRouteRule.)".to_owned()),
                                ..Default::default()
                            })),
                            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::Array))),
                            array: Some(Box::new(crate::kube::apis::schemars::schema::ArrayValidation {
                                items: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(__gen.subschema_for::<crate::kube::apis::networking::gateway::v1::HTTPRouteFilter>()))),
                                ..Default::default()
                            })),
                            ..Default::default()
                        }),
                    ),
                    (
                        "group".to_owned(),
                        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
                            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                                description: Some("Group is the group of the referent. For example, \"gateway.networking.k8s.io\". When unspecified or empty string, core API group is inferred.".to_owned()),
                                ..Default::default()
                            })),
                            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::String))),
                            ..Default::default()
                        }),
                    ),
                    (
                        "kind".to_owned(),
                        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
                            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                                description: Some("Kind is the Kubernetes resource kind of the referent. For example \"Service\". \n Defaults to \"Service\" when not specified. \n ExternalName services can refer to CNAME DNS records that may live outside of the cluster and as such are difficult to reason about in terms of conformance. They also may not be safe to forward to (see CVE-2021-25740 for more information). Implementations SHOULD NOT support ExternalName Services. \n Support: Core (Services with a type other than ExternalName) \n Support: Implementation-specific (Services with type ExternalName)".to_owned()),
                                ..Default::default()
                            })),
                            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::String))),
                            ..Default::default()
                        }),
                    ),
                    (
                        "name".to_owned(),
                        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
                            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                                description: Some("Name is the name of the referent.".to_owned()),
                                ..Default::default()
                            })),
                            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::String))),
                            ..Default::default()
                        }),
                    ),
                    (
                        "namespace".to_owned(),
                        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
                            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                                description: Some("Namespace is the namespace of the backend. When unspecified, the local namespace is inferred. \n Note that when a namespace different than the local namespace is specified, a ReferenceGrant object is required in the referent namespace to allow that namespace's owner to accept the reference. See the ReferenceGrant documentation for details. \n Support: Core".to_owned()),
                                ..Default::default()
                            })),
                            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::String))),
                            ..Default::default()
                        }),
                    ),
                    (
                        "port".to_owned(),
                        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
                            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                                description: Some("Port specifies the destination port number to use for this resource. Port is required when the referent is a Kubernetes Service. In this case, the port number is the service port number, not the target port. For other resources, destination port might be derived from the referent resource or this field.".to_owned()),
                                ..Default::default()
                            })),
                            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::Integer))),
                            format: Some("int32".to_owned()),
                            ..Default::default()
                        }),
                    ),
                    (
                        "weight".to_owned(),
                        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
                            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                                description: Some("Weight specifies the proportion of requests forwarded to the referenced backend. This is computed as weight/(sum of all weights in this BackendRefs list). For non-zero values, there may be some epsilon from the exact proportion defined here depending on the precision an implementation supports. Weight is not a percentage and the sum of weights does not need to equal 100. \n If only one backend is specified and it has a weight greater than 0, 100% of the traffic is forwarded to that backend. If weight is set to 0, no traffic should be forwarded for this entry. If unspecified, weight defaults to 1. \n Support for this field varies based on the context where used.".to_owned()),
                                ..Default::default()
                            })),
                            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::Integer))),
                            format: Some("int32".to_owned()),
                            ..Default::default()
                        }),
                    ),
                ].into(),
                required: [
                    "name".to_owned(),
                ].into(),
                ..Default::default()
            })),
            ..Default::default()
        })
    }
}
