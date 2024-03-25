// Generated from definition io.k8s.networking.gateway.v1.GatewayTLSConfig

/// TLS is the TLS configuration for the Listener. This field is required if the Protocol field is "HTTPS" or "TLS". It is invalid to set this field if the Protocol field is "HTTP", "TCP", or "UDP". 
///  The association of SNIs to Certificate defined in GatewayTLSConfig is defined based on the Hostname field for this listener. 
///  The GatewayClass MUST use the longest matching SNI out of all available certificates for any TLS handshake. 
///  Support: Core
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GatewayTLSConfig {
    /// CertificateRefs contains a series of references to Kubernetes objects that contains TLS certificates and private keys. These certificates are used to establish a TLS handshake for requests that match the hostname of the associated listener. 
    ///  A single CertificateRef to a Kubernetes Secret has "Core" support. Implementations MAY choose to support attaching multiple certificates to a Listener, but this behavior is implementation-specific. 
    ///  References to a resource in different namespace are invalid UNLESS there is a ReferenceGrant in the target namespace that allows the certificate to be attached. If a ReferenceGrant does not allow this reference, the "ResolvedRefs" condition MUST be set to False for this listener with the "RefNotPermitted" reason. 
    ///  This field is required to have at least one element when the mode is set to "Terminate" (default) and is optional otherwise. 
    ///  CertificateRefs can reference to standard Kubernetes resources, i.e. Secret, or implementation-specific custom resources. 
    ///  Support: Core - A single reference to a Kubernetes Secret of type kubernetes.io/tls 
    ///  Support: Implementation-specific (More than one reference or other resource types)
    pub certificate_refs: Option<Vec<crate::kube::apis::networking::gateway::v1::SecretObjectReference>>,

    /// Mode defines the TLS behavior for the TLS session initiated by the client. There are two possible modes: 
    ///  - Terminate: The TLS session between the downstream client and the Gateway is terminated at the Gateway. This mode requires certificateRefs to be set and contain at least one element. - Passthrough: The TLS session is NOT terminated by the Gateway. This implies that the Gateway can't decipher the TLS stream except for the ClientHello message of the TLS protocol. CertificateRefs field is ignored in this mode. 
    ///  Support: Core
    pub mode: Option<String>,

    /// Options are a list of key/value pairs to enable extended TLS configuration for each implementation. For example, configuring the minimum TLS version or supported cipher suites. 
    ///  A set of common keys MAY be defined by the API in the future. To avoid any ambiguity, implementation-specific definitions MUST use domain-prefixed names, such as `example.com/my-custom-option`. Un-prefixed names are reserved for key names defined by Gateway API. 
    ///  Support: Implementation-specific
    pub options: Option<std::collections::BTreeMap<String, String>>,
}

impl crate::kube::apis::DeepMerge for GatewayTLSConfig {
    fn merge_from(&mut self, other: Self) {
        crate::kube::apis::merge_strategies::list::atomic(&mut self.certificate_refs, other.certificate_refs);
        crate::kube::apis::DeepMerge::merge_from(&mut self.mode, other.mode);
        crate::kube::apis::merge_strategies::map::granular(&mut self.options, other.options, |current_item, other_item| {
            crate::kube::apis::DeepMerge::merge_from(current_item, other_item);
        });
    }
}

impl<'de> crate::kube::apis::serde::Deserialize<'de> for GatewayTLSConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: crate::kube::apis::serde::Deserializer<'de> {
        #[allow(non_camel_case_types)]
        enum Field {
            Key_certificate_refs,
            Key_mode,
            Key_options,
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
                            "certificateRefs" => Field::Key_certificate_refs,
                            "mode" => Field::Key_mode,
                            "options" => Field::Key_options,
                            _ => Field::Other,
                        })
                    }
                }

                deserializer.deserialize_identifier(Visitor)
            }
        }

        struct Visitor;

        impl<'de> crate::kube::apis::serde::de::Visitor<'de> for Visitor {
            type Value = GatewayTLSConfig;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("GatewayTLSConfig")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error> where A: crate::kube::apis::serde::de::MapAccess<'de> {
                let mut value_certificate_refs: Option<Vec<crate::kube::apis::networking::gateway::v1::SecretObjectReference>> = None;
                let mut value_mode: Option<String> = None;
                let mut value_options: Option<std::collections::BTreeMap<String, String>> = None;

                while let Some(key) = crate::kube::apis::serde::de::MapAccess::next_key::<Field>(&mut map)? {
                    match key {
                        Field::Key_certificate_refs => value_certificate_refs = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Key_mode => value_mode = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Key_options => value_options = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Other => { let _: crate::kube::apis::serde::de::IgnoredAny = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?; },
                    }
                }

                Ok(GatewayTLSConfig {
                    certificate_refs: value_certificate_refs,
                    mode: value_mode,
                    options: value_options,
                })
            }
        }

        deserializer.deserialize_struct(
            "GatewayTLSConfig",
            &[
                "certificateRefs",
                "mode",
                "options",
            ],
            Visitor,
        )
    }
}

