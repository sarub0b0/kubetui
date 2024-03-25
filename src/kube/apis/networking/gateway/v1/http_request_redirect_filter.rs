// Generated from definition io.k8s.networking.gateway.v1.HTTPRequestRedirectFilter

/// RequestRedirect defines a schema for a filter that responds to the request with an HTTP redirection. 
///  Support: Core
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HTTPRequestRedirectFilter {
    /// Hostname is the hostname to be used in the value of the `Location` header in the response. When empty, the hostname in the `Host` header of the request is used. 
    ///  Support: Core
    pub hostname: Option<String>,

    pub path: Option<crate::kube::apis::networking::gateway::v1::HTTPPathModifier>,

    /// Port is the port to be used in the value of the `Location` header in the response. 
    ///  If no port is specified, the redirect port MUST be derived using the following rules: 
    ///  * If redirect scheme is not-empty, the redirect port MUST be the well-known port associated with the redirect scheme. Specifically "http" to port 80 and "https" to port 443. If the redirect scheme does not have a well-known port, the listener port of the Gateway SHOULD be used. * If redirect scheme is empty, the redirect port MUST be the Gateway Listener port. 
    ///  Implementations SHOULD NOT add the port number in the 'Location' header in the following cases: 
    ///  * A Location header that will use HTTP (whether that is determined via the Listener protocol or the Scheme field) _and_ use port 80. * A Location header that will use HTTPS (whether that is determined via the Listener protocol or the Scheme field) _and_ use port 443. 
    ///  Support: Extended
    pub port: Option<i32>,

    /// Scheme is the scheme to be used in the value of the `Location` header in the response. When empty, the scheme of the request is used. 
    ///  Scheme redirects can affect the port of the redirect, for more information, refer to the documentation for the port field of this filter. 
    ///  Note that values may be added to this enum, implementations must ensure that unknown values will not cause a crash. 
    ///  Unknown values here must result in the implementation setting the Accepted Condition for the Route to `status: False`, with a Reason of `UnsupportedValue`. 
    ///  Support: Extended
    pub scheme: Option<String>,

    /// StatusCode is the HTTP status code to be used in response. 
    ///  Note that values may be added to this enum, implementations must ensure that unknown values will not cause a crash. 
    ///  Unknown values here must result in the implementation setting the Accepted Condition for the Route to `status: False`, with a Reason of `UnsupportedValue`. 
    ///  Support: Core
    pub status_code: Option<i64>,
}

impl crate::kube::apis::DeepMerge for HTTPRequestRedirectFilter {
    fn merge_from(&mut self, other: Self) {
        crate::kube::apis::DeepMerge::merge_from(&mut self.hostname, other.hostname);
        crate::kube::apis::DeepMerge::merge_from(&mut self.path, other.path);
        crate::kube::apis::DeepMerge::merge_from(&mut self.port, other.port);
        crate::kube::apis::DeepMerge::merge_from(&mut self.scheme, other.scheme);
        crate::kube::apis::DeepMerge::merge_from(&mut self.status_code, other.status_code);
    }
}

impl<'de> crate::kube::apis::serde::Deserialize<'de> for HTTPRequestRedirectFilter {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: crate::kube::apis::serde::Deserializer<'de> {
        #[allow(non_camel_case_types)]
        enum Field {
            Key_hostname,
            Key_path,
            Key_port,
            Key_scheme,
            Key_status_code,
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
                            "hostname" => Field::Key_hostname,
                            "path" => Field::Key_path,
                            "port" => Field::Key_port,
                            "scheme" => Field::Key_scheme,
                            "statusCode" => Field::Key_status_code,
                            _ => Field::Other,
                        })
                    }
                }

                deserializer.deserialize_identifier(Visitor)
            }
        }

        struct Visitor;

        impl<'de> crate::kube::apis::serde::de::Visitor<'de> for Visitor {
            type Value = HTTPRequestRedirectFilter;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("HTTPRequestRedirectFilter")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error> where A: crate::kube::apis::serde::de::MapAccess<'de> {
                let mut value_hostname: Option<String> = None;
                let mut value_path: Option<crate::kube::apis::networking::gateway::v1::HTTPPathModifier> = None;
                let mut value_port: Option<i32> = None;
                let mut value_scheme: Option<String> = None;
                let mut value_status_code: Option<i64> = None;

                while let Some(key) = crate::kube::apis::serde::de::MapAccess::next_key::<Field>(&mut map)? {
                    match key {
                        Field::Key_hostname => value_hostname = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Key_path => value_path = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Key_port => value_port = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Key_scheme => value_scheme = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Key_status_code => value_status_code = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Other => { let _: crate::kube::apis::serde::de::IgnoredAny = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?; },
                    }
                }

                Ok(HTTPRequestRedirectFilter {
                    hostname: value_hostname,
                    path: value_path,
                    port: value_port,
                    scheme: value_scheme,
                    status_code: value_status_code,
                })
            }
        }

        deserializer.deserialize_struct(
            "HTTPRequestRedirectFilter",
            &[
                "hostname",
                "path",
                "port",
                "scheme",
                "statusCode",
            ],
            Visitor,
        )
    }
}

