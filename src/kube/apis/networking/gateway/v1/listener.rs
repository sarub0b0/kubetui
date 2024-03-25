// Generated from definition io.k8s.networking.gateway.v1.Listener

/// Listener embodies the concept of a logical endpoint where a Gateway accepts network connections.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Listener {
    /// AllowedRoutes defines the types of routes that MAY be attached to a Listener and the trusted namespaces where those Route resources MAY be present. 
    ///  Although a client request may match multiple route rules, only one rule may ultimately receive the request. Matching precedence MUST be determined in order of the following criteria: 
    ///  * The most specific match as defined by the Route type. * The oldest Route based on creation timestamp. For example, a Route with a creation timestamp of "2020-09-08 01:02:03" is given precedence over a Route with a creation timestamp of "2020-09-08 01:02:04". * If everything else is equivalent, the Route appearing first in alphabetical order (namespace/name) should be given precedence. For example, foo/bar is given precedence over foo/baz. 
    ///  All valid rules within a Route attached to this Listener should be implemented. Invalid Route rules can be ignored (sometimes that will mean the full Route). If a Route rule transitions from valid to invalid, support for that Route rule should be dropped to ensure consistency. For example, even if a filter specified by a Route rule is invalid, the rest of the rules within that Route should still be supported. 
    ///  Support: Core
    pub allowed_routes: Option<crate::kube::apis::networking::gateway::v1::AllowedRoutes>,

    /// Hostname specifies the virtual hostname to match for protocol types that define this concept. When unspecified, all hostnames are matched. This field is ignored for protocols that don't require hostname based matching. 
    ///  Implementations MUST apply Hostname matching appropriately for each of the following protocols: 
    ///  * TLS: The Listener Hostname MUST match the SNI. * HTTP: The Listener Hostname MUST match the Host header of the request. * HTTPS: The Listener Hostname SHOULD match at both the TLS and HTTP protocol layers as described above. If an implementation does not ensure that both the SNI and Host header match the Listener hostname, it MUST clearly document that. 
    ///  For HTTPRoute and TLSRoute resources, there is an interaction with the `spec.hostnames` array. When both listener and route specify hostnames, there MUST be an intersection between the values for a Route to be accepted. For more information, refer to the Route specific Hostnames documentation. 
    ///  Hostnames that are prefixed with a wildcard label (`*.`) are interpreted as a suffix match. That means that a match for `*.example.com` would match both `test.example.com`, and `foo.test.example.com`, but not `example.com`. 
    ///  Support: Core
    pub hostname: Option<String>,

    /// Name is the name of the Listener. This name MUST be unique within a Gateway. 
    ///  Support: Core
    pub name: String,

    /// Port is the network port. Multiple listeners may use the same port, subject to the Listener compatibility rules. 
    ///  Support: Core
    pub port: i32,

    /// Protocol specifies the network protocol this listener expects to receive. 
    ///  Support: Core
    pub protocol: String,

    pub tls: Option<crate::kube::apis::networking::gateway::v1::GatewayTLSConfig>,
}

impl crate::kube::apis::DeepMerge for Listener {
    fn merge_from(&mut self, other: Self) {
        crate::kube::apis::DeepMerge::merge_from(&mut self.allowed_routes, other.allowed_routes);
        crate::kube::apis::DeepMerge::merge_from(&mut self.hostname, other.hostname);
        crate::kube::apis::DeepMerge::merge_from(&mut self.name, other.name);
        crate::kube::apis::DeepMerge::merge_from(&mut self.port, other.port);
        crate::kube::apis::DeepMerge::merge_from(&mut self.protocol, other.protocol);
        crate::kube::apis::DeepMerge::merge_from(&mut self.tls, other.tls);
    }
}

impl<'de> crate::kube::apis::serde::Deserialize<'de> for Listener {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: crate::kube::apis::serde::Deserializer<'de> {
        #[allow(non_camel_case_types)]
        enum Field {
            Key_allowed_routes,
            Key_hostname,
            Key_name,
            Key_port,
            Key_protocol,
            Key_tls,
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
                            "allowedRoutes" => Field::Key_allowed_routes,
                            "hostname" => Field::Key_hostname,
                            "name" => Field::Key_name,
                            "port" => Field::Key_port,
                            "protocol" => Field::Key_protocol,
                            "tls" => Field::Key_tls,
                            _ => Field::Other,
                        })
                    }
                }

                deserializer.deserialize_identifier(Visitor)
            }
        }

        struct Visitor;

        impl<'de> crate::kube::apis::serde::de::Visitor<'de> for Visitor {
            type Value = Listener;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("Listener")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error> where A: crate::kube::apis::serde::de::MapAccess<'de> {
                let mut value_allowed_routes: Option<crate::kube::apis::networking::gateway::v1::AllowedRoutes> = None;
                let mut value_hostname: Option<String> = None;
                let mut value_name: Option<String> = None;
                let mut value_port: Option<i32> = None;
                let mut value_protocol: Option<String> = None;
                let mut value_tls: Option<crate::kube::apis::networking::gateway::v1::GatewayTLSConfig> = None;

                while let Some(key) = crate::kube::apis::serde::de::MapAccess::next_key::<Field>(&mut map)? {
                    match key {
                        Field::Key_allowed_routes => value_allowed_routes = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Key_hostname => value_hostname = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Key_name => value_name = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Key_port => value_port = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Key_protocol => value_protocol = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Key_tls => value_tls = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Other => { let _: crate::kube::apis::serde::de::IgnoredAny = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?; },
                    }
                }

                Ok(Listener {
                    allowed_routes: value_allowed_routes,
                    hostname: value_hostname,
                    name: value_name.unwrap_or_default(),
                    port: value_port.unwrap_or_default(),
                    protocol: value_protocol.unwrap_or_default(),
                    tls: value_tls,
                })
            }
        }

        deserializer.deserialize_struct(
            "Listener",
            &[
                "allowedRoutes",
                "hostname",
                "name",
                "port",
                "protocol",
                "tls",
            ],
            Visitor,
        )
    }
}

