// Generated from definition io.k8s.networking.gateway.v1.HTTPPathModifier

/// Path defines parameters used to modify the path of the incoming request. The modified path is then used to construct the `Location` header. When empty, the request path is used as-is. 
///  Support: Extended
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HTTPPathModifier {
    /// ReplaceFullPath specifies the value with which to replace the full path of a request during a rewrite or redirect.
    pub replace_full_path: Option<String>,

    /// ReplacePrefixMatch specifies the value with which to replace the prefix match of a request during a rewrite or redirect. For example, a request to "/foo/bar" with a prefix match of "/foo" and a ReplacePrefixMatch of "/xyz" would be modified to "/xyz/bar". 
    ///  Note that this matches the behavior of the PathPrefix match type. This matches full path elements. A path element refers to the list of labels in the path split by the `/` separator. When specified, a trailing `/` is ignored. For example, the paths `/abc`, `/abc/`, and `/abc/def` would all match the prefix `/abc`, but the path `/abcd` would not. 
    ///  ReplacePrefixMatch is only compatible with a `PathPrefix` HTTPRouteMatch. Using any other HTTPRouteMatch type on the same HTTPRouteRule will result in the implementation setting the Accepted Condition for the Route to `status: False`. 
    ///  Request Path | Prefix Match | Replace Prefix | Modified Path -------------|--------------|----------------|---------- /foo/bar     | /foo         | /xyz           | /xyz/bar /foo/bar     | /foo         | /xyz/          | /xyz/bar /foo/bar     | /foo/        | /xyz           | /xyz/bar /foo/bar     | /foo/        | /xyz/          | /xyz/bar /foo         | /foo         | /xyz           | /xyz /foo/        | /foo         | /xyz           | /xyz/ /foo/bar     | /foo         | \<empty string\> | /bar /foo/        | /foo         | \<empty string\> | / /foo         | /foo         | \<empty string\> | / /foo/        | /foo         | /              | / /foo         | /foo         | /              | /
    pub replace_prefix_match: Option<String>,

    /// Type defines the type of path modifier. Additional types may be added in a future release of the API. 
    ///  Note that values may be added to this enum, implementations must ensure that unknown values will not cause a crash. 
    ///  Unknown values here must result in the implementation setting the Accepted Condition for the Route to `status: False`, with a Reason of `UnsupportedValue`.
    pub type_: String,
}

impl crate::kube::apis::DeepMerge for HTTPPathModifier {
    fn merge_from(&mut self, other: Self) {
        crate::kube::apis::DeepMerge::merge_from(&mut self.replace_full_path, other.replace_full_path);
        crate::kube::apis::DeepMerge::merge_from(&mut self.replace_prefix_match, other.replace_prefix_match);
        crate::kube::apis::DeepMerge::merge_from(&mut self.type_, other.type_);
    }
}

impl<'de> crate::kube::apis::serde::Deserialize<'de> for HTTPPathModifier {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: crate::kube::apis::serde::Deserializer<'de> {
        #[allow(non_camel_case_types)]
        enum Field {
            Key_replace_full_path,
            Key_replace_prefix_match,
            Key_type_,
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
                            "replaceFullPath" => Field::Key_replace_full_path,
                            "replacePrefixMatch" => Field::Key_replace_prefix_match,
                            "type" => Field::Key_type_,
                            _ => Field::Other,
                        })
                    }
                }

                deserializer.deserialize_identifier(Visitor)
            }
        }

        struct Visitor;

        impl<'de> crate::kube::apis::serde::de::Visitor<'de> for Visitor {
            type Value = HTTPPathModifier;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("HTTPPathModifier")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error> where A: crate::kube::apis::serde::de::MapAccess<'de> {
                let mut value_replace_full_path: Option<String> = None;
                let mut value_replace_prefix_match: Option<String> = None;
                let mut value_type_: Option<String> = None;

                while let Some(key) = crate::kube::apis::serde::de::MapAccess::next_key::<Field>(&mut map)? {
                    match key {
                        Field::Key_replace_full_path => value_replace_full_path = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Key_replace_prefix_match => value_replace_prefix_match = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Key_type_ => value_type_ = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Other => { let _: crate::kube::apis::serde::de::IgnoredAny = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?; },
                    }
                }

                Ok(HTTPPathModifier {
                    replace_full_path: value_replace_full_path,
                    replace_prefix_match: value_replace_prefix_match,
                    type_: value_type_.unwrap_or_default(),
                })
            }
        }

        deserializer.deserialize_struct(
            "HTTPPathModifier",
            &[
                "replaceFullPath",
                "replacePrefixMatch",
                "type",
            ],
            Visitor,
        )
    }
}

