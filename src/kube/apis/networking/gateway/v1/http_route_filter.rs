// Generated from definition io.k8s.networking.gateway.v1.HTTPRouteFilter

/// HTTPRouteFilter defines processing steps that must be completed during the request or response lifecycle. HTTPRouteFilters are meant as an extension point to express processing that may be done in Gateway implementations. Some examples include request or response modification, implementing authentication strategies, rate-limiting, and traffic shaping. API guarantee/conformance is defined based on the type of the filter.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HTTPRouteFilter {
    pub extension_ref: Option<crate::kube::apis::networking::gateway::v1::LocalObjectReference>,

    pub request_header_modifier: Option<crate::kube::apis::networking::gateway::v1::HTTPHeaderFilter>,

    pub request_mirror: Option<crate::kube::apis::networking::gateway::v1::HTTPRequestMirrorFilter>,

    pub request_redirect: Option<crate::kube::apis::networking::gateway::v1::HTTPRequestRedirectFilter>,

    pub response_header_modifier: Option<crate::kube::apis::networking::gateway::v1::HTTPHeaderFilter>,

    /// Type identifies the type of filter to apply. As with other API fields, types are classified into three conformance levels: 
    ///  - Core: Filter types and their corresponding configuration defined by "Support: Core" in this package, e.g. "RequestHeaderModifier". All implementations must support core filters. 
    ///  - Extended: Filter types and their corresponding configuration defined by "Support: Extended" in this package, e.g. "RequestMirror". Implementers are encouraged to support extended filters. 
    ///  - Implementation-specific: Filters that are defined and supported by specific vendors. In the future, filters showing convergence in behavior across multiple implementations will be considered for inclusion in extended or core conformance levels. Filter-specific configuration for such filters is specified using the ExtensionRef field. `Type` should be set to "ExtensionRef" for custom filters. 
    ///  Implementers are encouraged to define custom implementation types to extend the core API with implementation-specific behavior. 
    ///  If a reference to a custom filter type cannot be resolved, the filter MUST NOT be skipped. Instead, requests that would have been processed by that filter MUST receive a HTTP error response. 
    ///  Note that values may be added to this enum, implementations must ensure that unknown values will not cause a crash. 
    ///  Unknown values here must result in the implementation setting the Accepted Condition for the Route to `status: False`, with a Reason of `UnsupportedValue`.
    pub type_: String,

    pub url_rewrite: Option<crate::kube::apis::networking::gateway::v1::HTTPURLRewriteFilter>,
}

impl crate::kube::apis::DeepMerge for HTTPRouteFilter {
    fn merge_from(&mut self, other: Self) {
        crate::kube::apis::DeepMerge::merge_from(&mut self.extension_ref, other.extension_ref);
        crate::kube::apis::DeepMerge::merge_from(&mut self.request_header_modifier, other.request_header_modifier);
        crate::kube::apis::DeepMerge::merge_from(&mut self.request_mirror, other.request_mirror);
        crate::kube::apis::DeepMerge::merge_from(&mut self.request_redirect, other.request_redirect);
        crate::kube::apis::DeepMerge::merge_from(&mut self.response_header_modifier, other.response_header_modifier);
        crate::kube::apis::DeepMerge::merge_from(&mut self.type_, other.type_);
        crate::kube::apis::DeepMerge::merge_from(&mut self.url_rewrite, other.url_rewrite);
    }
}

