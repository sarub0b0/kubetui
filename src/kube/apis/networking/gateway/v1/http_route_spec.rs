// Generated from definition io.k8s.networking.gateway.v1.HTTPRouteSpec

/// Spec defines the desired state of HTTPRoute.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HTTPRouteSpec {
    /// Hostnames defines a set of hostnames that should match against the HTTP Host header to select a HTTPRoute used to process the request. Implementations MUST ignore any port value specified in the HTTP Host header while performing a match and (absent of any applicable header modification configuration) MUST forward this header unmodified to the backend. 
    ///  Valid values for Hostnames are determined by RFC 1123 definition of a hostname with 2 notable exceptions: 
    ///  1. IPs are not allowed. 2. A hostname may be prefixed with a wildcard label (`*.`). The wildcard label must appear by itself as the first label. 
    ///  If a hostname is specified by both the Listener and HTTPRoute, there must be at least one intersecting hostname for the HTTPRoute to be attached to the Listener. For example: 
    ///  * A Listener with `test.example.com` as the hostname matches HTTPRoutes that have either not specified any hostnames, or have specified at least one of `test.example.com` or `*.example.com`. * A Listener with `*.example.com` as the hostname matches HTTPRoutes that have either not specified any hostnames or have specified at least one hostname that matches the Listener hostname. For example, `*.example.com`, `test.example.com`, and `foo.test.example.com` would all match. On the other hand, `example.com` and `test.example.net` would not match. 
    ///  Hostnames that are prefixed with a wildcard label (`*.`) are interpreted as a suffix match. That means that a match for `*.example.com` would match both `test.example.com`, and `foo.test.example.com`, but not `example.com`. 
    ///  If both the Listener and HTTPRoute have specified hostnames, any HTTPRoute hostnames that do not match the Listener hostname MUST be ignored. For example, if a Listener specified `*.example.com`, and the HTTPRoute specified `test.example.com` and `test.example.net`, `test.example.net` must not be considered for a match. 
    ///  If both the Listener and HTTPRoute have specified hostnames, and none match with the criteria above, then the HTTPRoute is not accepted. The implementation must raise an 'Accepted' Condition with a status of `False` in the corresponding RouteParentStatus. 
    ///  In the event that multiple HTTPRoutes specify intersecting hostnames (e.g. overlapping wildcard matching and exact matching hostnames), precedence must be given to rules from the HTTPRoute with the largest number of: 
    ///  * Characters in a matching non-wildcard hostname. * Characters in a matching hostname. 
    ///  If ties exist across multiple Routes, the matching precedence rules for HTTPRouteMatches takes over. 
    ///  Support: Core
    pub hostnames: Option<Vec<String>>,

    /// ParentRefs references the resources (usually Gateways) that a Route wants to be attached to. Note that the referenced parent resource needs to allow this for the attachment to be complete. For Gateways, that means the Gateway needs to allow attachment from Routes of this kind and namespace. For Services, that means the Service must either be in the same namespace for a "producer" route, or the mesh implementation must support and allow "consumer" routes for the referenced Service. ReferenceGrant is not applicable for governing ParentRefs to Services - it is not possible to create a "producer" route for a Service in a different namespace from the Route. 
    ///  There are two kinds of parent resources with "Core" support: 
    ///  * Gateway (Gateway conformance profile)  This API may be extended in the future to support additional kinds of parent resources. 
    ///  ParentRefs must be _distinct_. This means either that: 
    ///  * They select different objects.  If this is the case, then parentRef entries are distinct. In terms of fields, this means that the multi-part key defined by `group`, `kind`, `namespace`, and `name` must be unique across all parentRef entries in the Route. * They do not select different objects, but for each optional field used, each ParentRef that selects the same object must set the same set of optional fields to different values. If one ParentRef sets a combination of optional fields, all must set the same combination. 
    ///  Some examples: 
    ///  * If one ParentRef sets `sectionName`, all ParentRefs referencing the same object must also set `sectionName`. * If one ParentRef sets `port`, all ParentRefs referencing the same object must also set `port`. * If one ParentRef sets `sectionName` and `port`, all ParentRefs referencing the same object must also set `sectionName` and `port`. 
    ///  It is possible to separately reference multiple distinct objects that may be collapsed by an implementation. For example, some implementations may choose to merge compatible Gateway Listeners together. If that is the case, the list of routes attached to those resources should also be merged. 
    ///  Note that for ParentRefs that cross namespace boundaries, there are specific rules. Cross-namespace references are only valid if they are explicitly allowed by something in the namespace they are referring to. For example, Gateway has the AllowedRoutes field, and ReferenceGrant provides a generic way to enable other kinds of cross-namespace reference. 
    ///   
    ///  
    pub parent_refs: Option<Vec<crate::kube::apis::networking::gateway::v1::ParentReference>>,

    /// Rules are a list of HTTP matchers, filters and actions.
    pub rules: Option<Vec<crate::kube::apis::networking::gateway::v1::HTTPRouteRule>>,
}

