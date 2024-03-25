// Generated from definition io.k8s.networking.gateway.v1.RouteParentStatus

/// RouteParentStatus describes the status of a route with respect to an associated Parent.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RouteParentStatus {
    /// Conditions describes the status of the route with respect to the Gateway. Note that the route's availability is also subject to the Gateway's own status conditions and listener status. 
    ///  If the Route's ParentRef specifies an existing Gateway that supports Routes of this kind AND that Gateway's controller has sufficient access, then that Gateway's controller MUST set the "Accepted" condition on the Route, to indicate whether the route has been accepted or rejected by the Gateway, and why. 
    ///  A Route MUST be considered "Accepted" if at least one of the Route's rules is implemented by the Gateway. 
    ///  There are a number of cases where the "Accepted" condition may not be set due to lack of controller visibility, that includes when: 
    ///  * The Route refers to a non-existent parent. * The Route is of a type that the controller does not support. * The Route is in a namespace the controller does not have access to.
    pub conditions: Option<Vec<crate::kube::apis::apimachinery::pkg::apis::meta::v1::Condition>>,

    /// ControllerName is a domain/path string that indicates the name of the controller that wrote this status. This corresponds with the controllerName field on GatewayClass. 
    ///  Example: "example.net/gateway-controller". 
    ///  The format of this field is DOMAIN "/" PATH, where DOMAIN and PATH are valid Kubernetes names (https://kubernetes.io/docs/concepts/overview/working-with-objects/names/#names). 
    ///  Controllers MUST populate this field when writing status. Controllers should ensure that entries to status populated with their ControllerName are cleaned up when they are no longer necessary.
    pub controller_name: String,

    pub parent_ref: crate::kube::apis::networking::gateway::v1::ParentReference,
}

impl crate::kube::apis::DeepMerge for RouteParentStatus {
    fn merge_from(&mut self, other: Self) {
        crate::kube::apis::merge_strategies::list::map(
            &mut self.conditions,
            other.conditions,
            &[|lhs, rhs| lhs.type_ == rhs.type_],
            |current_item, other_item| {
                crate::kube::apis::DeepMerge::merge_from(current_item, other_item);
            },
        );
        crate::kube::apis::DeepMerge::merge_from(&mut self.controller_name, other.controller_name);
        crate::kube::apis::DeepMerge::merge_from(&mut self.parent_ref, other.parent_ref);
    }
}

impl<'de> crate::kube::apis::serde::Deserialize<'de> for RouteParentStatus {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: crate::kube::apis::serde::Deserializer<'de> {
        #[allow(non_camel_case_types)]
        enum Field {
            Key_conditions,
            Key_controller_name,
            Key_parent_ref,
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
                            "conditions" => Field::Key_conditions,
                            "controllerName" => Field::Key_controller_name,
                            "parentRef" => Field::Key_parent_ref,
                            _ => Field::Other,
                        })
                    }
                }

                deserializer.deserialize_identifier(Visitor)
            }
        }

        struct Visitor;

        impl<'de> crate::kube::apis::serde::de::Visitor<'de> for Visitor {
            type Value = RouteParentStatus;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("RouteParentStatus")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error> where A: crate::kube::apis::serde::de::MapAccess<'de> {
                let mut value_conditions: Option<Vec<crate::kube::apis::apimachinery::pkg::apis::meta::v1::Condition>> = None;
                let mut value_controller_name: Option<String> = None;
                let mut value_parent_ref: Option<crate::kube::apis::networking::gateway::v1::ParentReference> = None;

                while let Some(key) = crate::kube::apis::serde::de::MapAccess::next_key::<Field>(&mut map)? {
                    match key {
                        Field::Key_conditions => value_conditions = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Key_controller_name => value_controller_name = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Key_parent_ref => value_parent_ref = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Other => { let _: crate::kube::apis::serde::de::IgnoredAny = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?; },
                    }
                }

                Ok(RouteParentStatus {
                    conditions: value_conditions,
                    controller_name: value_controller_name.unwrap_or_default(),
                    parent_ref: value_parent_ref.unwrap_or_default(),
                })
            }
        }

        deserializer.deserialize_struct(
            "RouteParentStatus",
            &[
                "conditions",
                "controllerName",
                "parentRef",
            ],
            Visitor,
        )
    }
}

