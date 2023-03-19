use std::collections::{btree_map, BTreeMap};

use async_trait::async_trait;
use base64::{engine::general_purpose, Engine};
use k8s_openapi::NamespaceResourceScope;
use kube::{core::ObjectMeta, Api};
use serde::Deserialize;

use crate::{
    error::Result,
    event::kubernetes::{client::KubeClient, color::Color, config::ConfigData},
};

use super::Fetch;

pub(super) struct SecretDataWorker<'a> {
    client: &'a KubeClient,
    namespace: String,
    name: String,
}

#[async_trait()]
impl<'a> Fetch<'a> for SecretDataWorker<'a> {
    fn new(client: &'a KubeClient, namespace: String, name: String) -> Self {
        Self {
            client,
            namespace,
            name,
        }
    }

    async fn fetch(&self) -> Result<ConfigData> {
        let list: Api<Secret> = Api::namespaced(self.client.as_client().clone(), &self.namespace);
        let target = list.get(&self.name).await?;

        if let Some(data) = target.data {
            Ok(data.to_string_key_values())
        } else {
            Ok(vec!["no data".into()])
        }
    }
}

#[derive(Debug, Default, Clone, Deserialize)]
struct Secret {
    metadata: ObjectMeta,
    data: Option<SecretData>,
}

impl kube::Resource for Secret {
    type DynamicType = ();
    type Scope = NamespaceResourceScope;

    fn kind(_: &Self::DynamicType) -> std::borrow::Cow<'_, str> {
        "Secret".into()
    }

    fn group(_: &Self::DynamicType) -> std::borrow::Cow<'_, str> {
        "".into()
    }

    fn version(_: &Self::DynamicType) -> std::borrow::Cow<'_, str> {
        "v1".into()
    }

    fn plural(_: &Self::DynamicType) -> std::borrow::Cow<'_, str> {
        "secrets".into()
    }

    fn meta(&self) -> &ObjectMeta {
        &self.metadata
    }

    fn meta_mut(&mut self) -> &mut ObjectMeta {
        &mut self.metadata
    }
}

#[derive(Debug, Default, Clone, Deserialize)]
struct SecretData(BTreeMap<String, String>);

impl SecretData {
    fn to_string_key_values(&self) -> Vec<String> {
        let ret: Vec<String> = self
            .iter()
            .flat_map(|key_value| {
                key_value
                    .lines()
                    .map(ToString::to_string)
                    .collect::<Vec<String>>()
            })
            .collect();

        ret
    }

    fn iter(&self) -> Iter {
        Iter {
            iter: self.0.iter(),
            color: Color::new(),
        }
    }
}

struct Iter<'a> {
    iter: btree_map::Iter<'a, String, String>,
    color: Color,
}

impl<'a> Iter<'a> {
    fn format_utf8(key: &str, value: &str, color: u8) -> String {
        if value.contains('\n') {
            let mut ret = format!("\x1b[{color}m{key}:\x1b[39m |\n", color = color, key = key);

            value.lines().for_each(|l| {
                ret += &format!("  {}\n", l);
            });

            ret
        } else {
            format!(
                "\x1b[{color}m{key}:\x1b[39m {value}",
                color = color,
                key = key,
                value = value,
            )
        }
    }

    fn format_non_utf8(key: &str, value: &str, color: u8) -> String {
        Self::format_error(key, value, "Can't output a non-UTF8 value", color)
    }

    fn format_decode_error(key: &str, value: &str, err: &str, color: u8) -> String {
        Self::format_error(key, value, &format!("\x1b[31m{}\x1b[39m", err), color)
    }

    fn format_error(key: &str, value: &str, err: &str, color: u8) -> String {
        format!(
            "\x1b[{color}m{key}:\x1b[39m {error}\n[base64-encoded] {value}",
            color = color,
            key = key,
            value = value,
            error = err
        )
    }
}

impl Iterator for Iter<'_> {
    type Item = String;
    fn next(&mut self) -> std::option::Option<<Self as Iterator>::Item> {
        if let Some((k, v)) = self.iter.next() {
            let c = self.color.next_color();

            match general_purpose::STANDARD.decode(v) {
                Ok(decoded_data) => {
                    if let Ok(utf8_data) = String::from_utf8(decoded_data) {
                        Some(Self::format_utf8(k, &utf8_data, c))
                    } else {
                        Some(Self::format_non_utf8(k, v, c))
                    }
                }
                Err(err) => Some(Self::format_decode_error(k, v, &err.to_string(), c)),
            }
        } else {
            None
        }
    }
}
