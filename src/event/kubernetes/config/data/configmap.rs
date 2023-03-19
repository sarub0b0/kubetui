use std::collections::BTreeMap;

use async_trait::async_trait;
use k8s_openapi::api::core::v1::ConfigMap;
use kube::Api;

use crate::{
    error::Result,
    event::kubernetes::{client::KubeClient, color::Color, config::ConfigData},
};

use super::Fetch;

pub(super) struct ConfigMapDataWorker<'a> {
    client: &'a KubeClient,
    namespace: String,
    name: String,
}

#[async_trait()]
impl<'a> Fetch<'a> for ConfigMapDataWorker<'a> {
    fn new(client: &'a KubeClient, namespace: String, name: String) -> Self {
        Self {
            client,
            namespace,
            name,
        }
    }

    async fn fetch(&self) -> Result<ConfigData> {
        let list: Api<ConfigMap> =
            Api::namespaced(self.client.as_client().clone(), &self.namespace);

        let target = list.get(&self.name).await?;

        if let Some(data) = target.data {
            let data = ConfigMapData(data);
            Ok(data.to_vec_string_with_color())
        } else {
            Ok(vec!["no data".into()])
        }
    }
}

struct ConfigMapData(BTreeMap<String, String>);

impl ConfigMapData {
    fn to_vec_string_with_color(&self) -> Vec<String> {
        let ret = self
            .0
            .iter()
            .scan(Color::new(), |color, (key, value)| {
                let color = color.next_color();

                if value.contains('\n') {
                    let mut ret = vec![format!(
                        "\x1b[{color}m{key}:\x1b[39m |",
                        color = color,
                        key = key
                    )];

                    let value: Vec<String> = value.lines().map(|l| format!("  {}\n", l)).collect();

                    ret.extend(value);

                    Some(ret)
                } else {
                    Some(vec![format!(
                        "\x1b[{color}m{key}:\x1b[39m {value}",
                        color = color,
                        key = key,
                        value = value,
                    )])
                }
            })
            .flatten()
            .collect();

        ret
    }
}
