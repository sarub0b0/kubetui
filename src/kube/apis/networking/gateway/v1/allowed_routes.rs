// Generated from definition io.k8s.networking.gateway.v1.AllowedRoutes

/// AllowedRoutes defines the types of routes that MAY be attached to a Listener and the trusted namespaces where those Route resources MAY be present. 
///  Although a client request may match multiple route rules, only one rule may ultimately receive the request. Matching precedence MUST be determined in order of the following criteria: 
///  * The most specific match as defined by the Route type. * The oldest Route based on creation timestamp. For example, a Route with a creation timestamp of "2020-09-08 01:02:03" is given precedence over a Route with a creation timestamp of "2020-09-08 01:02:04". * If everything else is equivalent, the Route appearing first in alphabetical order (namespace/name) should be given precedence. For example, foo/bar is given precedence over foo/baz. 
///  All valid rules within a Route attached to this Listener should be implemented. Invalid Route rules can be ignored (sometimes that will mean the full Route). If a Route rule transitions from valid to invalid, support for that Route rule should be dropped to ensure consistency. For example, even if a filter specified by a Route rule is invalid, the rest of the rules within that Route should still be supported. 
///  Support: Core
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AllowedRoutes {
    /// Kinds specifies the groups and kinds of Routes that are allowed to bind to this Gateway Listener. When unspecified or empty, the kinds of Routes selected are determined using the Listener protocol. 
    ///  A RouteGroupKind MUST correspond to kinds of Routes that are compatible with the application protocol specified in the Listener's Protocol field. If an implementation does not support or recognize this resource type, it MUST set the "ResolvedRefs" condition to False for this Listener with the "InvalidRouteKinds" reason. 
    ///  Support: Core
    pub kinds: Option<Vec<crate::kube::apis::networking::gateway::v1::RouteGroupKind>>,

    pub namespaces: Option<crate::kube::apis::networking::gateway::v1::RouteNamespaces>,
}

impl crate::kube::apis::DeepMerge for AllowedRoutes {
    fn merge_from(&mut self, other: Self) {
        crate::kube::apis::merge_strategies::list::atomic(&mut self.kinds, other.kinds);
        crate::kube::apis::DeepMerge::merge_from(&mut self.namespaces, other.namespaces);
    }
}

impl<'de> crate::kube::apis::serde::Deserialize<'de> for AllowedRoutes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: crate::kube::apis::serde::Deserializer<'de> {
        #[allow(non_camel_case_types)]
        enum Field {
            Key_kinds,
            Key_namespaces,
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
                            "kinds" => Field::Key_kinds,
                            "namespaces" => Field::Key_namespaces,
                            _ => Field::Other,
                        })
                    }
                }

                deserializer.deserialize_identifier(Visitor)
            }
        }

        struct Visitor;

        impl<'de> crate::kube::apis::serde::de::Visitor<'de> for Visitor {
            type Value = AllowedRoutes;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("AllowedRoutes")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error> where A: crate::kube::apis::serde::de::MapAccess<'de> {
                let mut value_kinds: Option<Vec<crate::kube::apis::networking::gateway::v1::RouteGroupKind>> = None;
                let mut value_namespaces: Option<crate::kube::apis::networking::gateway::v1::RouteNamespaces> = None;

                while let Some(key) = crate::kube::apis::serde::de::MapAccess::next_key::<Field>(&mut map)? {
                    match key {
                        Field::Key_kinds => value_kinds = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Key_namespaces => value_namespaces = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Other => { let _: crate::kube::apis::serde::de::IgnoredAny = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?; },
                    }
                }

                Ok(AllowedRoutes {
                    kinds: value_kinds,
                    namespaces: value_namespaces,
                })
            }
        }

        deserializer.deserialize_struct(
            "AllowedRoutes",
            &[
                "kinds",
                "namespaces",
            ],
            Visitor,
        )
    }
}

impl crate::kube::apis::serde::Serialize for AllowedRoutes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: crate::kube::apis::serde::Serializer {
        let mut state = serializer.serialize_struct(
            "AllowedRoutes",
            self.kinds.as_ref().map_or(0, |_| 1) +
            self.namespaces.as_ref().map_or(0, |_| 1),
        )?;
        if let Some(value) = &self.kinds {
            crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "kinds", value)?;
        }
        if let Some(value) = &self.namespaces {
            crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "namespaces", value)?;
        }
        crate::kube::apis::serde::ser::SerializeStruct::end(state)
    }
}

#[cfg(feature = "schemars")]
impl crate::kube::apis::schemars::JsonSchema for AllowedRoutes {
    fn schema_name() -> String {
        "io.k8s.networking.gateway.v1.AllowedRoutes".to_owned()
    }

    fn json_schema(__gen: &mut crate::kube::apis::schemars::gen::SchemaGenerator) -> crate::kube::apis::schemars::schema::Schema {
        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                description: Some("AllowedRoutes defines the types of routes that MAY be attached to a Listener and the trusted namespaces where those Route resources MAY be present. \n Although a client request may match multiple route rules, only one rule may ultimately receive the request. Matching precedence MUST be determined in order of the following criteria: \n * The most specific match as defined by the Route type. * The oldest Route based on creation timestamp. For example, a Route with a creation timestamp of \"2020-09-08 01:02:03\" is given precedence over a Route with a creation timestamp of \"2020-09-08 01:02:04\". * If everything else is equivalent, the Route appearing first in alphabetical order (namespace/name) should be given precedence. For example, foo/bar is given precedence over foo/baz. \n All valid rules within a Route attached to this Listener should be implemented. Invalid Route rules can be ignored (sometimes that will mean the full Route). If a Route rule transitions from valid to invalid, support for that Route rule should be dropped to ensure consistency. For example, even if a filter specified by a Route rule is invalid, the rest of the rules within that Route should still be supported. \n Support: Core".to_owned()),
                ..Default::default()
            })),
            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::Object))),
            object: Some(Box::new(crate::kube::apis::schemars::schema::ObjectValidation {
                properties: [
                    (
                        "kinds".to_owned(),
                        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
                            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                                description: Some("Kinds specifies the groups and kinds of Routes that are allowed to bind to this Gateway Listener. When unspecified or empty, the kinds of Routes selected are determined using the Listener protocol. \n A RouteGroupKind MUST correspond to kinds of Routes that are compatible with the application protocol specified in the Listener's Protocol field. If an implementation does not support or recognize this resource type, it MUST set the \"ResolvedRefs\" condition to False for this Listener with the \"InvalidRouteKinds\" reason. \n Support: Core".to_owned()),
                                ..Default::default()
                            })),
                            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::Array))),
                            array: Some(Box::new(crate::kube::apis::schemars::schema::ArrayValidation {
                                items: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(__gen.subschema_for::<crate::kube::apis::networking::gateway::v1::RouteGroupKind>()))),
                                ..Default::default()
                            })),
                            ..Default::default()
                        }),
                    ),
                    (
                        "namespaces".to_owned(),
                        __gen.subschema_for::<crate::kube::apis::networking::gateway::v1::RouteNamespaces>(),
                    ),
                ].into(),
                ..Default::default()
            })),
            ..Default::default()
        })
    }
}