impl crate::kube::apis::serde::Serialize for RouteParentStatus {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: crate::kube::apis::serde::Serializer {
        let mut state = serializer.serialize_struct(
            "RouteParentStatus",
            2 +
            self.conditions.as_ref().map_or(0, |_| 1),
        )?;
        if let Some(value) = &self.conditions {
            crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "conditions", value)?;
        }
        crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "controllerName", &self.controller_name)?;
        crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "parentRef", &self.parent_ref)?;
        crate::kube::apis::serde::ser::SerializeStruct::end(state)
    }
}

#[cfg(feature = "schemars")]
impl crate::kube::apis::schemars::JsonSchema for RouteParentStatus {
    fn schema_name() -> String {
        "io.k8s.networking.gateway.v1.RouteParentStatus".to_owned()
    }

    fn json_schema(__gen: &mut crate::kube::apis::schemars::gen::SchemaGenerator) -> crate::kube::apis::schemars::schema::Schema {
        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                description: Some("RouteParentStatus describes the status of a route with respect to an associated Parent.".to_owned()),
                ..Default::default()
            })),
            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::Object))),
            object: Some(Box::new(crate::kube::apis::schemars::schema::ObjectValidation {
                properties: [
                    (
                        "conditions".to_owned(),
                        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
                            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                                description: Some("Conditions describes the status of the route with respect to the Gateway. Note that the route's availability is also subject to the Gateway's own status conditions and listener status. \n If the Route's ParentRef specifies an existing Gateway that supports Routes of this kind AND that Gateway's controller has sufficient access, then that Gateway's controller MUST set the \"Accepted\" condition on the Route, to indicate whether the route has been accepted or rejected by the Gateway, and why. \n A Route MUST be considered \"Accepted\" if at least one of the Route's rules is implemented by the Gateway. \n There are a number of cases where the \"Accepted\" condition may not be set due to lack of controller visibility, that includes when: \n * The Route refers to a non-existent parent. * The Route is of a type that the controller does not support. * The Route is in a namespace the controller does not have access to.".to_owned()),
                                ..Default::default()
                            })),
                            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::Array))),
                            array: Some(Box::new(crate::kube::apis::schemars::schema::ArrayValidation {
                                items: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(__gen.subschema_for::<crate::kube::apis::apimachinery::pkg::apis::meta::v1::Condition>()))),
                                ..Default::default()
                            })),
                            ..Default::default()
                        }),
                    ),
                    (
                        "controllerName".to_owned(),
                        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
                            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                                description: Some("ControllerName is a domain/path string that indicates the name of the controller that wrote this status. This corresponds with the controllerName field on GatewayClass. \n Example: \"example.net/gateway-controller\". \n The format of this field is DOMAIN \"/\" PATH, where DOMAIN and PATH are valid Kubernetes names (https://kubernetes.io/docs/concepts/overview/working-with-objects/names/#names). \n Controllers MUST populate this field when writing status. Controllers should ensure that entries to status populated with their ControllerName are cleaned up when they are no longer necessary.".to_owned()),
                                ..Default::default()
                            })),
                            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::String))),
                            ..Default::default()
                        }),
                    ),
                    (
                        "parentRef".to_owned(),
                        __gen.subschema_for::<crate::kube::apis::networking::gateway::v1::ParentReference>(),
                    ),
                ].into(),
                required: [
                    "controllerName".to_owned(),
                    "parentRef".to_owned(),
                ].into(),
                ..Default::default()
            })),
            ..Default::default()
        })
    }
}
