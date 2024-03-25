// Generated from definition io.k8s.networking.gateway.v1.GatewaySpec

/// Spec defines the desired state of Gateway.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GatewaySpec {
    pub addresses: Option<crate::kube::apis::networking::gateway::v1::GatewayAddress>,

    /// GatewayClassName used for this Gateway. This is the name of a GatewayClass resource.
    pub gateway_class_name: String,

    /// Listeners associated with this Gateway. Listeners define logical endpoints that are bound on this Gateway's addresses. At least one Listener MUST be specified. 
    ///  Each Listener in a set of Listeners (for example, in a single Gateway) MUST be _distinct_, in that a traffic flow MUST be able to be assigned to exactly one listener. (This section uses "set of Listeners" rather than "Listeners in a single Gateway" because implementations MAY merge configuration from multiple Gateways onto a single data plane, and these rules _also_ apply in that case). 
    ///  Practically, this means that each listener in a set MUST have a unique combination of Port, Protocol, and, if supported by the protocol, Hostname. 
    ///  Some combinations of port, protocol, and TLS settings are considered Core support and MUST be supported by implementations based on their targeted conformance profile: 
    ///  HTTP Profile 
    ///  1. HTTPRoute, Port: 80, Protocol: HTTP 2. HTTPRoute, Port: 443, Protocol: HTTPS, TLS Mode: Terminate, TLS keypair provided 
    ///  TLS Profile 
    ///  1. TLSRoute, Port: 443, Protocol: TLS, TLS Mode: Passthrough 
    ///  "Distinct" Listeners have the following property: 
    ///  The implementation can match inbound requests to a single distinct Listener. When multiple Listeners share values for fields (for example, two Listeners with the same Port value), the implementation can match requests to only one of the Listeners using other Listener fields. 
    ///  For example, the following Listener scenarios are distinct: 
    ///  1. Multiple Listeners with the same Port that all use the "HTTP" Protocol that all have unique Hostname values. 2. Multiple Listeners with the same Port that use either the "HTTPS" or "TLS" Protocol that all have unique Hostname values. 3. A mixture of "TCP" and "UDP" Protocol Listeners, where no Listener with the same Protocol has the same Port value. 
    ///  Some fields in the Listener struct have possible values that affect whether the Listener is distinct. Hostname is particularly relevant for HTTP or HTTPS protocols. 
    ///  When using the Hostname value to select between same-Port, same-Protocol Listeners, the Hostname value must be different on each Listener for the Listener to be distinct. 
    ///  When the Listeners are distinct based on Hostname, inbound request hostnames MUST match from the most specific to least specific Hostname values to choose the correct Listener and its associated set of Routes. 
    ///  Exact matches must be processed before wildcard matches, and wildcard matches must be processed before fallback (empty Hostname value) matches. For example, `"foo.example.com"` takes precedence over `"*.example.com"`, and `"*.example.com"` takes precedence over `""`. 
    ///  Additionally, if there are multiple wildcard entries, more specific wildcard entries must be processed before less specific wildcard entries. For example, `"*.foo.example.com"` takes precedence over `"*.example.com"`. The precise definition here is that the higher the number of dots in the hostname to the right of the wildcard character, the higher the precedence. 
    ///  The wildcard character will match any number of characters _and dots_ to the left, however, so `"*.example.com"` will match both `"foo.bar.example.com"` _and_ `"bar.example.com"`. 
    ///  If a set of Listeners contains Listeners that are not distinct, then those Listeners are Conflicted, and the implementation MUST set the "Conflicted" condition in the Listener Status to "True". 
    ///  Implementations MAY choose to accept a Gateway with some Conflicted Listeners only if they only accept the partial Listener set that contains no Conflicted Listeners. To put this another way, implementations may accept a partial Listener set only if they throw out *all* the conflicting Listeners. No picking one of the conflicting listeners as the winner. This also means that the Gateway must have at least one non-conflicting Listener in this case, otherwise it violates the requirement that at least one Listener must be present. 
    ///  The implementation MUST set a "ListenersNotValid" condition on the Gateway Status when the Gateway contains Conflicted Listeners whether or not they accept the Gateway. That Condition SHOULD clearly indicate in the Message which Listeners are conflicted, and which are Accepted. Additionally, the Listener status for those listeners SHOULD indicate which Listeners are conflicted and not Accepted. 
    ///  A Gateway's Listeners are considered "compatible" if: 
    ///  1. They are distinct. 2. The implementation can serve them in compliance with the Addresses requirement that all Listeners are available on all assigned addresses. 
    ///  Compatible combinations in Extended support are expected to vary across implementations. A combination that is compatible for one implementation may not be compatible for another. 
    ///  For example, an implementation that cannot serve both TCP and UDP listeners on the same address, or cannot mix HTTPS and generic TLS listens on the same port would not consider those cases compatible, even though they are distinct. 
    ///  Note that requests SHOULD match at most one Listener. For example, if Listeners are defined for "foo.example.com" and "*.example.com", a request to "foo.example.com" SHOULD only be routed using routes attached to the "foo.example.com" Listener (and not the "*.example.com" Listener). This concept is known as "Listener Isolation". Implementations that do not support Listener Isolation MUST clearly document this. 
    ///  Implementations MAY merge separate Gateways onto a single set of Addresses if all Listeners across all Gateways are compatible. 
    ///  Support: Core
    pub listeners: Vec<crate::kube::apis::networking::gateway::v1::Listener>,
}