impl crate::kube::apis::serde::Serialize for HTTPRequestRedirectFilter {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: crate::kube::apis::serde::Serializer {
        let mut state = serializer.serialize_struct(
            "HTTPRequestRedirectFilter",
            self.hostname.as_ref().map_or(0, |_| 1) +
            self.path.as_ref().map_or(0, |_| 1) +
            self.port.as_ref().map_or(0, |_| 1) +
            self.scheme.as_ref().map_or(0, |_| 1) +
            self.status_code.as_ref().map_or(0, |_| 1),
        )?;
        if let Some(value) = &self.hostname {
            crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "hostname", value)?;
        }
        if let Some(value) = &self.path {
            crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "path", value)?;
        }
        if let Some(value) = &self.port {
            crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "port", value)?;
        }
        if let Some(value) = &self.scheme {
            crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "scheme", value)?;
        }
        if let Some(value) = &self.status_code {
            crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "statusCode", value)?;
        }
        crate::kube::apis::serde::ser::SerializeStruct::end(state)
    }
}

#[cfg(feature = "schemars")]
impl crate::kube::apis::schemars::JsonSchema for HTTPRequestRedirectFilter {
    fn schema_name() -> String {
        "io.k8s.networking.gateway.v1.HTTPRequestRedirectFilter".to_owned()
    }

    fn json_schema(__gen: &mut crate::kube::apis::schemars::gen::SchemaGenerator) -> crate::kube::apis::schemars::schema::Schema {
        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                description: Some("RequestRedirect defines a schema for a filter that responds to the request with an HTTP redirection. \n Support: Core".to_owned()),
                ..Default::default()
            })),
            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::Object))),
            object: Some(Box::new(crate::kube::apis::schemars::schema::ObjectValidation {
                properties: [
                    (
                        "hostname".to_owned(),
                        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
                            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                                description: Some("Hostname is the hostname to be used in the value of the `Location` header in the response. When empty, the hostname in the `Host` header of the request is used. \n Support: Core".to_owned()),
                                ..Default::default()
                            })),
                            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::String))),
                            ..Default::default()
                        }),
                    ),
                    (
                        "path".to_owned(),
                        __gen.subschema_for::<crate::kube::apis::networking::gateway::v1::HTTPPathModifier>(),
                    ),
                    (
                        "port".to_owned(),
                        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
                            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                                description: Some("Port is the port to be used in the value of the `Location` header in the response. \n If no port is specified, the redirect port MUST be derived using the following rules: \n * If redirect scheme is not-empty, the redirect port MUST be the well-known port associated with the redirect scheme. Specifically \"http\" to port 80 and \"https\" to port 443. If the redirect scheme does not have a well-known port, the listener port of the Gateway SHOULD be used. * If redirect scheme is empty, the redirect port MUST be the Gateway Listener port. \n Implementations SHOULD NOT add the port number in the 'Location' header in the following cases: \n * A Location header that will use HTTP (whether that is determined via the Listener protocol or the Scheme field) _and_ use port 80. * A Location header that will use HTTPS (whether that is determined via the Listener protocol or the Scheme field) _and_ use port 443. \n Support: Extended".to_owned()),
                                ..Default::default()
                            })),
                            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::Integer))),
                            format: Some("int32".to_owned()),
                            ..Default::default()
                        }),
                    ),
                    (
                        "scheme".to_owned(),
                        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
                            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                                description: Some("Scheme is the scheme to be used in the value of the `Location` header in the response. When empty, the scheme of the request is used. \n Scheme redirects can affect the port of the redirect, for more information, refer to the documentation for the port field of this filter. \n Note that values may be added to this enum, implementations must ensure that unknown values will not cause a crash. \n Unknown values here must result in the implementation setting the Accepted Condition for the Route to `status: False`, with a Reason of `UnsupportedValue`. \n Support: Extended".to_owned()),
                                ..Default::default()
                            })),
                            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::String))),
                            ..Default::default()
                        }),
                    ),
                    (
                        "statusCode".to_owned(),
                        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
                            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                                description: Some("StatusCode is the HTTP status code to be used in response. \n Note that values may be added to this enum, implementations must ensure that unknown values will not cause a crash. \n Unknown values here must result in the implementation setting the Accepted Condition for the Route to `status: False`, with a Reason of `UnsupportedValue`. \n Support: Core".to_owned()),
                                ..Default::default()
                            })),
                            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::Integer))),
                            format: Some("int64".to_owned()),
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
