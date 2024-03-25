// Generated from definition io.k8s.networking.gateway.v1.HTTPRouteRule

/// HTTPRouteRule defines semantics for matching an HTTP request based on conditions (matches), processing it (filters), and forwarding the request to an API object (backendRefs).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HTTPRouteRule {
    /// BackendRefs defines the backend(s) where matching requests should be sent. 
    ///  Failure behavior here depends on how many BackendRefs are specified and how many are invalid. 
    ///  If *all* entries in BackendRefs are invalid, and there are also no filters specified in this route rule, *all* traffic which matches this rule MUST receive a 500 status code. 
    ///  See the HTTPBackendRef definition for the rules about what makes a single HTTPBackendRef invalid. 
    ///  When a HTTPBackendRef is invalid, 500 status codes MUST be returned for requests that would have otherwise been routed to an invalid backend. If multiple backends are specified, and some are invalid, the proportion of requests that would otherwise have been routed to an invalid backend MUST receive a 500 status code. 
    ///  For example, if two backends are specified with equal weights, and one is invalid, 50 percent of traffic must receive a 500. Implementations may choose how that 50 percent is determined. 
    ///  Support: Core for Kubernetes Service 
    ///  Support: Extended for Kubernetes ServiceImport 
    ///  Support: Implementation-specific for any other resource 
    ///  Support for weight: Core
    pub backend_refs: Option<Vec<crate::kube::apis::networking::gateway::v1::HTTPBackendRef>>,

    /// Filters define the filters that are applied to requests that match this rule. 
    ///  The effects of ordering of multiple behaviors are currently unspecified. This can change in the future based on feedback during the alpha stage. 
    ///  Conformance-levels at this level are defined based on the type of filter: 
    ///  - ALL core filters MUST be supported by all implementations. - Implementers are encouraged to support extended filters. - Implementation-specific custom filters have no API guarantees across implementations. 
    ///  Specifying the same filter multiple times is not supported unless explicitly indicated in the filter. 
    ///  All filters are expected to be compatible with each other except for the URLRewrite and RequestRedirect filters, which may not be combined. If an implementation can not support other combinations of filters, they must clearly document that limitation. In cases where incompatible or unsupported filters are specified and cause the `Accepted` condition to be set to status `False`, implementations may use the `IncompatibleFilters` reason to specify this configuration error. 
    ///  Support: Core
    pub filters: Option<Vec<crate::kube::apis::networking::gateway::v1::HTTPRouteFilter>>,

    /// Matches define conditions used for matching the rule against incoming HTTP requests. Each match is independent, i.e. this rule will be matched if **any** one of the matches is satisfied. 
    ///  For example, take the following matches configuration: 
    ///  ``` matches: - path: value: "/foo" headers: - name: "version" value: "v2" - path: value: "/v2/foo" ``` 
    ///  For a request to match against this rule, a request must satisfy EITHER of the two conditions: 
    ///  - path prefixed with `/foo` AND contains the header `version: v2` - path prefix of `/v2/foo` 
    ///  See the documentation for HTTPRouteMatch on how to specify multiple match conditions that should be ANDed together. 
    ///  If no matches are specified, the default is a prefix path match on "/", which has the effect of matching every HTTP request. 
    ///  Proxy or Load Balancer routing configuration generated from HTTPRoutes MUST prioritize matches based on the following criteria, continuing on ties. Across all rules specified on applicable Routes, precedence must be given to the match having: 
    ///  * "Exact" path match. * "Prefix" path match with largest number of characters. * Method match. * Largest number of header matches. * Largest number of query param matches. 
    ///  Note: The precedence of RegularExpression path matches are implementation-specific. 
    ///  If ties still exist across multiple Routes, matching precedence MUST be determined in order of the following criteria, continuing on ties: 
    ///  * The oldest Route based on creation timestamp. * The Route appearing first in alphabetical order by "{namespace}/{name}". 
    ///  If ties still exist within an HTTPRoute, matching precedence MUST be granted to the FIRST matching rule (in list order) with a match meeting the above criteria. 
    ///  When no rules matching a request have been successfully attached to the parent a request is coming from, a HTTP 404 status code MUST be returned.
    pub matches: Option<Vec<crate::kube::apis::networking::gateway::v1::HTTPRouteMatch>>,
}

