// Generated from definition io.k8s.networking.gateway.v1.HTTPRouteMatch

/// HTTPRouteMatch defines the predicate used to match requests to a given action. Multiple match types are ANDed together, i.e. the match will evaluate to true only if all conditions are satisfied. 
///  For example, the match below will match a HTTP request only if its path starts with `/foo` AND it contains the `version: v1` header: 
///  ``` match: 
///  path: value: "/foo" headers: - name: "version" value "v1" 
///  ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HTTPRouteMatch {
    /// Headers specifies HTTP request header matchers. Multiple match values are ANDed together, meaning, a request must match all the specified headers to select the route.
    pub headers: Option<Vec<crate::kube::apis::networking::gateway::v1::HTTPHeaderMatch>>,

    /// Method specifies HTTP method matcher. When specified, this route will be matched only if the request has the specified method. 
    ///  Support: Extended
    pub method: Option<String>,

    pub path: Option<crate::kube::apis::networking::gateway::v1::HTTPPathMatch>,

    /// QueryParams specifies HTTP query parameter matchers. Multiple match values are ANDed together, meaning, a request must match all the specified query parameters to select the route. 
    ///  Support: Extended
    pub query_params: Option<Vec<crate::kube::apis::networking::gateway::v1::HTTPQueryParamMatch>>,
}

impl crate::kube::apis::DeepMerge for HTTPRouteMatch {
    fn merge_from(&mut self, other: Self) {
        crate::kube::apis::merge_strategies::list::map(
            &mut self.headers,
            other.headers,
            &[|lhs, rhs| lhs.name == rhs.name],
            |current_item, other_item| {
                crate::kube::apis::DeepMerge::merge_from(current_item, other_item);
            },
        );
        crate::kube::apis::DeepMerge::merge_from(&mut self.method, other.method);
        crate::kube::apis::DeepMerge::merge_from(&mut self.path, other.path);
        crate::kube::apis::merge_strategies::list::map(
            &mut self.query_params,
            other.query_params,
            &[|lhs, rhs| lhs.name == rhs.name],
            |current_item, other_item| {
                crate::kube::apis::DeepMerge::merge_from(current_item, other_item);
            },
        );
    }
}

impl<'de> crate::kube::apis::serde::Deserialize<'de> for HTTPRouteMatch {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: crate::kube::apis::serde::Deserializer<'de> {
        #[allow(non_camel_case_types)]
        enum Field {
            Key_headers,
            Key_method,
            Key_path,
            Key_query_params,
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
                            "headers" => Field::Key_headers,
                            "method" => Field::Key_method,
                            "path" => Field::Key_path,
                            "queryParams" => Field::Key_query_params,
                            _ => Field::Other,
                        })
                    }
                }

                deserializer.deserialize_identifier(Visitor)
            }
        }

        struct Visitor;

        impl<'de> crate::kube::apis::serde::de::Visitor<'de> for Visitor {
            type Value = HTTPRouteMatch;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("HTTPRouteMatch")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error> where A: crate::kube::apis::serde::de::MapAccess<'de> {
                let mut value_headers: Option<Vec<crate::kube::apis::networking::gateway::v1::HTTPHeaderMatch>> = None;
                let mut value_method: Option<String> = None;
                let mut value_path: Option<crate::kube::apis::networking::gateway::v1::HTTPPathMatch> = None;
                let mut value_query_params: Option<Vec<crate::kube::apis::networking::gateway::v1::HTTPQueryParamMatch>> = None;

                while let Some(key) = crate::kube::apis::serde::de::MapAccess::next_key::<Field>(&mut map)? {
                    match key {
                        Field::Key_headers => value_headers = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Key_method => value_method = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Key_path => value_path = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Key_query_params => value_query_params = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?,
                        Field::Other => { let _: crate::kube::apis::serde::de::IgnoredAny = crate::kube::apis::serde::de::MapAccess::next_value(&mut map)?; },
                    }
                }

                Ok(HTTPRouteMatch {
                    headers: value_headers,
                    method: value_method,
                    path: value_path,
                    query_params: value_query_params,
                })
            }
        }

        deserializer.deserialize_struct(
            "HTTPRouteMatch",
            &[
                "headers",
                "method",
                "path",
                "queryParams",
            ],
            Visitor,
        )
    }
}