impl crate::kube::apis::serde::Serialize for Listener {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: crate::kube::apis::serde::Serializer {
        let mut state = serializer.serialize_struct(
            "Listener",
            3 +
            self.allowed_routes.as_ref().map_or(0, |_| 1) +
            self.hostname.as_ref().map_or(0, |_| 1) +
            self.tls.as_ref().map_or(0, |_| 1),
        )?;
        if let Some(value) = &self.allowed_routes {
            crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "allowedRoutes", value)?;
        }
        if let Some(value) = &self.hostname {
            crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "hostname", value)?;
        }
        crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "name", &self.name)?;
        crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "port", &self.port)?;
        crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "protocol", &self.protocol)?;
        if let Some(value) = &self.tls {
            crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "tls", value)?;
        }
        crate::kube::apis::serde::ser::SerializeStruct::end(state)
    }
}

#[cfg(feature = "schemars")]
impl crate::kube::apis::schemars::JsonSchema for Listener {
    fn schema_name() -> String {
        "io.k8s.networking.gateway.v1.Listener".to_owned()
    }

    fn json_schema(__gen: &mut crate::kube::apis::schemars::gen::SchemaGenerator) -> crate::kube::apis::schemars::schema::Schema {
        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                description: Some("Listener embodies the concept of a logical endpoint where a Gateway accepts network connections.".to_owned()),
                ..Default::default()
            })),
            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::Object))),
            object: Some(Box::new(crate::kube::apis::schemars::schema::ObjectValidation {
                properties: [
                    (
                        "allowedRoutes".to_owned(),
                        {
                            let mut schema_obj = __gen.subschema_for::<crate::kube::apis::networking::gateway::v1::AllowedRoutes>().into_object();
                            schema_obj.metadata = Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                                description: Some("AllowedRoutes defines the types of routes that MAY be attached to a Listener and the trusted namespaces where those Route resources MAY be present. \n Although a client request may match multiple route rules, only one rule may ultimately receive the request. Matching precedence MUST be determined in order of the following criteria: \n * The most specific match as defined by the Route type. * The oldest Route based on creation timestamp. For example, a Route with a creation timestamp of \"2020-09-08 01:02:03\" is given precedence over a Route with a creation timestamp of \"2020-09-08 01:02:04\". * If everything else is equivalent, the Route appearing first in alphabetical order (namespace/name) should be given precedence. For example, foo/bar is given precedence over foo/baz. \n All valid rules within a Route attached to this Listener should be implemented. Invalid Route rules can be ignored (sometimes that will mean the full Route). If a Route rule transitions from valid to invalid, support for that Route rule should be dropped to ensure consistency. For example, even if a filter specified by a Route rule is invalid, the rest of the rules within that Route should still be supported. \n Support: Core".to_owned()),
                                ..Default::default()
                            }));
                            crate::kube::apis::schemars::schema::Schema::Object(schema_obj)
                        },
                    ),
                    (
                        "hostname".to_owned(),
                        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
                            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                                description: Some("Hostname specifies the virtual hostname to match for protocol types that define this concept. When unspecified, all hostnames are matched. This field is ignored for protocols that don't require hostname based matching. \n Implementations MUST apply Hostname matching appropriately for each of the following protocols: \n * TLS: The Listener Hostname MUST match the SNI. * HTTP: The Listener Hostname MUST match the Host header of the request. * HTTPS: The Listener Hostname SHOULD match at both the TLS and HTTP protocol layers as described above. If an implementation does not ensure that both the SNI and Host header match the Listener hostname, it MUST clearly document that. \n For HTTPRoute and TLSRoute resources, there is an interaction with the `spec.hostnames` array. When both listener and route specify hostnames, there MUST be an intersection between the values for a Route to be accepted. For more information, refer to the Route specific Hostnames documentation. \n Hostnames that are prefixed with a wildcard label (`*.`) are interpreted as a suffix match. That means that a match for `*.example.com` would match both `test.example.com`, and `foo.test.example.com`, but not `example.com`. \n Support: Core".to_owned()),
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
                                description: Some("Name is the name of the Listener. This name MUST be unique within a Gateway. \n Support: Core".to_owned()),
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
                                description: Some("Port is the network port. Multiple listeners may use the same port, subject to the Listener compatibility rules. \n Support: Core".to_owned()),
                                ..Default::default()
                            })),
                            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::Integer))),
                            format: Some("int32".to_owned()),
                            ..Default::default()
                        }),
                    ),
                    (
                        "protocol".to_owned(),
                        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
                            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                                description: Some("Protocol specifies the network protocol this listener expects to receive. \n Support: Core".to_owned()),
                                ..Default::default()
                            })),
                            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::String))),
                            ..Default::default()
                        }),
                    ),
                    (
                        "tls".to_owned(),
                        __gen.subschema_for::<crate::kube::apis::networking::gateway::v1::GatewayTLSConfig>(),
                    ),
                ].into(),
                required: [
                    "name".to_owned(),
                    "port".to_owned(),
                    "protocol".to_owned(),
                ].into(),
                ..Default::default()
            })),
            ..Default::default()
        })
    }
}