impl crate::kube::apis::DeepMerge for HTTPRouteRule {
    fn merge_from(&mut self, other: Self) {
        crate::kube::apis::merge_strategies::list::atomic(&mut self.backend_refs, other.backend_refs);
        crate::kube::apis::merge_strategies::list::atomic(&mut self.filters, other.filters);
        crate::kube::apis::merge_strategies::list::atomic(&mut self.matches, other.matches);
    }
}

impl<'de> crate::kube::apis::serde::Deserialize<'de> for HTTPRouteRule {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: crate::kube::apis::serde::Deserializer<'de> {
        #[allow(non_camel_case_types)]
        enum Field {
            Key_backend_refs,
            Key_filters,
            Key_matches,
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
                            "backendRefs" => Field::Key_backend_refs,
                            "filters" => Field::Key_filters,
                            "matches" => Field::Key_matches,
                            _ => Field::Other,
                        })
                    }
                }

                deserializer.deserialize_identifier(Visitor)
            }
        }

        struct Visitor;

        impl<'de> crate::kube::apis::serde::de::Visitor<'de> for Visitor {
            type Value = HTTPRouteRule;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("HTTPRouteRule")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error> where A: crate::kube::apis::serde::de::MapAccess<'de> {
                let mut value_backend_refs: Option<Vec<crate::kube::apis::networking::gateway::v1::HTTPBackendRef>> = None;
                let mut value_filters: Option<Vec<crate::kube::apis::networking::gateway::v1::HTTPRouteFilter>> = None;
                let mut value_matches: Option<Vec<crate::kube::apis::networking::gateway::v1::HTTPRouteMatch>> = None;

                while let Some(key) = crate::kube::apis::serde::de::MapAccess::next_key::<Field>(&mut map)? {
                    match key {
                        Field::Key_backend_refs => value_backend_refs = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Key_filters => value_filters = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Key_matches => value_matches = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Other => { let _: crate::kube::apis::serde::de::IgnoredAny = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?; },
                    }
                }

                Ok(HTTPRouteRule {
                    backend_refs: value_backend_refs,
                    filters: value_filters,
                    matches: value_matches,
                })
            }
        }

        deserializer.deserialize_struct(
            "HTTPRouteRule",
            &[
                "backendRefs",
                "filters",
                "matches",
            ],
            Visitor,
        )
    }
}

impl crate::kube::apis::serde::Serialize for HTTPRouteRule {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: crate::kube::apis::serde::Serializer {
        let mut state = serializer.serialize_struct(
            "HTTPRouteRule",
            self.backend_refs.as_ref().map_or(0, |_| 1) +
            self.filters.as_ref().map_or(0, |_| 1) +
            self.matches.as_ref().map_or(0, |_| 1),
        )?;
        if let Some(value) = &self.backend_refs {
            crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "backendRefs", value)?;
        }
        if let Some(value) = &self.filters {
            crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "filters", value)?;
        }
        if let Some(value) = &self.matches {
            crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "matches", value)?;
        }
        crate::kube::apis::serde::ser::SerializeStruct::end(state)
    }
}

#[cfg(feature = "schemars")]
impl crate::kube::apis::schemars::JsonSchema for HTTPRouteRule {
    fn schema_name() -> String {
        "io.k8s.networking.gateway.v1.HTTPRouteRule".to_owned()
    }

