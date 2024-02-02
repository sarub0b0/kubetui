mod any;
mod format;
mod helm;

use std::collections::BTreeMap;

use anyhow::Result;
use async_trait::async_trait;
use k8s_openapi::{api::core::v1::Secret, ByteString};
use kube::Api;

use crate::{features::config::message::ConfigData, kube::KubeClient};

use self::{any::Any, helm::Helm};

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

        let type_ = target.type_.as_deref().unwrap_or_default();

        let Some(data) = target.data else {
            return Ok(vec!["no data".into()]);
        };

        let data = SecretData::new(type_, data)?;
        Ok(data.to_string_key_values())
    }
}

#[derive(Debug)]
enum SecretData {
    Helm(Helm),
    Any(Any),
}

impl SecretData {
    fn new(type_: &str, data: BTreeMap<String, ByteString>) -> Result<Self> {
        match type_ {
            "helm.sh/release.v1" => Ok(Self::Helm(Helm::new(data))),
            _ => Ok(Self::Any(Any::new(data))),
        }
    }

    fn to_string_key_values(&self) -> ConfigData {
        match self {
            Self::Helm(helm) => helm.to_string_key_values(),
            Self::Any(any) => any.to_string_key_values(),
        }
    }
}
