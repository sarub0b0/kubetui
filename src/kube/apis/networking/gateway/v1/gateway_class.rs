// Generated from definition io.k8s.networking.gateway.v1.GatewayClass

/// GatewayClass describes a class of Gateways available to the user for creating Gateway resources. 
///  It is recommended that this resource be used as a template for Gateways. This means that a Gateway is based on the state of the GatewayClass at the time it was created and changes to the GatewayClass or associated parameters are not propagated down to existing Gateways. This recommendation is intended to limit the blast radius of changes to GatewayClass or associated parameters. If implementations choose to propagate GatewayClass changes to existing Gateways, that MUST be clearly documented by the implementation. 
///  Whenever one or more Gateways are using a GatewayClass, implementations SHOULD add the `gateway-exists-finalizer.gateway.networking.k8s.io` finalizer on the associated GatewayClass. This ensures that a GatewayClass associated with a Gateway is not deleted while in use. 
///  GatewayClass is a Cluster level resource.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GatewayClass {
    /// Standard object's metadata. More info: https://git.k8s.io/community/contributors/devel/sig-architecture/api-conventions.md#metadata
    pub metadata: crate::kube::apis::apimachinery::pkg::apis::meta::v1::ObjectMeta,

    pub spec: crate::kube::apis::networking::gateway::v1::GatewayClassSpec,

    pub status: Option<crate::kube::apis::networking::gateway::v1::GatewayClassStatus>,
}

impl crate::kube::apis::Resource for GatewayClass {
    const API_VERSION: &'static str = "gateway.networking.k8s.io/v1";
    const GROUP: &'static str = "gateway.networking.k8s.io";
    const KIND: &'static str = "GatewayClass";
    const VERSION: &'static str = "v1";
    const URL_PATH_SEGMENT: &'static str = "gatewayclasses";
    type Scope = crate::kube::apis::ClusterResourceScope;
}

impl crate::kube::apis::ListableResource for GatewayClass {
    const LIST_KIND: &'static str = "GatewayClassList";
}

impl crate::kube::apis::Metadata for GatewayClass {
    type Ty = crate::kube::apis::apimachinery::pkg::apis::meta::v1::ObjectMeta;

    fn metadata(&self) -> &<Self as crate::kube::apis::Metadata>::Ty {
        &self.metadata
    }

    fn metadata_mut(&mut self) -> &mut<Self as crate::kube::apis::Metadata>::Ty {
        &mut self.metadata
    }
}

impl crate::kube::apis::DeepMerge for GatewayClass {
    fn merge_from(&mut self, other: Self) {
        crate::kube::apis::DeepMerge::merge_from(&mut self.metadata, other.metadata);
        crate::kube::apis::DeepMerge::merge_from(&mut self.spec, other.spec);
        crate::kube::apis::DeepMerge::merge_from(&mut self.status, other.status);
    }
}

impl<'de> crate::kube::apis::serde::Deserialize<'de> for GatewayClass {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: crate::kube::apis::serde::Deserializer<'de> {
        #[allow(non_camel_case_types)]
        enum Field {
            Key_api_version,
            Key_kind,
            Key_metadata,
            Key_spec,
            Key_status,
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
                            "apiVersion" => Field::Key_api_version,
                            "kind" => Field::Key_kind,
                            "metadata" => Field::Key_metadata,
                            "spec" => Field::Key_spec,
                            "status" => Field::Key_status,
                            _ => Field::Other,
                        })
                    }
                }

                deserializer.deserialize_identifier(Visitor)
            }
        }

        struct Visitor;

        impl<'de> crate::kube::apis::serde::de::Visitor<'de> for Visitor {
            type Value = GatewayClass;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(<Self::Value as crate::kube::apis::Resource>::KIND)
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error> where A: crate::kube::apis::serde::de::MapAccess<'de> {
                let mut value_metadata: Option<crate::kube::apis::apimachinery::pkg::apis::meta::v1::ObjectMeta> = None;
                let mut value_spec: Option<crate::kube::apis::networking::gateway::v1::GatewayClassSpec> = None;
                let mut value_status: Option<crate::kube::apis::networking::gateway::v1::GatewayClassStatus> = None;

                while let Some(key) = crate::kube::apis::serde::de::MapAccess::next_key::<Field>(&mut map)? {
                    match key {
                        Field::Key_api_version => {
                            let value_api_version: String = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?;
                            if value_api_version != <Self::Value as crate::kube::apis::Resource>::API_VERSION {
                                return Err(crate::kube::apis::serde::de::Error::invalid_value(crate::kube::apis::serde::de::Unexpected::Str(&value_api_version), &<Self::Value as crate::kube::apis::Resource>::API_VERSION));
                            }
                        },
                        Field::Key_kind => {
                            let value_kind: String = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?;
                            if value_kind != <Self::Value as crate::kube::apis::Resource>::KIND {
                                return Err(crate::kube::apis::serde::de::Error::invalid_value(crate::kube::apis::serde::de::Unexpected::Str(&value_kind), &<Self::Value as crate::kube::apis::Resource>::KIND));
                            }
                        },
                        Field::Key_metadata => value_metadata = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Key_spec => value_spec = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Key_status => value_status = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Other => { let _: crate::kube::apis::serde::de::IgnoredAny = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?; },
                    }
                }

                Ok(GatewayClass {
                    metadata: value_metadata.unwrap_or_default(),
                    spec: value_spec.unwrap_or_default(),
                    status: value_status,
                })
            }
        }

        deserializer.deserialize_struct(
            <Self as crate::kube::apis::Resource>::KIND,
            &[
                "apiVersion",
                "kind",
                "metadata",
                "spec",
                "status",
            ],
            Visitor,
        )
    }
}

