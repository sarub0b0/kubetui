// Generated from definition io.k8s.networking.gateway.v1.LocalObjectReference

/// ExtensionRef is an optional, implementation-specific extension to the "filter" behavior.  For example, resource "myroutefilter" in group "networking.example.net"). ExtensionRef MUST NOT be used for core and extended filters. 
///  This filter can be used multiple times within the same rule. 
///  Support: Implementation-specific
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LocalObjectReference {
    /// Group is the group of the referent. For example, "gateway.networking.k8s.io". When unspecified or empty string, core API group is inferred.
    pub group: String,

    /// Kind is kind of the referent. For example "HTTPRoute" or "Service".
    pub kind: String,

    /// Name is the name of the referent.
    pub name: String,
}

impl crate::kube::apis::DeepMerge for LocalObjectReference {
    fn merge_from(&mut self, other: Self) {
        crate::kube::apis::DeepMerge::merge_from(&mut self.group, other.group);
        crate::kube::apis::DeepMerge::merge_from(&mut self.kind, other.kind);
        crate::kube::apis::DeepMerge::merge_from(&mut self.name, other.name);
    }
}

impl<'de> crate::kube::apis::serde::Deserialize<'de> for LocalObjectReference {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: crate::kube::apis::serde::Deserializer<'de> {
        #[allow(non_camel_case_types)]
        enum Field {
            Key_group,
            Key_kind,
            Key_name,
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
                            "group" => Field::Key_group,
                            "kind" => Field::Key_kind,
                            "name" => Field::Key_name,
                            _ => Field::Other,
                        })
                    }
                }

                deserializer.deserialize_identifier(Visitor)
            }
        }

        struct Visitor;

        impl<'de> crate::kube::apis::serde::de::Visitor<'de> for Visitor {
            type Value = LocalObjectReference;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("LocalObjectReference")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error> where A: crate::kube::apis::serde::de::MapAccess<'de> {
                let mut value_group: Option<String> = None;
                let mut value_kind: Option<String> = None;
                let mut value_name: Option<String> = None;

                while let Some(key) = crate::kube::apis::serde::de::MapAccess::next_key::<Field>(&mut map)? {
                    match key {
                        Field::Key_group => value_group = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Key_kind => value_kind = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Key_name => value_name = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Other => { let _: crate::kube::apis::serde::de::IgnoredAny = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?; },
                    }
                }

                Ok(LocalObjectReference {
                    group: value_group.unwrap_or_default(),
                    kind: value_kind.unwrap_or_default(),
                    name: value_name.unwrap_or_default(),
                })
            }
        }

        deserializer.deserialize_struct(
            "LocalObjectReference",
            &[
                "group",
                "kind",
                "name",
            ],
            Visitor,
        )
    }
}

impl crate::kube::apis::serde::Serialize for LocalObjectReference {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: crate::kube::apis::serde::Serializer {
        let mut state = serializer.serialize_struct(
            "LocalObjectReference",
            3,
        )?;
        crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "group", &self.group)?;
        crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "kind", &self.kind)?;
        crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "name", &self.name)?;
        crate::kube::apis::serde::ser::SerializeStruct::end(state)
    }
}

#[cfg(feature = "schemars")]
impl crate::kube::apis::schemars::JsonSchema for LocalObjectReference {
    fn schema_name() -> String {
        "io.k8s.networking.gateway.v1.LocalObjectReference".to_owned()
    }

    fn json_schema(__gen: &mut crate::kube::apis::schemars::gen::SchemaGenerator) -> crate::kube::apis::schemars::schema::Schema {
        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                description: Some("ExtensionRef is an optional, implementation-specific extension to the \"filter\" behavior.  For example, resource \"myroutefilter\" in group \"networking.example.net\"). ExtensionRef MUST NOT be used for core and extended filters. \n This filter can be used multiple times within the same rule. \n Support: Implementation-specific".to_owned()),
                ..Default::default()
            })),
            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::Object))),
            object: Some(Box::new(crate::kube::apis::schemars::schema::ObjectValidation {
                properties: [
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
                                description: Some("Kind is kind of the referent. For example \"HTTPRoute\" or \"Service\".".to_owned()),
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
                ].into(),
                required: [
                    "group".to_owned(),
                    "kind".to_owned(),
                    "name".to_owned(),
                ].into(),
                ..Default::default()
            })),
            ..Default::default()
        })
    }
}