impl<'de> crate::kube::apis::serde::Deserialize<'de> for HTTPRouteFilter {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: crate::kube::apis::serde::Deserializer<'de> {
        #[allow(non_camel_case_types)]
        enum Field {
            Key_extension_ref,
            Key_request_header_modifier,
            Key_request_mirror,
            Key_request_redirect,
            Key_response_header_modifier,
            Key_type_,
            Key_url_rewrite,
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
                            "extensionRef" => Field::Key_extension_ref,
                            "requestHeaderModifier" => Field::Key_request_header_modifier,
                            "requestMirror" => Field::Key_request_mirror,
                            "requestRedirect" => Field::Key_request_redirect,
                            "responseHeaderModifier" => Field::Key_response_header_modifier,
                            "type" => Field::Key_type_,
                            "urlRewrite" => Field::Key_url_rewrite,
                            _ => Field::Other,
                        })
                    }
                }

                deserializer.deserialize_identifier(Visitor)
            }
        }

        struct Visitor;

        impl<'de> crate::kube::apis::serde::de::Visitor<'de> for Visitor {
            type Value = HTTPRouteFilter;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("HTTPRouteFilter")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error> where A: crate::kube::apis::serde::de::MapAccess<'de> {
                let mut value_extension_ref: Option<crate::kube::apis::networking::gateway::v1::LocalObjectReference> = None;
                let mut value_request_header_modifier: Option<crate::kube::apis::networking::gateway::v1::HTTPHeaderFilter> = None;
                let mut value_request_mirror: Option<crate::kube::apis::networking::gateway::v1::HTTPRequestMirrorFilter> = None;
                let mut value_request_redirect: Option<crate::kube::apis::networking::gateway::v1::HTTPRequestRedirectFilter> = None;
                let mut value_response_header_modifier: Option<crate::kube::apis::networking::gateway::v1::HTTPHeaderFilter> = None;
                let mut value_type_: Option<String> = None;
                let mut value_url_rewrite: Option<crate::kube::apis::networking::gateway::v1::HTTPURLRewriteFilter> = None;

                while let Some(key) = crate::kube::apis::serde::de::MapAccess::next_key::<Field>(&mut map)? {
                    match key {
                        Field::Key_extension_ref => value_extension_ref = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Key_request_header_modifier => value_request_header_modifier = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Key_request_mirror => value_request_mirror = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Key_request_redirect => value_request_redirect = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Key_response_header_modifier => value_response_header_modifier = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Key_type_ => value_type_ = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Key_url_rewrite => value_url_rewrite = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Other => { let _: crate::kube::apis::serde::de::IgnoredAny = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?; },
                    }
                }

                Ok(HTTPRouteFilter {
                    extension_ref: value_extension_ref,
                    request_header_modifier: value_request_header_modifier,
                    request_mirror: value_request_mirror,
                    request_redirect: value_request_redirect,
                    response_header_modifier: value_response_header_modifier,
                    type_: value_type_.unwrap_or_default(),
                    url_rewrite: value_url_rewrite,
                })
            }
        }

        deserializer.deserialize_struct(
            "HTTPRouteFilter",
            &[
                "extensionRef",
                "requestHeaderModifier",
                "requestMirror",
                "requestRedirect",
                "responseHeaderModifier",
                "type",
                "urlRewrite",
            ],
            Visitor,
        )
    }
}

impl crate::kube::apis::serde::Serialize for HTTPRouteFilter {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: crate::kube::apis::serde::Serializer {
        let mut state = serializer.serialize_struct(
            "HTTPRouteFilter",
            1 +
            self.extension_ref.as_ref().map_or(0, |_| 1) +
            self.request_header_modifier.as_ref().map_or(0, |_| 1) +
            self.request_mirror.as_ref().map_or(0, |_| 1) +
            self.request_redirect.as_ref().map_or(0, |_| 1) +
            self.response_header_modifier.as_ref().map_or(0, |_| 1) +
            self.url_rewrite.as_ref().map_or(0, |_| 1),
        )?;
        if let Some(value) = &self.extension_ref {
            crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "extensionRef", value)?;
        }
        if let Some(value) = &self.request_header_modifier {
            crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "requestHeaderModifier", value)?;
        }
        if let Some(value) = &self.request_mirror {
            crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "requestMirror", value)?;
        }
        if let Some(value) = &self.request_redirect {
            crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "requestRedirect", value)?;
        }
        if let Some(value) = &self.response_header_modifier {
            crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "responseHeaderModifier", value)?;
        }
        crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "type", &self.type_)?;
        if let Some(value) = &self.url_rewrite {
            crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "urlRewrite", value)?;
        }
        crate::kube::apis::serde::ser::SerializeStruct::end(state)
    }
}