impl crate::kube::apis::DeepMerge for GatewaySpec {
    fn merge_from(&mut self, other: Self) {
        crate::kube::apis::DeepMerge::merge_from(&mut self.addresses, other.addresses);
        crate::kube::apis::DeepMerge::merge_from(&mut self.gateway_class_name, other.gateway_class_name);
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

impl<'de> crate::kube::apis::serde::Deserialize<'de> for GatewaySpec {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: crate::kube::apis::serde::Deserializer<'de> {
        #[allow(non_camel_case_types)]
        enum Field {
            Key_addresses,
            Key_gateway_class_name,
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
                            "gatewayClassName" => Field::Key_gateway_class_name,
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
            type Value = GatewaySpec;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("GatewaySpec")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error> where A: crate::kube::apis::serde::de::MapAccess<'de> {
                let mut value_addresses: Option<crate::kube::apis::networking::gateway::v1::GatewayAddress> = None;
                let mut value_gateway_class_name: Option<String> = None;
                let mut value_listeners: Option<Vec<crate::kube::apis::networking::gateway::v1::Listener>> = None;

                while let Some(key) = crate::kube::apis::serde::de::MapAccess::next_key::<Field>(&mut map)? {
                    match key {
                        Field::Key_addresses => value_addresses = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Key_gateway_class_name => value_gateway_class_name = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Key_listeners => value_listeners = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Other => { let _: crate::kube::apis::serde::de::IgnoredAny = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?; },
                    }
                }

                Ok(GatewaySpec {
                    addresses: value_addresses,
                    gateway_class_name: value_gateway_class_name.unwrap_or_default(),
                    listeners: value_listeners.unwrap_or_default(),
                })
            }
        }

        deserializer.deserialize_struct(
            "GatewaySpec",
            &[
                "addresses",
                "gatewayClassName",
                "listeners",
            ],
            Visitor,
        )
    }
}

impl crate::kube::apis::serde::Serialize for GatewaySpec {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: crate::kube::apis::serde::Serializer {
        let mut state = serializer.serialize_struct(
            "GatewaySpec",
            2 +
            self.addresses.as_ref().map_or(0, |_| 1),
        )?;
        if let Some(value) = &self.addresses {
            crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "addresses", value)?;
        }
        crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "gatewayClassName", &self.gateway_class_name)?;
        crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "listeners", &self.listeners)?;
        crate::kube::apis::serde::ser::SerializeStruct::end(state)
    }
}

#[cfg(feature = "schemars")]
impl crate::kube::apis::schemars::JsonSchema for GatewaySpec {
    fn schema_name() -> String {
        "io.k8s.networking.gateway.v1.GatewaySpec".to_owned()
    }

