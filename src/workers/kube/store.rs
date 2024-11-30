use std::{collections::BTreeMap, fmt::Debug};

use anyhow::{anyhow, Result};
use futures::future::try_join_all;
use kube::{
    config::{KubeConfigOptions, Kubeconfig},
    Client, Config,
};

use crate::kube::KubeClient;

use super::controller::{TargetApiResources, TargetNamespaces};

pub type Context = String;

#[derive(Clone)]
pub struct KubeState {
    pub client: KubeClient,
    pub target_namespaces: TargetNamespaces,
    pub target_api_resources: TargetApiResources,
}

impl KubeState {
    pub fn new(
        client: KubeClient,
        target_namespaces: TargetNamespaces,
        target_api_resources: TargetApiResources,
    ) -> Self {
        Self {
            client,
            target_namespaces,
            target_api_resources,
        }
    }
}

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub struct KubeStore {
    inner: BTreeMap<Context, KubeState>,
}

impl From<BTreeMap<Context, KubeState>> for KubeStore {
    fn from(inner: BTreeMap<Context, KubeState>) -> Self {
        KubeStore { inner }
    }
}

impl std::fmt::Debug for KubeState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "KubeStore {{ client: _, target_namespaces: {:?}, target_api_resources: {:?} }}",
            self.target_namespaces, self.target_api_resources
        )
    }
}

impl KubeStore {
    pub async fn try_from_kubeconfig(config: Kubeconfig) -> Result<Self> {
        let Kubeconfig {
            clusters,
            contexts,
            auth_infos,
            ..
        } = &config;

        let jobs: Vec<(Context, KubeState)> = try_join_all(contexts.iter().map(|context| async {
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
            let target_namespace = config.default_namespace.to_string();

            let client = Client::try_from(config)?;

            let kube_client = KubeClient::new(client, cluster_url);

            anyhow::Ok((
                context.name.to_string(),
                KubeState {
                    client: kube_client,
                    target_namespaces: vec![target_namespace],
                    target_api_resources: vec![],
                },
            ))
        }))
        .await?;

        let inner: BTreeMap<Context, KubeState> = jobs.into_iter().collect();

        Ok(inner.into())
    }

    pub fn get(&self, context: &str) -> Result<&KubeState> {
        self.inner
            .get(context)
            .ok_or_else(|| anyhow!(format!("Cannot get context {}", context)))
    }

    pub fn get_mut(&mut self, context: &str) -> Result<&mut KubeState> {
        self.inner
            .get_mut(context)
            .ok_or_else(|| anyhow!(format!("Cannot get context {}", context)))
    }

    pub fn insert(&mut self, context: Context, state: KubeState) {
        self.inner.insert(context, state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;
    use pretty_assertions::assert_eq;

    impl PartialEq for KubeState {
        fn eq(&self, rhs: &Self) -> bool {
            self.target_namespaces == rhs.target_namespaces
                && self.target_api_resources == rhs.target_api_resources
                && self.client.as_server_url() == rhs.client.as_server_url()
        }
    }

    const CONFIG: &str = indoc! {
        r#"
            apiVersion: v1
            clusters:
              - cluster:
                  certificate-authority-data: ""
                  server: https://192.168.0.1
                name: cluster-1
              - cluster:
                  certificate-authority-data: ""
                  server: https://192.168.0.2
                name: cluster-2
              - cluster:
                  certificate-authority-data: ""
                  server: https://192.168.0.3
                name: cluster-3
            contexts:
              - context:
                  cluster: cluster-1
                  namespace: ns-1
                  user: user-1
                name: cluster-1
              - context:
                  cluster: cluster-2
                  namespace: ns-2
                  user: user-2
                name: cluster-2
              - context:
                  cluster: cluster-3
                  user: user-3
                name: cluster-3
            current-context: cluster-2
            kind: Config
            preferences: {}
            users:
              - name: user-1
                user:
                  token: user-1
              - name: user-2
                user:
                  token: user-2
              - name: user-3
                user:
                  token: user-3
            "#
    };

    #[tokio::test]
    async fn kubeconfigからstateを生成() {
        let kubeconfig = Kubeconfig::from_yaml(CONFIG).unwrap();

        let actual = KubeStore::try_from_kubeconfig(kubeconfig).await.unwrap();

        let config = Config::new(Default::default());

        let client = Client::try_from(config).unwrap();

        let expected = BTreeMap::from([
            (
                "cluster-1".to_string(),
                KubeState {
                    client: KubeClient::new(client.clone(), "https://192.168.0.1/"),
                    target_namespaces: vec!["ns-1".to_string()],
                    target_api_resources: Default::default(),
                },
            ),
            (
                "cluster-2".to_string(),
                KubeState {
                    client: KubeClient::new(client.clone(), "https://192.168.0.2/"),
                    target_namespaces: vec!["ns-2".to_string()],
                    target_api_resources: Default::default(),
                },
            ),
            (
                "cluster-3".to_string(),
                KubeState {
                    client: KubeClient::new(client, "https://192.168.0.3/"),
                    target_namespaces: vec!["default".to_string()],
                    target_api_resources: Default::default(),
                },
            ),
        ])
        .into();

        assert_eq!(actual, expected);
    }
}
