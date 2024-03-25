// Generated from definition io.k8s.networking.gateway.v1.GatewayClassSpec

/// Spec defines the desired state of GatewayClass.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GatewayClassSpec {
    /// ControllerName is the name of the controller that is managing Gateways of this class. The value of this field MUST be a domain prefixed path. 
    ///  Example: "example.net/gateway-controller". 
    ///  This field is not mutable and cannot be empty. 
    ///  Support: Core
    pub controller_name: String,

    /// Description helps describe a GatewayClass with more details.
    pub description: Option<String>,

    pub parameters_ref: Option<crate::kube::apis::networking::gateway::v1::ParametersReference>,
}

impl crate::kube::apis::DeepMerge for GatewayClassSpec {
    fn merge_from(&mut self, other: Self) {
        crate::kube::apis::DeepMerge::merge_from(&mut self.controller_name, other.controller_name);
        crate::kube::apis::DeepMerge::merge_from(&mut self.description, other.description);
        crate::kube::apis::DeepMerge::merge_from(&mut self.parameters_ref, other.parameters_ref);
    }
}

impl<'de> crate::kube::apis::serde::Deserialize<'de> for GatewayClassSpec {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: crate::kube::apis::serde::Deserializer<'de> {
        #[allow(non_camel_case_types)]
        enum Field {
            Key_controller_name,
            Key_description,
            Key_parameters_ref,
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
                            "controllerName" => Field::Key_controller_name,
                            "description" => Field::Key_description,
                            "parametersRef" => Field::Key_parameters_ref,
                            _ => Field::Other,
                        })
                    }
                }

                deserializer.deserialize_identifier(Visitor)
            }
        }

        struct Visitor;

        impl<'de> crate::kube::apis::serde::de::Visitor<'de> for Visitor {
            type Value = GatewayClassSpec;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("GatewayClassSpec")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error> where A: crate::kube::apis::serde::de::MapAccess<'de> {
                let mut value_controller_name: Option<String> = None;
                let mut value_description: Option<String> = None;
                let mut value_parameters_ref: Option<crate::kube::apis::networking::gateway::v1::ParametersReference> = None;

                while let Some(key) = crate::kube::apis::serde::de::MapAccess::next_key::<Field>(&mut map)? {
                    match key {
                        Field::Key_controller_name => value_controller_name = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Key_description => value_description = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Key_parameters_ref => value_parameters_ref = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Other => { let _: crate::kube::apis::serde::de::IgnoredAny = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?; },
                    }
                }

                Ok(GatewayClassSpec {
                    controller_name: value_controller_name.unwrap_or_default(),
                    description: value_description,
                    parameters_ref: value_parameters_ref,
                })
            }
        }

        deserializer.deserialize_struct(
            "GatewayClassSpec",
            &[
                "controllerName",
                "description",
                "parametersRef",
            ],
            Visitor,
        )
    }
}

impl crate::kube::apis::serde::Serialize for GatewayClassSpec {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: crate::kube::apis::serde::Serializer {
        let mut state = serializer.serialize_struct(
            "GatewayClassSpec",
            1 +
            self.description.as_ref().map_or(0, |_| 1) +
            self.parameters_ref.as_ref().map_or(0, |_| 1),
        )?;
        crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "controllerName", &self.controller_name)?;
        if let Some(value) = &self.description {
            crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "description", value)?;
        }
        if let Some(value) = &self.parameters_ref {
            crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "parametersRef", value)?;
        }
        crate::kube::apis::serde::ser::SerializeStruct::end(state)
    }
}

#[cfg(feature = "schemars")]
impl crate::kube::apis::schemars::JsonSchema for GatewayClassSpec {
    fn schema_name() -> String {
        "io.k8s.networking.gateway.v1.GatewayClassSpec".to_owned()
    }

    fn json_schema(__gen: &mut crate::kube::apis::schemars::gen::SchemaGenerator) -> crate::kube::apis::schemars::schema::Schema {
        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                description: Some("Spec defines the desired state of GatewayClass.".to_owned()),
                ..Default::default()
            })),
            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::Object))),
            object: Some(Box::new(crate::kube::apis::schemars::schema::ObjectValidation {
                properties: [
                    (
                        "controllerName".to_owned(),
                        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
                            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                                description: Some("ControllerName is the name of the controller that is managing Gateways of this class. The value of this field MUST be a domain prefixed path. \n Example: \"example.net/gateway-controller\". \n This field is not mutable and cannot be empty. \n Support: Core".to_owned()),
                                ..Default::default()
                            })),
                            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::String))),
                            ..Default::default()
                        }),
                    ),
                    (
                        "description".to_owned(),
                        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
                            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                                description: Some("Description helps describe a GatewayClass with more details.".to_owned()),
                                ..Default::default()
                            })),
                            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::String))),
                            ..Default::default()
                        }),
                    ),
                    (
                        "parametersRef".to_owned(),
                        __gen.subschema_for::<crate::kube::apis::networking::gateway::v1::ParametersReference>(),
                    ),
                ].into(),
                required: [
                    "controllerName".to_owned(),
                ].into(),
                ..Default::default()
            })),
            ..Default::default()
        })
    }
}
