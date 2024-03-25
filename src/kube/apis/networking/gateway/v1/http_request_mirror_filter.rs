// Generated from definition io.k8s.networking.gateway.v1.HTTPRequestMirrorFilter

/// RequestMirror defines a schema for a filter that mirrors requests. Requests are sent to the specified destination, but responses from that destination are ignored. 
///  This filter can be used multiple times within the same rule. Note that not all implementations will be able to support mirroring to multiple backends. 
///  Support: Extended
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HTTPRequestMirrorFilter {
    pub backend_ref: crate::kube::apis::networking::gateway::v1::BackendObjectReference,
}

impl crate::kube::apis::DeepMerge for HTTPRequestMirrorFilter {
    fn merge_from(&mut self, other: Self) {
        crate::kube::apis::DeepMerge::merge_from(&mut self.backend_ref, other.backend_ref);
    }
}

impl<'de> crate::kube::apis::serde::Deserialize<'de> for HTTPRequestMirrorFilter {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: crate::kube::apis::serde::Deserializer<'de> {
        #[allow(non_camel_case_types)]
        enum Field {
            Key_backend_ref,
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
                            "backendRef" => Field::Key_backend_ref,
                            _ => Field::Other,
                        })
                    }
                }

                deserializer.deserialize_identifier(Visitor)
            }
        }

        struct Visitor;

        impl<'de> crate::kube::apis::serde::de::Visitor<'de> for Visitor {
            type Value = HTTPRequestMirrorFilter;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("HTTPRequestMirrorFilter")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error> where A: crate::kube::apis::serde::de::MapAccess<'de> {
                let mut value_backend_ref: Option<crate::kube::apis::networking::gateway::v1::BackendObjectReference> = None;

                while let Some(key) = crate::kube::apis::serde::de::MapAccess::next_key::<Field>(&mut map)? {
                    match key {
                        Field::Key_backend_ref => value_backend_ref = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Other => { let _: crate::kube::apis::serde::de::IgnoredAny = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?; },
                    }
                }

                Ok(HTTPRequestMirrorFilter {
                    backend_ref: value_backend_ref.unwrap_or_default(),
                })
            }
        }

        deserializer.deserialize_struct(
            "HTTPRequestMirrorFilter",
            &[
                "backendRef",
            ],
            Visitor,
        )
    }
}

impl crate::kube::apis::serde::Serialize for HTTPRequestMirrorFilter {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: crate::kube::apis::serde::Serializer {
        let mut state = serializer.serialize_struct(
            "HTTPRequestMirrorFilter",
            1,
        )?;
        crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "backendRef", &self.backend_ref)?;
        crate::kube::apis::serde::ser::SerializeStruct::end(state)
    }
}

#[cfg(feature = "schemars")]
impl crate::kube::apis::schemars::JsonSchema for HTTPRequestMirrorFilter {
    fn schema_name() -> String {
        "io.k8s.networking.gateway.v1.HTTPRequestMirrorFilter".to_owned()
    }

    fn json_schema(__gen: &mut crate::kube::apis::schemars::gen::SchemaGenerator) -> crate::kube::apis::schemars::schema::Schema {
        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                description: Some("RequestMirror defines a schema for a filter that mirrors requests. Requests are sent to the specified destination, but responses from that destination are ignored. \n This filter can be used multiple times within the same rule. Note that not all implementations will be able to support mirroring to multiple backends. \n Support: Extended".to_owned()),
                ..Default::default()
            })),
            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::Object))),
            object: Some(Box::new(crate::kube::apis::schemars::schema::ObjectValidation {
                properties: [
                    (
                        "backendRef".to_owned(),
                        __gen.subschema_for::<crate::kube::apis::networking::gateway::v1::BackendObjectReference>(),
                    ),
                ].into(),
                required: [
                    "backendRef".to_owned(),
                ].into(),
                ..Default::default()
            })),
            ..Default::default()
        })
    }
}
