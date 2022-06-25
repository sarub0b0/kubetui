use super::{
    v1_table::*,
    worker::{PollWorker, Worker},
    Event, Kube, KubeClient, KubeTable, KubeTableRow, WorkerResult,
};

use std::{
    collections::{btree_map::Iter, BTreeMap},
    time,
};

use futures::future::try_join_all;
use k8s_openapi::api::core::v1::ConfigMap;

use kube::{api::ObjectMeta, Api};

use async_trait::async_trait;
use serde::Deserialize;

use crate::{
    error::{anyhow, Error, Result},
    event::util::color::Color,
};

#[derive(Debug)]
pub enum ConfigMessage {
    List(Result<KubeTable>),
    DataRequest {
        namespace: String,
        kind: String,
        name: String,
    },
    DataResponse(Result<Vec<String>>),
}

impl From<ConfigMessage> for Kube {
    fn from(msg: ConfigMessage) -> Self {
        Kube::Config(msg)
    }
}

impl From<ConfigMessage> for Event {
    fn from(msg: ConfigMessage) -> Self {
        Event::Kube(msg.into())
    }
}

#[derive(Clone)]
pub struct ConfigsPollWorker {
    inner: PollWorker,
}

impl ConfigsPollWorker {
    pub fn new(inner: PollWorker) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl Worker for ConfigsPollWorker {
    type Output = Result<WorkerResult>;

    async fn run(&self) -> Self::Output {
        let mut interval = tokio::time::interval(time::Duration::from_secs(1));

        let Self {
            inner:
                PollWorker {
                    is_terminated,
                    tx,
                    namespaces,
                    kube_client,
                },
        } = self;

        while !is_terminated.load(std::sync::atomic::Ordering::Relaxed) {
            interval.tick().await;

            let namespaces = namespaces.read().await;

            let table = fetch_configs(kube_client, &namespaces).await;

            tx.send(ConfigMessage::List(table).into())?;
        }
        Ok(WorkerResult::Terminated)
    }
}

#[derive(Clone, Copy)]
enum Configs {
    ConfigMap,
    Secret,
}

impl Configs {
    fn kind(&self) -> &'static str {
        match self {
            Self::ConfigMap => "configmaps",
            Self::Secret => "secrets",
        }
    }

    fn resource(&self) -> &'static str {
        match self {
            Self::ConfigMap => "ConfigMap",
            Self::Secret => "Secret",
        }
    }
}

async fn fetch_configs_per_namespace(
    client: &KubeClient,
    namespaces: &[String],
    ty: Configs,
) -> Result<Vec<KubeTableRow>> {
    let insert_ns = insert_ns(namespaces);
    let jobs = try_join_all(namespaces.iter().map(|ns| {
        get_resource_per_namespace(
            client,
            format!("api/v1/namespaces/{}/{}", ns, ty.kind()),
            &["Name", "Data", "Age"],
            move |row: &TableRow, indexes: &[usize]| {
                let mut row = vec![
                    ty.resource().to_string(),
                    row.cells[indexes[0]].to_string(),
                    row.cells[indexes[1]].to_string(),
                    row.cells[indexes[2]].to_string(),
                ];

                let kind = row[0].clone();
                let name = row[1].clone();

                if insert_ns {
                    row.insert(0, ns.to_string())
                }

                KubeTableRow {
                    namespace: ns.to_string(),
                    name,
                    row,
                    metadata: Some(BTreeMap::from([("kind".to_string(), kind)])),
                }
            },
        )
    }))
    .await?;

    Ok(jobs.into_iter().flatten().collect())
}

async fn fetch_configs(client: &KubeClient, namespaces: &[String]) -> Result<KubeTable> {
    let mut table = KubeTable {
        header: if namespaces.len() == 1 {
            ["KIND", "NAME", "DATA", "AGE"]
                .iter()
                .map(ToString::to_string)
                .collect()
        } else {
            ["NAMESPACE", "KIND", "NAME", "DATA", "AGE"]
                .iter()
                .map(ToString::to_string)
                .collect()
        },
        ..Default::default()
    };

    let jobs = try_join_all([
        fetch_configs_per_namespace(client, namespaces, Configs::ConfigMap),
        fetch_configs_per_namespace(client, namespaces, Configs::Secret),
    ])
    .await?;

    table.update_rows(jobs.into_iter().flatten().collect());

    Ok(table)
}

#[derive(Debug, Default, Clone, Deserialize)]
struct Secret {
    metadata: ObjectMeta,
    data: Option<SecretData>,
}

impl kube::Resource for Secret {
    type DynamicType = ();

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
            match base64::decode(v) {
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