impl crate::kube::apis::serde::Serialize for HTTPRouteMatch {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: crate::kube::apis::serde::Serializer {
        let mut state = serializer.serialize_struct(
            "HTTPRouteMatch",
            self.headers.as_ref().map_or(0, |_| 1) +
            self.method.as_ref().map_or(0, |_| 1) +
            self.path.as_ref().map_or(0, |_| 1) +
            self.query_params.as_ref().map_or(0, |_| 1),
        )?;
        if let Some(value) = &self.headers {
            crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "headers", value)?;
        }
        if let Some(value) = &self.method {
            crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "method", value)?;
        }
        if let Some(value) = &self.path {
            crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "path", value)?;
        }
        if let Some(value) = &self.query_params {
            crate::kube::apis::serde::ser::SerializeStruct::serialize_field(&mut state, "queryParams", value)?;
        }
        crate::kube::apis::serde::ser::SerializeStruct::end(state)
    }
}

#[cfg(feature = "schemars")]
impl crate::kube::apis::schemars::JsonSchema for HTTPRouteMatch {
    fn schema_name() -> String {
        "io.k8s.networking.gateway.v1.HTTPRouteMatch".to_owned()
    }

    fn json_schema(__gen: &mut crate::kube::apis::schemars::gen::SchemaGenerator) -> crate::kube::apis::schemars::schema::Schema {
        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                description: Some("HTTPRouteMatch defines the predicate used to match requests to a given action. Multiple match types are ANDed together, i.e. the match will evaluate to true only if all conditions are satisfied. \n For example, the match below will match a HTTP request only if its path starts with `/foo` AND it contains the `version: v1` header: \n ``` match: \n path: value: \"/foo\" headers: - name: \"version\" value \"v1\" \n ```".to_owned()),
                ..Default::default()
            })),
            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::Object))),
            object: Some(Box::new(crate::kube::apis::schemars::schema::ObjectValidation {
                properties: [
                    (
                        "headers".to_owned(),
                        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
                            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                                description: Some("Headers specifies HTTP request header matchers. Multiple match values are ANDed together, meaning, a request must match all the specified headers to select the route.".to_owned()),
                                ..Default::default()
                            })),
                            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::Array))),
                            array: Some(Box::new(crate::kube::apis::schemars::schema::ArrayValidation {
                                items: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(__gen.subschema_for::<crate::kube::apis::networking::gateway::v1::HTTPHeaderMatch>()))),
                                ..Default::default()
                            })),
                            ..Default::default()
                        }),
                    ),
                    (
                        "method".to_owned(),
                        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
                            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                                description: Some("Method specifies HTTP method matcher. When specified, this route will be matched only if the request has the specified method. \n Support: Extended".to_owned()),
                                ..Default::default()
                            })),
                            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::String))),
                            ..Default::default()
                        }),
                    ),
                    (
                        "path".to_owned(),
                        __gen.subschema_for::<crate::kube::apis::networking::gateway::v1::HTTPPathMatch>(),
                    ),
                    (
                        "queryParams".to_owned(),
                        crate::kube::apis::schemars::schema::Schema::Object(crate::kube::apis::schemars::schema::SchemaObject {
                            metadata: Some(Box::new(crate::kube::apis::schemars::schema::Metadata {
                                description: Some("QueryParams specifies HTTP query parameter matchers. Multiple match values are ANDed together, meaning, a request must match all the specified query parameters to select the route. \n Support: Extended".to_owned()),
                                ..Default::default()
                            })),
                            instance_type: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(crate::kube::apis::schemars::schema::InstanceType::Array))),
                            array: Some(Box::new(crate::kube::apis::schemars::schema::ArrayValidation {
                                items: Some(crate::kube::apis::schemars::schema::SingleOrVec::Single(Box::new(__gen.subschema_for::<crate::kube::apis::networking::gateway::v1::HTTPQueryParamMatch>()))),
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
