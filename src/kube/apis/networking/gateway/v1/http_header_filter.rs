// Generated from definition io.k8s.networking.gateway.v1.HTTPHeaderFilter

/// RequestHeaderModifier defines a schema for a filter that modifies request headers. 
///  Support: Core
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HTTPHeaderFilter {
    /// Add adds the given header(s) (name, value) to the request before the action. It appends to any existing values associated with the header name. 
    ///  Input: GET /foo HTTP/1.1 my-header: foo 
    ///  Config: add: - name: "my-header" value: "bar,baz" 
    ///  Output: GET /foo HTTP/1.1 my-header: foo,bar,baz
    pub add: Option<Vec<crate::kube::apis::networking::gateway::v1::HTTPHeader>>,

    /// Remove the given header(s) from the HTTP request before the action. The value of Remove is a list of HTTP header names. Note that the header names are case-insensitive (see https://datatracker.ietf.org/doc/html/rfc2616#section-4.2). 
    ///  Input: GET /foo HTTP/1.1 my-header1: foo my-header2: bar my-header3: baz 
    ///  Config: remove: \["my-header1", "my-header3"\] 
    ///  Output: GET /foo HTTP/1.1 my-header2: bar
    pub remove: Option<Vec<String>>,

    /// Set overwrites the request with the given header (name, value) before the action. 
    ///  Input: GET /foo HTTP/1.1 my-header: foo 
    ///  Config: set: - name: "my-header" value: "bar" 
    ///  Output: GET /foo HTTP/1.1 my-header: bar
    pub set: Option<Vec<crate::kube::apis::networking::gateway::v1::HTTPHeader>>,
}

impl crate::kube::apis::DeepMerge for HTTPHeaderFilter {
    fn merge_from(&mut self, other: Self) {
        crate::kube::apis::merge_strategies::list::map(
            &mut self.add,
            other.add,
            &[|lhs, rhs| lhs.name == rhs.name],
            |current_item, other_item| {
                crate::kube::apis::DeepMerge::merge_from(current_item, other_item);
            },
        );
        crate::kube::apis::merge_strategies::list::set(&mut self.remove, other.remove);
        crate::kube::apis::merge_strategies::list::map(
            &mut self.set,
            other.set,
            &[|lhs, rhs| lhs.name == rhs.name],
            |current_item, other_item| {
                crate::kube::apis::DeepMerge::merge_from(current_item, other_item);
            },
        );
    }
}

impl<'de> crate::kube::apis::serde::Deserialize<'de> for HTTPHeaderFilter {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: crate::kube::apis::serde::Deserializer<'de> {
        #[allow(non_camel_case_types)]
        enum Field {
            Key_add,
            Key_remove,
            Key_set,
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
                            "add" => Field::Key_add,
                            "remove" => Field::Key_remove,
                            "set" => Field::Key_set,
                            _ => Field::Other,
                        })
                    }
                }

                deserializer.deserialize_identifier(Visitor)
            }
        }

        struct Visitor;

        impl<'de> crate::kube::apis::serde::de::Visitor<'de> for Visitor {
            type Value = HTTPHeaderFilter;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("HTTPHeaderFilter")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error> where A: crate::kube::apis::serde::de::MapAccess<'de> {
                let mut value_add: Option<Vec<crate::kube::apis::networking::gateway::v1::HTTPHeader>> = None;
                let mut value_remove: Option<Vec<String>> = None;
                let mut value_set: Option<Vec<crate::kube::apis::networking::gateway::v1::HTTPHeader>> = None;

                while let Some(key) = crate::kube::apis::serde::de::MapAccess::next_key::<Field>(&mut map)? {
                    match key {
                        Field::Key_add => value_add = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Key_remove => value_remove = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Key_set => value_set = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Other => { let _: crate::kube::apis::serde::de::IgnoredAny = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?; },
                    }
                }

                Ok(HTTPHeaderFilter {
                    add: value_add,
                    remove: value_remove,
                    set: value_set,
                })
            }
        }

        deserializer.deserialize_struct(
            "HTTPHeaderFilter",
            &[
                "add",
                "remove",
                "set",
            ],
            Visitor,
        )
    }
}

impl crate::kube::apis::serde::Serialize for HTTPHeaderFilter {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: crate::kube::apis::serde::Serializer {
        let mut state = serializer.serialize_struct(
            "HTTPHeaderFilter",
            self.add.as_ref().map_or(0, |_| 1) +
            self.remove.as_ref().map_or(0, |_| 1) +
            self.set.as_ref().map_or(0, |_| 1),
        )?;
        if let Some(value) = &self.add {
            crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "add", value)?;
        }
        if let Some(value) = &self.remove {
            crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "remove", value)?;
        }
        if let Some(value) = &self.set {
            crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "set", value)?;
        }
        crate::kube::apis::serde::ser::SerializeStruct::end(state)
    }
}

#[cfg(feature = "schemars")]
impl crate::kube::apis::schemars::JsonSchema for HTTPHeaderFilter {
    fn schema_name() -> String {
        "io.k8s.networking.gateway.v1.HTTPHeaderFilter".to_owned()
    }

    fn json_schema(__gen: &mut crate::kube::apis::schemars::gen::SchemaGenerator) -> crate::kube::apis::schemars::schema::Schema {
        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                description: Some("RequestHeaderModifier defines a schema for a filter that modifies request headers. \n Support: Core".to_owned()),
                ..Default::default()
            })),
            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::Object))),
            object: Some(Box::new(crate::kube::apis::schemars::schema::ObjectValidation {
                properties: [
                    (
                        "add".to_owned(),
                        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
                            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                                description: Some("Add adds the given header(s) (name, value) to the request before the action. It appends to any existing values associated with the header name. \n Input: GET /foo HTTP/1.1 my-header: foo \n Config: add: - name: \"my-header\" value: \"bar,baz\" \n Output: GET /foo HTTP/1.1 my-header: foo,bar,baz".to_owned()),
                                ..Default::default()
                            })),
                            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::Array))),
                            array: Some(Box::new(crate::kube::apis::schemars::schema::ArrayValidation {
                                items: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(__gen.subschema_for::<crate::kube::apis::networking::gateway::v1::HTTPHeader>()))),
                                ..Default::default()
                            })),
                            ..Default::default()
                        }),
                    ),
                    (
                        "remove".to_owned(),
                        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
                            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                                description: Some("Remove the given header(s) from the HTTP request before the action. The value of Remove is a list of HTTP header names. Note that the header names are case-insensitive (see https://datatracker.ietf.org/doc/html/rfc2616#section-4.2). \n Input: GET /foo HTTP/1.1 my-header1: foo my-header2: bar my-header3: baz \n Config: remove: [\"my-header1\", \"my-header3\"] \n Output: GET /foo HTTP/1.1 my-header2: bar".to_owned()),
                                ..Default::default()
                            })),
                            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::Array))),
                            array: Some(Box::new(crate::kube::apis::schemars::schema::ArrayValidation {
                                items: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(
                                    crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
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
                        "set".to_owned(),
                        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
                            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                                description: Some("Set overwrites the request with the given header (name, value) before the action. \n Input: GET /foo HTTP/1.1 my-header: foo \n Config: set: - name: \"my-header\" value: \"bar\" \n Output: GET /foo HTTP/1.1 my-header: bar".to_owned()),
                                ..Default::default()
                            })),
                            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::Array))),
                            array: Some(Box::new(crate::kube::apis::schemars::schema::ArrayValidation {
                                items: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(__gen.subschema_for::<crate::kube::apis::networking::gateway::v1::HTTPHeader>()))),
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
