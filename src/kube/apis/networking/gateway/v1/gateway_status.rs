// Generated from definition io.k8s.networking.gateway.v1.GatewayStatus

/// Status defines the current state of Gateway.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GatewayStatus {
    /// Addresses lists the network addresses that have been bound to the Gateway. 
    ///  This list may differ from the addresses provided in the spec under some conditions: 
    ///  * no addresses are specified, all addresses are dynamically assigned * a combination of specified and dynamic addresses are assigned * a specified address was unusable (e.g. already in use) 
    ///  
    pub addresses: Option<Vec<crate::kube::apis::networking::gateway::v1::GatewayStatusAddress>>,

    /// Conditions describe the current conditions of the Gateway. 
    ///  Implementations should prefer to express Gateway conditions using the `GatewayConditionType` and `GatewayConditionReason` constants so that operators and tools can converge on a common vocabulary to describe Gateway state. 
    ///  Known condition types are: 
    ///  * "Accepted" * "Programmed" * "Ready"
    pub conditions: Option<Vec<crate::kube::apis::apimachinery::pkg::apis::meta::v1::Condition>>,

    /// Listeners provide status for each unique listener port defined in the Spec.
    pub listeners: Option<Vec<crate::kube::apis::networking::gateway::v1::ListenerStatus>>,
}

impl crate::kube::apis::DeepMerge for GatewayStatus {
    fn merge_from(&mut self, other: Self) {
        crate::kube::apis::merge_strategies::list::atomic(&mut self.addresses, other.addresses);
        crate::kube::apis::merge_strategies::list::map(
            &mut self.conditions,
            other.conditions,
            &[|lhs, rhs| lhs.type_ == rhs.type_],
            |current_item, other_item| {
                crate::kube::apis::DeepMerge::merge_from(current_item, other_item);
            },
        );
        crate::kube::apis::merge_strategies::list::map(
            &mut self.listeners,
            other.listeners,
            &[|lhs, rhs| lhs.name == rhs.name],
            |current_item, other_item| {
                crate::kube::apis::DeepMerge::merge_from(current_item, other_item);
            },
        );
    }
}

impl<'de> crate::kube::apis::serde::Deserialize<'de> for GatewayStatus {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: crate::kube::apis::serde::Deserializer<'de> {
        #[allow(non_camel_case_types)]
        enum Field {
            Key_addresses,
            Key_conditions,
            Key_listeners,
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
                            "addresses" => Field::Key_addresses,
                            "conditions" => Field::Key_conditions,
                            "listeners" => Field::Key_listeners,
                            _ => Field::Other,
                        })
                    }
                }

                deserializer.deserialize_identifier(Visitor)
            }
        }

        struct Visitor;

        impl<'de> crate::kube::apis::serde::de::Visitor<'de> for Visitor {
            type Value = GatewayStatus;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("GatewayStatus")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error> where A: crate::kube::apis::serde::de::MapAccess<'de> {
                let mut value_addresses: Option<Vec<crate::kube::apis::networking::gateway::v1::GatewayStatusAddress>> = None;
                let mut value_conditions: Option<Vec<crate::kube::apis::apimachinery::pkg::apis::meta::v1::Condition>> = None;
                let mut value_listeners: Option<Vec<crate::kube::apis::networking::gateway::v1::ListenerStatus>> = None;

                while let Some(key) = crate::kube::apis::serde::de::MapAccess::next_key::<Field>(&mut map)? {
                    match key {
                        Field::Key_addresses => value_addresses = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Key_conditions => value_conditions = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Key_listeners => value_listeners = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Other => { let _: crate::kube::apis::serde::de::IgnoredAny = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?; },
                    }
                }

                Ok(GatewayStatus {
                    addresses: value_addresses,
                    conditions: value_conditions,
                    listeners: value_listeners,
                })
            }
        }

        deserializer.deserialize_struct(
            "GatewayStatus",
            &[
                "addresses",
                "conditions",
                "listeners",
            ],
            Visitor,
        )
    }
}

impl crate::kube::apis::serde::Serialize for GatewayStatus {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: crate::kube::apis::serde::Serializer {
        let mut state = serializer.serialize_struct(
            "GatewayStatus",
            self.addresses.as_ref().map_or(0, |_| 1) +
            self.conditions.as_ref().map_or(0, |_| 1) +
            self.listeners.as_ref().map_or(0, |_| 1),
        )?;
        if let Some(value) = &self.addresses {
            crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "addresses", value)?;
        }
        if let Some(value) = &self.conditions {
            crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "conditions", value)?;
        }
        if let Some(value) = &self.listeners {
            crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "listeners", value)?;
        }
        crate::kube::apis::serde::ser::SerializeStruct::end(state)
    }
}

#[cfg(feature = "schemars")]
impl crate::kube::apis::schemars::JsonSchema for GatewayStatus {
    fn schema_name() -> String {
        "io.k8s.networking.gateway.v1.GatewayStatus".to_owned()
    }

    fn json_schema(__gen: &mut crate::kube::apis::schemars::gen::SchemaGenerator) -> crate::kube::apis::schemars::schema::Schema {
        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                description: Some("Status defines the current state of Gateway.".to_owned()),
                ..Default::default()
            })),
            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::Object))),
            object: Some(Box::new(crate::kube::apis::schemars::schema::ObjectValidation {
                properties: [
                    (
                        "addresses".to_owned(),
                        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
                            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                                description: Some("Addresses lists the network addresses that have been bound to the Gateway. \n This list may differ from the addresses provided in the spec under some conditions: \n * no addresses are specified, all addresses are dynamically assigned * a combination of specified and dynamic addresses are assigned * a specified address was unusable (e.g. already in use) \n ".to_owned()),
                                ..Default::default()
                            })),
                            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::Array))),
                            array: Some(Box::new(crate::kube::apis::schemars::schema::ArrayValidation {
                                items: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(__gen.subschema_for::<crate::kube::apis::networking::gateway::v1::GatewayStatusAddress>()))),
                                ..Default::default()
                            })),
                            ..Default::default()
                        }),
                    ),
                    (
                        "conditions".to_owned(),
                        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
                            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                                description: Some("Conditions describe the current conditions of the Gateway. \n Implementations should prefer to express Gateway conditions using the `GatewayConditionType` and `GatewayConditionReason` constants so that operators and tools can converge on a common vocabulary to describe Gateway state. \n Known condition types are: \n * \"Accepted\" * \"Programmed\" * \"Ready\"".to_owned()),
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
                        "listeners".to_owned(),
                        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
                            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                                description: Some("Listeners provide status for each unique listener port defined in the Spec.".to_owned()),
                                ..Default::default()
                            })),
                            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::Array))),
                            array: Some(Box::new(crate::kube::apis::schemars::schema::ArrayValidation {
                                items: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(__gen.subschema_for::<crate::kube::apis::networking::gateway::v1::ListenerStatus>()))),
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