impl crate::kube::apis::serde::Serialize for GatewayTLSConfig {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: crate::kube::apis::serde::Serializer {
        let mut state = serializer.serialize_struct(
            "GatewayTLSConfig",
            self.certificate_refs.as_ref().map_or(0, |_| 1) +
            self.mode.as_ref().map_or(0, |_| 1) +
            self.options.as_ref().map_or(0, |_| 1),
        )?;
        if let Some(value) = &self.certificate_refs {
            crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "certificateRefs", value)?;
        }
        if let Some(value) = &self.mode {
            crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "mode", value)?;
        }
        if let Some(value) = &self.options {
            crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "options", value)?;
        }
        crate::kube::apis::serde::ser::SerializeStruct::end(state)
    }
}

#[cfg(feature = "schemars")]
impl crate::kube::apis::schemars::JsonSchema for GatewayTLSConfig {
    fn schema_name() -> String {
        "io.k8s.networking.gateway.v1.GatewayTLSConfig".to_owned()
    }

    fn json_schema(__gen: &mut crate::kube::apis::schemars::gen::SchemaGenerator) -> crate::kube::apis::schemars::schema::Schema {
        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                description: Some("TLS is the TLS configuration for the Listener. This field is required if the Protocol field is \"HTTPS\" or \"TLS\". It is invalid to set this field if the Protocol field is \"HTTP\", \"TCP\", or \"UDP\". \n The association of SNIs to Certificate defined in GatewayTLSConfig is defined based on the Hostname field for this listener. \n The GatewayClass MUST use the longest matching SNI out of all available certificates for any TLS handshake. \n Support: Core".to_owned()),
                ..Default::default()
            })),
            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::Object))),
            object: Some(Box::new(crate::kube::apis::schemars::schema::ObjectValidation {
                properties: [
                    (
                        "certificateRefs".to_owned(),
                        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
                            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                                description: Some("CertificateRefs contains a series of references to Kubernetes objects that contains TLS certificates and private keys. These certificates are used to establish a TLS handshake for requests that match the hostname of the associated listener. \n A single CertificateRef to a Kubernetes Secret has \"Core\" support. Implementations MAY choose to support attaching multiple certificates to a Listener, but this behavior is implementation-specific. \n References to a resource in different namespace are invalid UNLESS there is a ReferenceGrant in the target namespace that allows the certificate to be attached. If a ReferenceGrant does not allow this reference, the \"ResolvedRefs\" condition MUST be set to False for this listener with the \"RefNotPermitted\" reason. \n This field is required to have at least one element when the mode is set to \"Terminate\" (default) and is optional otherwise. \n CertificateRefs can reference to standard Kubernetes resources, i.e. Secret, or implementation-specific custom resources. \n Support: Core - A single reference to a Kubernetes Secret of type kubernetes.io/tls \n Support: Implementation-specific (More than one reference or other resource types)".to_owned()),
                                ..Default::default()
                            })),
                            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::Array))),
                            array: Some(Box::new(crate::kube::apis::schemars::schema::ArrayValidation {
                                items: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(__gen.subschema_for::<crate::kube::apis::networking::gateway::v1::SecretObjectReference>()))),
                                ..Default::default()
                            })),
                            ..Default::default()
                        }),
                    ),
                    (
                        "mode".to_owned(),
                        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
                            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                                description: Some("Mode defines the TLS behavior for the TLS session initiated by the client. There are two possible modes: \n - Terminate: The TLS session between the downstream client and the Gateway is terminated at the Gateway. This mode requires certificateRefs to be set and contain at least one element. - Passthrough: The TLS session is NOT terminated by the Gateway. This implies that the Gateway can't decipher the TLS stream except for the ClientHello message of the TLS protocol. CertificateRefs field is ignored in this mode. \n Support: Core".to_owned()),
                                ..Default::default()
                            })),
                            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::String))),
                            ..Default::default()
                        }),
                    ),
                    (
                        "options".to_owned(),
                        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
                            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                                description: Some("Options are a list of key/value pairs to enable extended TLS configuration for each implementation. For example, configuring the minimum TLS version or supported cipher suites. \n A set of common keys MAY be defined by the API in the future. To avoid any ambiguity, implementation-specific definitions MUST use domain-prefixed names, such as `example.com/my-custom-option`. Un-prefixed names are reserved for key names defined by Gateway API. \n Support: Implementation-specific".to_owned()),
                                ..Default::default()
                            })),
                            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::Object))),
                            object: Some(Box::new(crate::kube::apis::schemars::schema::ObjectValidation {
                                additional_properties: Some(Box::new(
                                    crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
                                        metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                                            description: Some("AnnotationValue is the value of an annotation in Gateway API. This is used for validation of maps such as TLS options. This roughly matches Kubernetes annotation validation, although the length validation in that case is based on the entire size of the annotations struct.".to_owned()),
                                            ..Default::default()
                                        })),
                                        instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::String))),
                                        ..Default::default()
                                    })
                                )),
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