#[cfg(feature = "schemars")]
impl crate::kube::apis::schemars::JsonSchema for HTTPRouteFilter {
    fn schema_name() -> String {
        "io.k8s.networking.gateway.v1.HTTPRouteFilter".to_owned()
    }

    fn json_schema(__gen: &mut crate::kube::apis::schemars::gen::SchemaGenerator) -> crate::kube::apis::schemars::schema::Schema {
        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                description: Some("HTTPRouteFilter defines processing steps that must be completed during the request or response lifecycle. HTTPRouteFilters are meant as an extension point to express processing that may be done in Gateway implementations. Some examples include request or response modification, implementing authentication strategies, rate-limiting, and traffic shaping. API guarantee/conformance is defined based on the type of the filter.".to_owned()),
                ..Default::default()
            })),
            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::Object))),
            object: Some(Box::new(crate::kube::apis::schemars::schema::ObjectValidation {
                properties: [
                    (
                        "extensionRef".to_owned(),
                        __gen.subschema_for::<crate::kube::apis::networking::gateway::v1::LocalObjectReference>(),
                    ),
                    (
                        "requestHeaderModifier".to_owned(),
                        __gen.subschema_for::<crate::kube::apis::networking::gateway::v1::HTTPHeaderFilter>(),
                    ),
                    (
                        "requestMirror".to_owned(),
                        __gen.subschema_for::<crate::kube::apis::networking::gateway::v1::HTTPRequestMirrorFilter>(),
                    ),
                    (
                        "requestRedirect".to_owned(),
                        __gen.subschema_for::<crate::kube::apis::networking::gateway::v1::HTTPRequestRedirectFilter>(),
                    ),
                    (
                        "responseHeaderModifier".to_owned(),
                        __gen.subschema_for::<crate::kube::apis::networking::gateway::v1::HTTPHeaderFilter>(),
                    ),
                    (
                        "type".to_owned(),
                        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
                            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                                description: Some("Type identifies the type of filter to apply. As with other API fields, types are classified into three conformance levels: \n - Core: Filter types and their corresponding configuration defined by \"Support: Core\" in this package, e.g. \"RequestHeaderModifier\". All implementations must support core filters. \n - Extended: Filter types and their corresponding configuration defined by \"Support: Extended\" in this package, e.g. \"RequestMirror\". Implementers are encouraged to support extended filters. \n - Implementation-specific: Filters that are defined and supported by specific vendors. In the future, filters showing convergence in behavior across multiple implementations will be considered for inclusion in extended or core conformance levels. Filter-specific configuration for such filters is specified using the ExtensionRef field. `Type` should be set to \"ExtensionRef\" for custom filters. \n Implementers are encouraged to define custom implementation types to extend the core API with implementation-specific behavior. \n If a reference to a custom filter type cannot be resolved, the filter MUST NOT be skipped. Instead, requests that would have been processed by that filter MUST receive a HTTP error response. \n Note that values may be added to this enum, implementations must ensure that unknown values will not cause a crash. \n Unknown values here must result in the implementation setting the Accepted Condition for the Route to `status: False`, with a Reason of `UnsupportedValue`.".to_owned()),
                                ..Default::default()
                            })),
                            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::String))),
                            ..Default::default()
                        }),
                    ),
                    (
                        "urlRewrite".to_owned(),
                        __gen.subschema_for::<crate::kube::apis::networking::gateway::v1::HTTPURLRewriteFilter>(),
                    ),
                ].into(),
                required: [
                    "type".to_owned(),
                ].into(),
                ..Default::default()
            })),
            ..Default::default()
        })
    }
}