    fn json_schema(__gen: &mut crate::kube::apis::schemars::gen::SchemaGenerator) -> crate::kube::apis::schemars::schema::Schema {
        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                description: Some("Spec defines the desired state of Gateway.".to_owned()),
                ..Default::default()
            })),
            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::Object))),
            object: Some(Box::new(crate::kube::apis::schemars::schema::ObjectValidation {
                properties: [
                    (
                        "addresses".to_owned(),
                        __gen.subschema_for::<crate::kube::apis::networking::gateway::v1::GatewayAddress>(),
                    ),
                    (
                        "gatewayClassName".to_owned(),
                        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
                            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                                description: Some("GatewayClassName used for this Gateway. This is the name of a GatewayClass resource.".to_owned()),
                                ..Default::default()
                            })),
                            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::String))),
                            ..Default::default()
                        }),
                    ),
                    (
                        "listeners".to_owned(),
                        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
                            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                                description: Some("Listeners associated with this Gateway. Listeners define logical endpoints that are bound on this Gateway's addresses. At least one Listener MUST be specified. \n Each Listener in a set of Listeners (for example, in a single Gateway) MUST be _distinct_, in that a traffic flow MUST be able to be assigned to exactly one listener. (This section uses \"set of Listeners\" rather than \"Listeners in a single Gateway\" because implementations MAY merge configuration from multiple Gateways onto a single data plane, and these rules _also_ apply in that case). \n Practically, this means that each listener in a set MUST have a unique combination of Port, Protocol, and, if supported by the protocol, Hostname. \n Some combinations of port, protocol, and TLS settings are considered Core support and MUST be supported by implementations based on their targeted conformance profile: \n HTTP Profile \n 1. HTTPRoute, Port: 80, Protocol: HTTP 2. HTTPRoute, Port: 443, Protocol: HTTPS, TLS Mode: Terminate, TLS keypair provided \n TLS Profile \n 1. TLSRoute, Port: 443, Protocol: TLS, TLS Mode: Passthrough \n \"Distinct\" Listeners have the following property: \n The implementation can match inbound requests to a single distinct Listener. When multiple Listeners share values for fields (for example, two Listeners with the same Port value), the implementation can match requests to only one of the Listeners using other Listener fields. \n For example, the following Listener scenarios are distinct: \n 1. Multiple Listeners with the same Port that all use the \"HTTP\" Protocol that all have unique Hostname values. 2. Multiple Listeners with the same Port that use either the \"HTTPS\" or \"TLS\" Protocol that all have unique Hostname values. 3. A mixture of \"TCP\" and \"UDP\" Protocol Listeners, where no Listener with the same Protocol has the same Port value. \n Some fields in the Listener struct have possible values that affect whether the Listener is distinct. Hostname is particularly relevant for HTTP or HTTPS protocols. \n When using the Hostname value to select between same-Port, same-Protocol Listeners, the Hostname value must be different on each Listener for the Listener to be distinct. \n When the Listeners are distinct based on Hostname, inbound request hostnames MUST match from the most specific to least specific Hostname values to choose the correct Listener and its associated set of Routes. \n Exact matches must be processed before wildcard matches, and wildcard matches must be processed before fallback (empty Hostname value) matches. For example, `\"foo.example.com\"` takes precedence over `\"*.example.com\"`, and `\"*.example.com\"` takes precedence over `\"\"`. \n Additionally, if there are multiple wildcard entries, more specific wildcard entries must be processed before less specific wildcard entries. For example, `\"*.foo.example.com\"` takes precedence over `\"*.example.com\"`. The precise definition here is that the higher the number of dots in the hostname to the right of the wildcard character, the higher the precedence. \n The wildcard character will match any number of characters _and dots_ to the left, however, so `\"*.example.com\"` will match both `\"foo.bar.example.com\"` _and_ `\"bar.example.com\"`. \n If a set of Listeners contains Listeners that are not distinct, then those Listeners are Conflicted, and the implementation MUST set the \"Conflicted\" condition in the Listener Status to \"True\". \n Implementations MAY choose to accept a Gateway with some Conflicted Listeners only if they only accept the partial Listener set that contains no Conflicted Listeners. To put this another way, implementations may accept a partial Listener set only if they throw out *all* the conflicting Listeners. No picking one of the conflicting listeners as the winner. This also means that the Gateway must have at least one non-conflicting Listener in this case, otherwise it violates the requirement that at least one Listener must be present. \n The implementation MUST set a \"ListenersNotValid\" condition on the Gateway Status when the Gateway contains Conflicted Listeners whether or not they accept the Gateway. That Condition SHOULD clearly indicate in the Message which Listeners are conflicted, and which are Accepted. Additionally, the Listener status for those listeners SHOULD indicate which Listeners are conflicted and not Accepted. \n A Gateway's Listeners are considered \"compatible\" if: \n 1. They are distinct. 2. The implementation can serve them in compliance with the Addresses requirement that all Listeners are available on all assigned addresses. \n Compatible combinations in Extended support are expected to vary across implementations. A combination that is compatible for one implementation may not be compatible for another. \n For example, an implementation that cannot serve both TCP and UDP listeners on the same address, or cannot mix HTTPS and generic TLS listens on the same port would not consider those cases compatible, even though they are distinct. \n Note that requests SHOULD match at most one Listener. For example, if Listeners are defined for \"foo.example.com\" and \"*.example.com\", a request to \"foo.example.com\" SHOULD only be routed using routes attached to the \"foo.example.com\" Listener (and not the \"*.example.com\" Listener). This concept is known as \"Listener Isolation\". Implementations that do not support Listener Isolation MUST clearly document this. \n Implementations MAY merge separate Gateways onto a single set of Addresses if all Listeners across all Gateways are compatible. \n Support: Core".to_owned()),
                                ..Default::default()
                            })),
                            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::Array))),
                            array: Some(Box::new(crate::kube::apis::schemars::schema::ArrayValidation {
                                items: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(__gen.subschema_for::<crate::kube::apis::networking::gateway::v1::Listener>()))),
                                ..Default::default()
                            })),
                            ..Default::default()
                        }),
                    ),
                ].into(),
                required: [
                    "gatewayClassName".to_owned(),
                    "listeners".to_owned(),
                ].into(),
                ..Default::default()
            })),
            ..Default::default()
        })
    }
}
