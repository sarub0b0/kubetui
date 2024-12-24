use std::{ops::Deref, path::PathBuf};

use anyhow::{anyhow, Result};
use kube::config::{Kubeconfig, KubeconfigError};

use crate::features::{
    api_resources::kube::ApiConfig, event::kube::EventConfig, pod::kube::PodConfig,
};

use super::TargetNamespaces;

#[derive(Debug, Default, Clone)]
pub struct KubeWorkerConfig {
    pub kubeconfig: Option<PathBuf>,
    pub target_namespaces: Option<TargetNamespaces>,
    pub context: Option<String>,
    pub all_namespaces: bool,
    pub pod_config: PodConfig,
    pub event_config: EventConfig,
    pub api_config: ApiConfig,
}

pub struct Context(String);

impl Deref for Context {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Context {
    pub fn try_from(kubeconfig: &Kubeconfig, context: Option<String>) -> Result<Self> {
        let context = if let Some(context) = context {
            kubeconfig
                .contexts
                .iter()
                .find_map(|ctx| {
                    if ctx.name == context {
                        Some(ctx.name.to_string())
                    } else {
                        None
                    }
                })
                .ok_or_else(|| anyhow!(format!("Cannot find context {}", context)))?
        } else if let Some(current_context) = &kubeconfig.current_context {
            current_context.to_string()
        } else {
            kubeconfig
                .contexts
                .first()
                .ok_or_else(|| anyhow!("Empty contexts"))?
                .name
                .to_string()
        };

        Ok(Self(context))
    }
}

pub fn read_kubeconfig(kubeconfig: Option<PathBuf>) -> Result<Kubeconfig, KubeconfigError> {
    if let Some(path) = kubeconfig {
        Kubeconfig::read_from(path)
    } else {
        Kubeconfig::read()
    }
}
