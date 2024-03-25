// Generated from definition io.k8s.networking.gateway.v1.GatewayAddress

/// GatewayAddress describes an address that can be bound to a Gateway.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GatewayAddress {
    /// Type of the address.
    pub type_: Option<String>,

    /// Value of the address. The validity of the values will depend on the type and support by the controller. 
    ///  Examples: `1.2.3.4`, `128::1`, `my-ip-address`.
    pub value: String,
}

impl crate::kube::apis::DeepMerge for GatewayAddress {
    fn merge_from(&mut self, other: Self) {
        crate::kube::apis::DeepMerge::merge_from(&mut self.type_, other.type_);
        crate::kube::apis::DeepMerge::merge_from(&mut self.value, other.value);
    }
}

impl<'de> crate::kube::apis::serde::Deserialize<'de> for GatewayAddress {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: crate::kube::apis::serde::Deserializer<'de> {
        #[allow(non_camel_case_types)]
        enum Field {
            Key_type_,
            Key_value,
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
                            "type" => Field::Key_type_,
                            "value" => Field::Key_value,
                            _ => Field::Other,
                        })
                    }
                }

                deserializer.deserialize_identifier(Visitor)
            }
        }

        struct Visitor;

        impl<'de> crate::kube::apis::serde::de::Visitor<'de> for Visitor {
            type Value = GatewayAddress;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("GatewayAddress")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error> where A: crate::kube::apis::serde::de::MapAccess<'de> {
                let mut value_type_: Option<String> = None;
                let mut value_value: Option<String> = None;

                while let Some(key) = crate::kube::apis::serde::de::MapAccess::next_key::<Field>(&mut map)? {
                    match key {
                        Field::Key_type_ => value_type_ = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Key_value => value_value = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Other => { let _: crate::kube::apis::serde::de::IgnoredAny = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?; },
                    }
                }

                Ok(GatewayAddress {
                    type_: value_type_,
                    value: value_value.unwrap_or_default(),
                })
            }
        }

        deserializer.deserialize_struct(
            "GatewayAddress",
            &[
                "type",
                "value",
            ],
            Visitor,
        )
    }
}

impl crate::kube::apis::serde::Serialize for GatewayAddress {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: crate::kube::apis::serde::Serializer {
        let mut state = serializer.serialize_struct(
            "GatewayAddress",
            1 +
            self.type_.as_ref().map_or(0, |_| 1),
        )?;
        if let Some(value) = &self.type_ {
            crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "type", value)?;
        }
        crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "value", &self.value)?;
        crate::kube::apis::serde::ser::SerializeStruct::end(state)
    }
}

#[cfg(feature = "schemars")]
impl crate::kube::apis::schemars::JsonSchema for GatewayAddress {
    fn schema_name() -> String {
        "io.k8s.networking.gateway.v1.GatewayAddress".to_owned()
    }

    fn json_schema(__gen: &mut crate::kube::apis::schemars::gen::SchemaGenerator) -> crate::kube::apis::schemars::schema::Schema {
        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                description: Some("GatewayAddress describes an address that can be bound to a Gateway.".to_owned()),
                ..Default::default()
            })),
            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::Object))),
            object: Some(Box::new(crate::kube::apis::schemars::schema::ObjectValidation {
                properties: [
                    (
                        "type".to_owned(),
                        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
                            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                                description: Some("Type of the address.".to_owned()),
                                ..Default::default()
                            })),
                            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::String))),
                            ..Default::default()
                        }),
                    ),
                    (
                        "value".to_owned(),
                        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
                            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                                description: Some("Value of the address. The validity of the values will depend on the type and support by the controller. \n Examples: `1.2.3.4`, `128::1`, `my-ip-address`.".to_owned()),
                                ..Default::default()
                            })),
                            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::String))),
                            ..Default::default()
                        }),
                    ),
                ].into(),
                required: [
                    "value".to_owned(),
                ].into(),
                ..Default::default()
            })),
            ..Default::default()
        })
    }
}