impl crate::kube::apis::serde::Serialize for HTTPPathModifier {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: crate::kube::apis::serde::Serializer {
        let mut state = serializer.serialize_struct(
            "HTTPPathModifier",
            1 +
            self.replace_full_path.as_ref().map_or(0, |_| 1) +
            self.replace_prefix_match.as_ref().map_or(0, |_| 1),
        )?;
        if let Some(value) = &self.replace_full_path {
            crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "replaceFullPath", value)?;
        }
        if let Some(value) = &self.replace_prefix_match {
            crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "replacePrefixMatch", value)?;
        }
        crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "type", &self.type_)?;
        crate::kube::apis::serde::ser::SerializeStruct::end(state)
    }
}

#[cfg(feature = "schemars")]
impl crate::kube::apis::schemars::JsonSchema for HTTPPathModifier {
    fn schema_name() -> String {
        "io.k8s.networking.gateway.v1.HTTPPathModifier".to_owned()
    }

    fn json_schema(__gen: &mut crate::kube::apis::schemars::gen::SchemaGenerator) -> crate::kube::apis::schemars::schema::Schema {
        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                description: Some("Path defines parameters used to modify the path of the incoming request. The modified path is then used to construct the `Location` header. When empty, the request path is used as-is. \n Support: Extended".to_owned()),
                ..Default::default()
            })),
            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::Object))),
            object: Some(Box::new(crate::kube::apis::schemars::schema::ObjectValidation {
                properties: [
                    (
                        "replaceFullPath".to_owned(),
                        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
                            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                                description: Some("ReplaceFullPath specifies the value with which to replace the full path of a request during a rewrite or redirect.".to_owned()),
                                ..Default::default()
                            })),
                            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::String))),
                            ..Default::default()
                        }),
                    ),
                    (
                        "replacePrefixMatch".to_owned(),
                        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
                            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                                description: Some("ReplacePrefixMatch specifies the value with which to replace the prefix match of a request during a rewrite or redirect. For example, a request to \"/foo/bar\" with a prefix match of \"/foo\" and a ReplacePrefixMatch of \"/xyz\" would be modified to \"/xyz/bar\". \n Note that this matches the behavior of the PathPrefix match type. This matches full path elements. A path element refers to the list of labels in the path split by the `/` separator. When specified, a trailing `/` is ignored. For example, the paths `/abc`, `/abc/`, and `/abc/def` would all match the prefix `/abc`, but the path `/abcd` would not. \n ReplacePrefixMatch is only compatible with a `PathPrefix` HTTPRouteMatch. Using any other HTTPRouteMatch type on the same HTTPRouteRule will result in the implementation setting the Accepted Condition for the Route to `status: False`. \n Request Path | Prefix Match | Replace Prefix | Modified Path -------------|--------------|----------------|---------- /foo/bar     | /foo         | /xyz           | /xyz/bar /foo/bar     | /foo         | /xyz/          | /xyz/bar /foo/bar     | /foo/        | /xyz           | /xyz/bar /foo/bar     | /foo/        | /xyz/          | /xyz/bar /foo         | /foo         | /xyz           | /xyz /foo/        | /foo         | /xyz           | /xyz/ /foo/bar     | /foo         | <empty string> | /bar /foo/        | /foo         | <empty string> | / /foo         | /foo         | <empty string> | / /foo/        | /foo         | /              | / /foo         | /foo         | /              | /".to_owned()),
                                ..Default::default()
                            })),
                            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::String))),
                            ..Default::default()
                        }),
                    ),
                    (
                        "type".to_owned(),
                        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
                            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                                description: Some("Type defines the type of path modifier. Additional types may be added in a future release of the API. \n Note that values may be added to this enum, implementations must ensure that unknown values will not cause a crash. \n Unknown values here must result in the implementation setting the Accepted Condition for the Route to `status: False`, with a Reason of `UnsupportedValue`.".to_owned()),
                                ..Default::default()
                            })),
                            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::String))),
                            ..Default::default()
                        }),
                    ),
                ].into(),
                required: [
                    "type".to_owned(),
                ].into(),
                ..Default::default()
            })),
            ..Default::default()
        })
    }
}