impl crate::kube::apis::serde::Serialize for GatewayClass {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: crate::kube::apis::serde::Serializer {
        let mut state = serializer.serialize_struct(
            <Self as crate::kube::apis::Resource>::KIND,
            4 +
            self.status.as_ref().map_or(0, |_| 1),
        )?;
        crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "apiVersion", <Self as crate::kube::apis::Resource>::API_VERSION)?;
        crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "kind", <Self as crate::kube::apis::Resource>::KIND)?;
        crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "metadata", &self.metadata)?;
        crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "spec", &self.spec)?;
        if let Some(value) = &self.status {
            crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "status", value)?;
        }
        crate::kube::apis::serde::ser::SerializeStruct::end(state)
    }
}

#[cfg(feature = "schemars")]
impl crate::kube::apis::schemars::JsonSchema for GatewayClass {
    fn schema_name() -> String {
        "io.k8s.networking.gateway.v1.GatewayClass".to_owned()
    }

    fn json_schema(__gen: &mut crate::kube::apis::schemars::gen::SchemaGenerator) -> crate::kube::apis::schemars::schema::Schema {
        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                description: Some("GatewayClass describes a class of Gateways available to the user for creating Gateway resources. \n It is recommended that this resource be used as a template for Gateways. This means that a Gateway is based on the state of the GatewayClass at the time it was created and changes to the GatewayClass or associated parameters are not propagated down to existing Gateways. This recommendation is intended to limit the blast radius of changes to GatewayClass or associated parameters. If implementations choose to propagate GatewayClass changes to existing Gateways, that MUST be clearly documented by the implementation. \n Whenever one or more Gateways are using a GatewayClass, implementations SHOULD add the `gateway-exists-finalizer.gateway.networking.k8s.io` finalizer on the associated GatewayClass. This ensures that a GatewayClass associated with a Gateway is not deleted while in use. \n GatewayClass is a Cluster level resource.".to_owned()),
                ..Default::default()
            })),
            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::Object))),
            object: Some(Box::new(crate::kube::apis::schemars::schema::ObjectValidation {
                properties: [
                    (
                        "apiVersion".to_owned(),
                        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
                            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                                description: Some("APIVersion defines the versioned schema of this representation of an object. Servers should convert recognized schemas to the latest internal value, and may reject unrecognized values. More info: https://git.k8s.io/community/contributors/devel/sig-architecture/api-conventions.md#resources".to_owned()),
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
                                description: Some("Kind is a string value representing the REST resource this object represents. Servers may infer this from the endpoint the client submits requests to. Cannot be updated. In CamelCase. More info: https://git.k8s.io/community/contributors/devel/sig-architecture/api-conventions.md#types-kinds".to_owned()),
                                ..Default::default()
                            })),
                            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::String))),
                            ..Default::default()
                        }),
                    ),
                    (
                        "metadata".to_owned(),
                        {
                            let mut schema_obj = __gen.subschema_for::<crate::kube::apis::apimachinery::pkg::apis::meta::v1::ObjectMeta>().into_object();
                            schema_obj.metadata = Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                                description: Some("Standard object's metadata. More info: https://git.k8s.io/community/contributors/devel/sig-architecture/api-conventions.md#metadata".to_owned()),
                                ..Default::default()
                            }));
                            crate::kube::apis::schemars::schema::Schema::Object(schema_obj)
                        },
                    ),
                    (
                        "spec".to_owned(),
                        __gen.subschema_for::<crate::kube::apis::networking::gateway::v1::GatewayClassSpec>(),
                    ),
                    (
                        "status".to_owned(),
                        __gen.subschema_for::<crate::kube::apis::networking::gateway::v1::GatewayClassStatus>(),
                    ),
                ].into(),
                required: [
                    "metadata".to_owned(),
                    "spec".to_owned(),
                ].into(),
                ..Default::default()
            })),
            ..Default::default()
        })
    }
}
