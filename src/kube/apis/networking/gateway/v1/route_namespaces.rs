// Generated from definition io.k8s.networking.gateway.v1.RouteNamespaces

/// Namespaces indicates namespaces from which Routes may be attached to this Listener. This is restricted to the namespace of this Gateway by default. 
///  Support: Core
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RouteNamespaces {
    pub from: Option<crate::kube::apis::networking::gateway::v1::FromNamespaces>,

    /// Selector must be specified when From is set to "Selector". In that case, only Routes in Namespaces matching this Selector will be selected by this Gateway. This field is ignored for other values of "From". 
    ///  Support: Core
    pub selector: Option<crate::kube::apis::apimachinery::pkg::apis::meta::v1::LabelSelector>,
}

impl crate::kube::apis::DeepMerge for RouteNamespaces {
    fn merge_from(&mut self, other: Self) {
        crate::kube::apis::DeepMerge::merge_from(&mut self.from, other.from);
        crate::kube::apis::DeepMerge::merge_from(&mut self.selector, other.selector);
    }
}

impl<'de> crate::kube::apis::serde::Deserialize<'de> for RouteNamespaces {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: crate::kube::apis::serde::Deserializer<'de> {
        #[allow(non_camel_case_types)]
        enum Field {
            Key_from,
            Key_selector,
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
                            "from" => Field::Key_from,
                            "selector" => Field::Key_selector,
                            _ => Field::Other,
                        })
                    }
                }

                deserializer.deserialize_identifier(Visitor)
            }
        }

        struct Visitor;

        impl<'de> crate::kube::apis::serde::de::Visitor<'de> for Visitor {
            type Value = RouteNamespaces;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("RouteNamespaces")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error> where A: crate::kube::apis::serde::de::MapAccess<'de> {
                let mut value_from: Option<crate::kube::apis::networking::gateway::v1::FromNamespaces> = None;
                let mut value_selector: Option<crate::kube::apis::apimachinery::pkg::apis::meta::v1::LabelSelector> = None;

                while let Some(key) = crate::kube::apis::serde::de::MapAccess::next_key::<Field>(&mut map)? {
                    match key {
                        Field::Key_from => value_from = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Key_selector => value_selector = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Other => { let _: crate::kube::apis::serde::de::IgnoredAny = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?; },
                    }
                }

                Ok(RouteNamespaces {
                    from: value_from,
                    selector: value_selector,
                })
            }
        }

        deserializer.deserialize_struct(
            "RouteNamespaces",
            &[
                "from",
                "selector",
            ],
            Visitor,
        )
    }
}

impl crate::kube::apis::serde::Serialize for RouteNamespaces {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: crate::kube::apis::serde::Serializer {
        let mut state = serializer.serialize_struct(
            "RouteNamespaces",
            self.from.as_ref().map_or(0, |_| 1) +
            self.selector.as_ref().map_or(0, |_| 1),
        )?;
        if let Some(value) = &self.from {
            crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "from", value)?;
        }
        if let Some(value) = &self.selector {
            crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "selector", value)?;
        }
        crate::kube::apis::serde::ser::SerializeStruct::end(state)
    }
}

#[cfg(feature = "schemars")]
impl crate::kube::apis::schemars::JsonSchema for RouteNamespaces {
    fn schema_name() -> String {
        "io.k8s.networking.gateway.v1.RouteNamespaces".to_owned()
    }

    fn json_schema(__gen: &mut crate::kube::apis::schemars::gen::SchemaGenerator) -> crate::kube::apis::schemars::schema::Schema {
        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                description: Some("Namespaces indicates namespaces from which Routes may be attached to this Listener. This is restricted to the namespace of this Gateway by default. \n Support: Core".to_owned()),
                ..Default::default()
            })),
            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::Object))),
            object: Some(Box::new(crate::kube::apis::schemars::schema::ObjectValidation {
                properties: [
                    (
                        "from".to_owned(),
                        __gen.subschema_for::<crate::kube::apis::networking::gateway::v1::FromNamespaces>(),
                    ),
                    (
                        "selector".to_owned(),
                        {
                            let mut schema_obj = __gen.subschema_for::<crate::kube::apis::apimachinery::pkg::apis::meta::v1::LabelSelector>().into_object();
                            schema_obj.metadata = Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                                description: Some("Selector must be specified when From is set to \"Selector\". In that case, only Routes in Namespaces matching this Selector will be selected by this Gateway. This field is ignored for other values of \"From\". \n Support: Core".to_owned()),
                                ..Default::default()
                            }));
                            crate::kube::apis::schemars::schema::Schema::Object(schema_obj)
                        },
                    ),
                ].into(),
                ..Default::default()
            })),
            ..Default::default()
        })
    }
}
