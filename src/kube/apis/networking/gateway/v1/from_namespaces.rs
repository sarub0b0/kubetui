// Generated from definition io.k8s.networking.gateway.v1.FromNamespaces

/// From indicates where Routes will be selected for this Gateway. Possible values are: 
///  * All: Routes in all namespaces may be used by this Gateway. * Selector: Routes in namespaces selected by the selector may be used by this Gateway. * Same: Only Routes in the same namespace may be used by this Gateway. 
///  Support: Core
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FromNamespaces(pub String);

impl crate::kube::apis::DeepMerge for FromNamespaces {
    fn merge_from(&mut self, other: Self) {
        crate::kube::apis::DeepMerge::merge_from(&mut self.0, other.0);
    }
}

impl<'de> crate::kube::apis::serde::Deserialize<'de> for FromNamespaces {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: crate::kube::apis::serde::Deserializer<'de> {
        struct Visitor;

        impl<'de> crate::kube::apis::serde::de::Visitor<'de> for Visitor {
            type Value = FromNamespaces;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("FromNamespaces")
            }

            fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error> where D: crate::kube::apis::serde::Deserializer<'de> {
                Ok(FromNamespaces(crate::kube::apis::serde::Deserialize::deserialize(deserializer)?))
            }
        }

        deserializer.deserialize_newtype_struct("FromNamespaces", Visitor)
    }
}

impl crate::kube::apis::serde::Serialize for FromNamespaces {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: crate::kube::apis::serde::Serializer {
        serializer.serialize_newtype_struct("FromNamespaces", &self.0)
    }
}

#[cfg(feature = "schemars")]
impl crate::kube::apis::schemars::JsonSchema for FromNamespaces {
    fn schema_name() -> String {
        "io.k8s.networking.gateway.v1.FromNamespaces".to_owned()
    }

    fn json_schema(__gen: &mut crate::kube::apis::schemars::gen::SchemaGenerator) -> crate::kube::apis::schemars::schema::Schema {
        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                description: Some("From indicates where Routes will be selected for this Gateway. Possible values are: \n * All: Routes in all namespaces may be used by this Gateway. * Selector: Routes in namespaces selected by the selector may be used by this Gateway. * Same: Only Routes in the same namespace may be used by this Gateway. \n Support: Core".to_owned()),
                ..Default::default()
            })),
            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::String))),
            ..Default::default()
        })
    }
}
