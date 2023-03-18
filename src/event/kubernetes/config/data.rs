use std::collections::{btree_map::Iter, BTreeMap};

use anyhow::anyhow;
use base64::{engine::general_purpose, Engine};
use k8s_openapi::{api::core::v1::ConfigMap, NamespaceResourceScope};
use kube::{core::ObjectMeta, Api};
use serde::Deserialize;

use crate::{
    error::{Error, Result},
    event::{kubernetes::client::KubeClient, util::color::Color},
};

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
        SecretDataToStringIterator::new(self.0.iter())
            .flat_map(|line| line.lines().map(ToString::to_string).collect::<Vec<_>>())
            .collect()
    }
}

struct SecretDataToStringIterator<'a> {
    iter: Iter<'a, String, String>,
    color: Color,
}

impl<'a> SecretDataToStringIterator<'a> {
    fn new(iter: Iter<'a, String, String>) -> Self {
        Self {
            iter,
            color: Color::new(),
        }
    }
}

impl<'a> SecretDataToStringIterator<'a> {
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

        // let newline = if value.contains('\n') { "\n" } else { "" };
        // format!(
        //     "\x1b[{color}m{key}:\x1b[39m {newline}{value}",
        //     color = color,
        //     key = key,
        //     value = value,
        //     newline = newline
        // )
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

impl Iterator for SecretDataToStringIterator<'_> {
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

fn format_config_key_value(key: &str, value: &str, color: u8) -> Vec<String> {
    if value.contains('\n') {
        let mut ret = vec![format!(
            "\x1b[{color}m{key}:\x1b[39m |",
            color = color,
            key = key
        )];

        let value: Vec<String> = value.lines().map(|l| format!("  {}", l)).collect();

        ret.extend(value);

        ret
    } else {
        vec![format!(
            "\x1b[{color}m{key}:\x1b[39m {value}",
            color = color,
            key = key,
            value = value,
        )]
    }
}

pub async fn get_config(
    client: KubeClient,
    ns: &str,
    kind: &str,
    name: &str,
) -> Result<Vec<String>> {
    match kind {
        "ConfigMap" => {
            let cms: Api<ConfigMap> = Api::namespaced(client.as_client().clone(), ns);
            let cm = cms.get(name).await?;
            if let Some(data) = cm.data {
                Ok(data
                    .iter()
                    .scan(Color::new(), |c, (k, v)| {
                        Some(format_config_key_value(k, v, c.next_color()))
                    })
                    .flatten()
                    .collect())
            } else {
                Err(anyhow!(Error::NoneParameter("configmap.data")))
            }
        }
        "Secret" => {
            let secs: Api<Secret> = Api::namespaced(client.as_client().clone(), ns);
            let sec = secs.get(name).await?;

            if let Some(data) = sec.data {
                Ok(data.to_string_key_values())
            } else {
                Err(anyhow!(Error::NoneParameter("secret.data")))
            }
        }
        _ => Err(anyhow!(Error::Raw(format!(
            "Invalid kind [{}]. Set kind ConfigMap or Secret",
            kind
        )))),
    }
}