impl crate::kube::apis::DeepMerge for HTTPRouteSpec {
    fn merge_from(&mut self, other: Self) {
        crate::kube::apis::merge_strategies::list::atomic(&mut self.hostnames, other.hostnames);
        crate::kube::apis::merge_strategies::list::atomic(&mut self.parent_refs, other.parent_refs);
        crate::kube::apis::merge_strategies::list::atomic(&mut self.rules, other.rules);
    }
}

impl<'de> crate::kube::apis::serde::Deserialize<'de> for HTTPRouteSpec {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: crate::kube::apis::serde::Deserializer<'de> {
        #[allow(non_camel_case_types)]
        enum Field {
            Key_hostnames,
            Key_parent_refs,
            Key_rules,
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
                            "hostnames" => Field::Key_hostnames,
                            "parentRefs" => Field::Key_parent_refs,
                            "rules" => Field::Key_rules,
                            _ => Field::Other,
                        })
                    }
                }

                deserializer.deserialize_identifier(Visitor)
            }
        }

        struct Visitor;

        impl<'de> crate::kube::apis::serde::de::Visitor<'de> for Visitor {
            type Value = HTTPRouteSpec;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("HTTPRouteSpec")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error> where A: crate::kube::apis::serde::de::MapAccess<'de> {
                let mut value_hostnames: Option<Vec<String>> = None;
                let mut value_parent_refs: Option<Vec<crate::kube::apis::networking::gateway::v1::ParentReference>> = None;
                let mut value_rules: Option<Vec<crate::kube::apis::networking::gateway::v1::HTTPRouteRule>> = None;

                while let Some(key) = crate::kube::apis::serde::de::MapAccess::next_key::<Field>(&mut map)? {
                    match key {
                        Field::Key_hostnames => value_hostnames = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Key_parent_refs => value_parent_refs = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Key_rules => value_rules = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Other => { let _: crate::kube::apis::serde::de::IgnoredAny = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?; },
                    }
                }

                Ok(HTTPRouteSpec {
                    hostnames: value_hostnames,
                    parent_refs: value_parent_refs,
                    rules: value_rules,
                })
            }
        }

        deserializer.deserialize_struct(
            "HTTPRouteSpec",
            &[
                "hostnames",
                "parentRefs",
                "rules",
            ],
            Visitor,
        )
    }
}

impl crate::kube::apis::serde::Serialize for HTTPRouteSpec {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: crate::kube::apis::serde::Serializer {
        let mut state = serializer.serialize_struct(
            "HTTPRouteSpec",
            self.hostnames.as_ref().map_or(0, |_| 1) +
            self.parent_refs.as_ref().map_or(0, |_| 1) +
            self.rules.as_ref().map_or(0, |_| 1),
        )?;
        if let Some(value) = &self.hostnames {
            crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "hostnames", value)?;
        }
        if let Some(value) = &self.parent_refs {
            crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "parentRefs", value)?;
        }
        if let Some(value) = &self.rules {
            crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "rules", value)?;
        }
        crate::kube::apis::serde::ser::SerializeStruct::end(state)
    }
}

#[cfg(feature = "schemars")]
impl crate::kube::apis::schemars::JsonSchema for HTTPRouteSpec {
    fn schema_name() -> String {
        "io.k8s.networking.gateway.v1.HTTPRouteSpec".to_owned()
    }

