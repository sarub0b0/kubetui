use k8s_openapi::apimachinery::pkg::apis::meta::v1::{ListMeta, ObjectMeta, Time};
use kube::api::TypeMeta;

use serde::Deserialize;
use serde::Deserializer;

use std::collections::HashMap;

fn deserialize_unwrap_or_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    T: Default + Deserialize<'de>,
    D: Deserializer<'de>,
{
    Ok(Option::deserialize(deserializer)?.unwrap_or_default())
}

type ResourceList = HashMap<String, String>;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeMetrics {
    #[serde(flatten)]
    pub type_meta: Option<TypeMeta>,
    pub metadata: Option<ObjectMeta>,

    pub timestamp: Time,
    pub window: String,
    pub usage: ResourceList,
}

#[derive(Default, Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeMetricsList {
    #[serde(flatten)]
    pub type_meta: Option<TypeMeta>,
    pub metadata: Option<ListMeta>,

    #[serde(deserialize_with = "deserialize_unwrap_or_default")]
    pub items: Vec<NodeMetrics>,
}

impl NodeMetricsList {
    pub fn names(&self) -> Vec<String> {
        self.items
            .iter()
            .map(|i| {
                i.metadata
                    .clone()
                    .unwrap_or_default()
                    .name
                    .unwrap_or_default()
            })
            .collect()
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PodMetrics {
    #[serde(flatten)]
    pub type_meta: Option<TypeMeta>,
    pub metadata: Option<ObjectMeta>,

    pub timestamp: Time,
    pub window: String,
    #[serde(deserialize_with = "deserialize_unwrap_or_default")]
    pub containers: Vec<ContainerMetrics>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PodMetricsList {
    #[serde(flatten)]
    pub type_meta: Option<TypeMeta>,
    pub metadata: Option<ListMeta>,

    #[serde(deserialize_with = "deserialize_unwrap_or_default")]
    pub items: Vec<PodMetrics>,
}

impl PodMetricsList {
    pub fn names(&self) -> Vec<String> {
        self.items
            .iter()
            .map(|i| {
                i.metadata
                    .clone()
                    .unwrap_or_default()
                    .name
                    .unwrap_or_default()
            })
            .collect()
    }
}
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContainerMetrics {
    pub name: String,
    pub usage: ResourceList,
}
