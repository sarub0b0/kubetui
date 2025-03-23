use std::path::PathBuf;

use anyhow::{anyhow, Result};
use clap::{Parser, ValueEnum};
use k8s_openapi::api::core::v1::Namespace;
use kube::{
    api::ListParams,
    config::{KubeConfigOptions, Kubeconfig, KubeconfigError, NamedContext},
    Api, Client, Config, ResourceExt as _,
};

use crate::kube::KubeClient;

use super::{
    completion::{generate_bash_completion, generate_zsh_completion},
    Command,
};

#[derive(ValueEnum, Debug, Clone)]
pub enum Shell {
    Zsh,
    Bash,
}

#[derive(Parser, Debug, Clone)]
pub enum SubCommand {
    /// Generate completion script
    Completion { shell: Shell },

    #[command(subcommand, name = "__complete", hide = true)]
    Complete(CompletionCandidate),
}

#[derive(Parser, Debug, Clone)]
pub enum CompletionCandidate {
    Context {
        #[arg(raw = true)]
        args: Vec<String>,
    },
    Namespace {
        #[arg(raw = true)]
        args: Vec<String>,
    },
}

impl SubCommand {
    pub fn run(self) -> Result<()> {
        match self {
            SubCommand::Completion { shell } => {
                generate_completion_script(shell);
            }
            SubCommand::Complete(CompletionCandidate::Context { args }) => {
                complete_context(args)?;
            }
            SubCommand::Complete(CompletionCandidate::Namespace { args }) => {
                complete_namespace(args)?;
            }
        }

        Ok(())
    }
}

fn generate_completion_script(shell: Shell) {
    match shell {
        Shell::Zsh => {
            generate_zsh_completion();
        }
        Shell::Bash => {
            generate_bash_completion();
        }
    }
}

fn read_kubeconfig(path: Option<PathBuf>) -> Result<Kubeconfig, KubeconfigError> {
    if let Some(path) = path {
        Kubeconfig::read_from(path)
    } else {
        Kubeconfig::read()
    }
}

fn read_context(kubeconfig: &Kubeconfig, context: Option<String>) -> Result<NamedContext> {
    let context = if let Some(context) = context {
        kubeconfig
            .contexts
            .iter()
            .find(|ctx| ctx.name == context)
            .cloned()
            .ok_or_else(|| anyhow!(format!("Cannot find context {}", context)))?
    } else if let Some(current_context) = &kubeconfig.current_context {
        kubeconfig
            .contexts
            .iter()
            .find(|ctx| ctx.name == *current_context)
            .cloned()
            .ok_or_else(|| anyhow!(format!("Cannot find context {}", current_context)))?
    } else {
        kubeconfig
            .contexts
            .first()
            .cloned()
            .ok_or_else(|| anyhow!("Empty contexts"))?
    };

    Ok(context)
}

fn read_contexts(path: Option<PathBuf>) -> Result<Vec<String>> {
    let kubeconfig = read_kubeconfig(path)?;

    let contexts = kubeconfig
        .contexts
        .iter()
        .map(|ctx| ctx.name.to_string())
        .collect();

    Ok(contexts)
}

fn complete_context(args: Vec<String>) -> Result<()> {
    let cmd = Command::parse_from(args);

    let contexts = read_contexts(cmd.kubeconfig)?;

    contexts
        .iter()
        .filter(|ctx| {
            if let Some(context) = &cmd.context {
                ctx.starts_with(context)
            } else {
                true
            }
        })
        .for_each(|ctx| {
            println!("{}", ctx);
        });

    Ok(())
}

fn complete_namespace(args: Vec<String>) -> Result<()> {
    let rt = tokio::runtime::Runtime::new()?;

    rt.block_on(async {
        let cmd = Command::parse_from(args);

        let kubeconfig = read_kubeconfig(cmd.kubeconfig)?;

        let context = read_context(&kubeconfig, cmd.context)?;

        let client = kubeclient(&kubeconfig, &context).await?;

        let mut namespaces: Vec<_> = {
            let namespaces: Api<Namespace> = Api::all(client.as_client().clone());
            let lp = ListParams::default();
            let ns_list = namespaces.list(&lp).await?;

            anyhow::Ok(ns_list.iter().map(|ns| ns.name_any()).collect())
        }?;

        if let Some(retains) = cmd.namespaces.as_ref() {
            namespaces.retain(|ns| {
                !retains.contains(ns)
                    || cmd
                        .namespaces
                        .as_ref()
                        .is_some_and(|namespaces| namespaces.last().is_some_and(|last| last == ns))
            });
        }

        let last_namespace = cmd.namespaces.as_ref().map(|namespaces| {
            namespaces
                .iter()
                .last()
                .map(|ns| ns.as_str())
                .unwrap_or_else(|| "")
        });

        namespaces
            .iter()
            .filter(|ns| {
                if let Some(namespace) = &last_namespace {
                    ns.starts_with(namespace)
                } else {
                    true
                }
            })
            .for_each(|ns| {
                println!("{}", ns);
            });

        anyhow::Ok(())
    })
}

async fn kubeclient(config: &Kubeconfig, context: &NamedContext) -> Result<KubeClient> {
    let Kubeconfig {
        clusters,
        auth_infos,
        ..
    } = &config;

    let cluster = clusters.iter().find_map(|cluster| {
        if cluster.name == context.name {
            Some(cluster.name.to_string())
        } else {
            None
        }
    });

    let user = auth_infos.iter().find_map(|auth_info| {
        let kube::config::Context { user, .. } = context.context.as_ref()?;

        let user = user.as_ref()?;

        if &auth_info.name == user {
            Some(auth_info.name.to_string())
        } else {
            None
        }
    });

    let options = KubeConfigOptions {
        context: Some(context.name.to_string()),
        cluster,
        user,
    };

    let config = Config::from_custom_kubeconfig(config.clone(), &options).await?;

    let cluster_url: String = config.cluster_url.to_string();

    let client = Client::try_from(config)?;

    let kube_client = KubeClient::new(client, cluster_url);

    Ok(kube_client)
}