    fn json_schema(__gen: &mut crate::kube::apis::schemars::gen::SchemaGenerator) -> crate::kube::apis::schemars::schema::Schema {
        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                description: Some("Spec defines the desired state of HTTPRoute.".to_owned()),
                ..Default::default()
            })),
            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::Object))),
            object: Some(Box::new(crate::kube::apis::schemars::schema::ObjectValidation {
                properties: [
                    (
                        "hostnames".to_owned(),
                        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
                            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                                description: Some("Hostnames defines a set of hostnames that should match against the HTTP Host header to select a HTTPRoute used to process the request. Implementations MUST ignore any port value specified in the HTTP Host header while performing a match and (absent of any applicable header modification configuration) MUST forward this header unmodified to the backend. \n Valid values for Hostnames are determined by RFC 1123 definition of a hostname with 2 notable exceptions: \n 1. IPs are not allowed. 2. A hostname may be prefixed with a wildcard label (`*.`). The wildcard label must appear by itself as the first label. \n If a hostname is specified by both the Listener and HTTPRoute, there must be at least one intersecting hostname for the HTTPRoute to be attached to the Listener. For example: \n * A Listener with `test.example.com` as the hostname matches HTTPRoutes that have either not specified any hostnames, or have specified at least one of `test.example.com` or `*.example.com`. * A Listener with `*.example.com` as the hostname matches HTTPRoutes that have either not specified any hostnames or have specified at least one hostname that matches the Listener hostname. For example, `*.example.com`, `test.example.com`, and `foo.test.example.com` would all match. On the other hand, `example.com` and `test.example.net` would not match. \n Hostnames that are prefixed with a wildcard label (`*.`) are interpreted as a suffix match. That means that a match for `*.example.com` would match both `test.example.com`, and `foo.test.example.com`, but not `example.com`. \n If both the Listener and HTTPRoute have specified hostnames, any HTTPRoute hostnames that do not match the Listener hostname MUST be ignored. For example, if a Listener specified `*.example.com`, and the HTTPRoute specified `test.example.com` and `test.example.net`, `test.example.net` must not be considered for a match. \n If both the Listener and HTTPRoute have specified hostnames, and none match with the criteria above, then the HTTPRoute is not accepted. The implementation must raise an 'Accepted' Condition with a status of `False` in the corresponding RouteParentStatus. \n In the event that multiple HTTPRoutes specify intersecting hostnames (e.g. overlapping wildcard matching and exact matching hostnames), precedence must be given to rules from the HTTPRoute with the largest number of: \n * Characters in a matching non-wildcard hostname. * Characters in a matching hostname. \n If ties exist across multiple Routes, the matching precedence rules for HTTPRouteMatches takes over. \n Support: Core".to_owned()),
                                ..Default::default()
                            })),
                            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::Array))),
                            array: Some(Box::new(crate::kube::apis::schemars::schema::ArrayValidation {
                                items: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(
                                    crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
                                        metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                                            description: Some("Hostname is the fully qualified domain name of a network host. This matches the RFC 1123 definition of a hostname with 2 notable exceptions: \n 1. IPs are not allowed. 2. A hostname may be prefixed with a wildcard label (`*.`). The wildcard label must appear by itself as the first label. \n Hostname can be \"precise\" which is a domain name without the terminating dot of a network host (e.g. \"foo.example.com\") or \"wildcard\", which is a domain name prefixed with a single wildcard label (e.g. `*.example.com`). \n Note that as per RFC1035 and RFC1123, a *label* must consist of lower case alphanumeric characters or '-', and must start and end with an alphanumeric character. No other punctuation is allowed.".to_owned()),
                                            ..Default::default()
                                        })),
                                        instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::String))),
                                        ..Default::default()
                                    })
                                ))),
                                ..Default::default()
                            })),
                            ..Default::default()
                        }),
                    ),
                    (
                        "parentRefs".to_owned(),
                        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
                            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                                description: Some("ParentRefs references the resources (usually Gateways) that a Route wants to be attached to. Note that the referenced parent resource needs to allow this for the attachment to be complete. For Gateways, that means the Gateway needs to allow attachment from Routes of this kind and namespace. For Services, that means the Service must either be in the same namespace for a \"producer\" route, or the mesh implementation must support and allow \"consumer\" routes for the referenced Service. ReferenceGrant is not applicable for governing ParentRefs to Services - it is not possible to create a \"producer\" route for a Service in a different namespace from the Route. \n There are two kinds of parent resources with \"Core\" support: \n * Gateway (Gateway conformance profile)  This API may be extended in the future to support additional kinds of parent resources. \n ParentRefs must be _distinct_. This means either that: \n * They select different objects.  If this is the case, then parentRef entries are distinct. In terms of fields, this means that the multi-part key defined by `group`, `kind`, `namespace`, and `name` must be unique across all parentRef entries in the Route. * They do not select different objects, but for each optional field used, each ParentRef that selects the same object must set the same set of optional fields to different values. If one ParentRef sets a combination of optional fields, all must set the same combination. \n Some examples: \n * If one ParentRef sets `sectionName`, all ParentRefs referencing the same object must also set `sectionName`. * If one ParentRef sets `port`, all ParentRefs referencing the same object must also set `port`. * If one ParentRef sets `sectionName` and `port`, all ParentRefs referencing the same object must also set `sectionName` and `port`. \n It is possible to separately reference multiple distinct objects that may be collapsed by an implementation. For example, some implementations may choose to merge compatible Gateway Listeners together. If that is the case, the list of routes attached to those resources should also be merged. \n Note that for ParentRefs that cross namespace boundaries, there are specific rules. Cross-namespace references are only valid if they are explicitly allowed by something in the namespace they are referring to. For example, Gateway has the AllowedRoutes field, and ReferenceGrant provides a generic way to enable other kinds of cross-namespace reference. \n  \n ".to_owned()),
                                ..Default::default()
                            })),
                            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::Array))),
                            array: Some(Box::new(crate::kube::apis::schemars::schema::ArrayValidation {
                                items: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(__gen.subschema_for::<crate::kube::apis::networking::gateway::v1::ParentReference>()))),
                                ..Default::default()
                            })),
                            ..Default::default()
                        }),
                    ),
                    (
                        "rules".to_owned(),
                        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
                            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                                description: Some("Rules are a list of HTTP matchers, filters and actions.".to_owned()),
                                ..Default::default()
                            })),
                            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::Array))),
                            array: Some(Box::new(crate::kube::apis::schemars::schema::ArrayValidation {
                                items: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(__gen.subschema_for::<crate::kube::apis::networking::gateway::v1::HTTPRouteRule>()))),
                                ..Default::default()
                            })),
                            ..Default::default()
                        }),
                    ),
                ].into(),
                ..Default::default()
            })),
            ..Default::default()
        })
    }
}