    fn json_schema(__gen: &mut crate::kube::apis::schemars::gen::SchemaGenerator) -> crate::kube::apis::schemars::schema::Schema {
        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                description: Some("HTTPRouteRule defines semantics for matching an HTTP request based on conditions (matches), processing it (filters), and forwarding the request to an API object (backendRefs).".to_owned()),
                ..Default::default()
            })),
            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::Object))),
            object: Some(Box::new(crate::kube::apis::schemars::schema::ObjectValidation {
                properties: [
                    (
                        "backendRefs".to_owned(),
                        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
                            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                                description: Some("BackendRefs defines the backend(s) where matching requests should be sent. \n Failure behavior here depends on how many BackendRefs are specified and how many are invalid. \n If *all* entries in BackendRefs are invalid, and there are also no filters specified in this route rule, *all* traffic which matches this rule MUST receive a 500 status code. \n See the HTTPBackendRef definition for the rules about what makes a single HTTPBackendRef invalid. \n When a HTTPBackendRef is invalid, 500 status codes MUST be returned for requests that would have otherwise been routed to an invalid backend. If multiple backends are specified, and some are invalid, the proportion of requests that would otherwise have been routed to an invalid backend MUST receive a 500 status code. \n For example, if two backends are specified with equal weights, and one is invalid, 50 percent of traffic must receive a 500. Implementations may choose how that 50 percent is determined. \n Support: Core for Kubernetes Service \n Support: Extended for Kubernetes ServiceImport \n Support: Implementation-specific for any other resource \n Support for weight: Core".to_owned()),
                                ..Default::default()
                            })),
                            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::Array))),
                            array: Some(Box::new(crate::kube::apis::schemars::schema::ArrayValidation {
                                items: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(__gen.subschema_for::<crate::kube::apis::networking::gateway::v1::HTTPBackendRef>()))),
                                ..Default::default()
                            })),
                            ..Default::default()
                        }),
                    ),
                    (
                        "filters".to_owned(),
                        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
                            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                                description: Some("Filters define the filters that are applied to requests that match this rule. \n The effects of ordering of multiple behaviors are currently unspecified. This can change in the future based on feedback during the alpha stage. \n Conformance-levels at this level are defined based on the type of filter: \n - ALL core filters MUST be supported by all implementations. - Implementers are encouraged to support extended filters. - Implementation-specific custom filters have no API guarantees across implementations. \n Specifying the same filter multiple times is not supported unless explicitly indicated in the filter. \n All filters are expected to be compatible with each other except for the URLRewrite and RequestRedirect filters, which may not be combined. If an implementation can not support other combinations of filters, they must clearly document that limitation. In cases where incompatible or unsupported filters are specified and cause the `Accepted` condition to be set to status `False`, implementations may use the `IncompatibleFilters` reason to specify this configuration error. \n Support: Core".to_owned()),
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
                        "matches".to_owned(),
                        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
                            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                                description: Some("Matches define conditions used for matching the rule against incoming HTTP requests. Each match is independent, i.e. this rule will be matched if **any** one of the matches is satisfied. \n For example, take the following matches configuration: \n ``` matches: - path: value: \"/foo\" headers: - name: \"version\" value: \"v2\" - path: value: \"/v2/foo\" ``` \n For a request to match against this rule, a request must satisfy EITHER of the two conditions: \n - path prefixed with `/foo` AND contains the header `version: v2` - path prefix of `/v2/foo` \n See the documentation for HTTPRouteMatch on how to specify multiple match conditions that should be ANDed together. \n If no matches are specified, the default is a prefix path match on \"/\", which has the effect of matching every HTTP request. \n Proxy or Load Balancer routing configuration generated from HTTPRoutes MUST prioritize matches based on the following criteria, continuing on ties. Across all rules specified on applicable Routes, precedence must be given to the match having: \n * \"Exact\" path match. * \"Prefix\" path match with largest number of characters. * Method match. * Largest number of header matches. * Largest number of query param matches. \n Note: The precedence of RegularExpression path matches are implementation-specific. \n If ties still exist across multiple Routes, matching precedence MUST be determined in order of the following criteria, continuing on ties: \n * The oldest Route based on creation timestamp. * The Route appearing first in alphabetical order by \"{namespace}/{name}\". \n If ties still exist within an HTTPRoute, matching precedence MUST be granted to the FIRST matching rule (in list order) with a match meeting the above criteria. \n When no rules matching a request have been successfully attached to the parent a request is coming from, a HTTP 404 status code MUST be returned.".to_owned()),
                                ..Default::default()
                            })),
                            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::Array))),
                            array: Some(Box::new(crate::kube::apis::schemars::schema::ArrayValidation {
                                items: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(__gen.subschema_for::<crate::kube::apis::networking::gateway::v1::HTTPRouteMatch>()))),
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
