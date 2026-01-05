use std::{collections::BTreeMap, fmt::Debug};

use anyhow::{anyhow, Result};
use futures::future::try_join_all;
use kube::{
    config::{KubeConfigOptions, Kubeconfig, NamedContext},
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
    fn find_context<'a>(
        kubeconfig: &'a Kubeconfig,
        context_name: &str,
    ) -> Result<&'a NamedContext> {
        kubeconfig
            .contexts
            .iter()
            .find(|ctx| ctx.name == context_name)
            .ok_or_else(|| anyhow!(format!("Cannot find context {}", context_name)))
    }

    fn kubeconfig_options(context: &NamedContext) -> KubeConfigOptions {
        KubeConfigOptions {
            context: Some(context.name.to_string()),
            ..Default::default()
        }
    }

    async fn build_state(config: &Kubeconfig, context: &NamedContext) -> Result<KubeState> {
        let options = Self::kubeconfig_options(context);

        let config = Config::from_custom_kubeconfig(config.clone(), &options).await?;

        let target_namespace = config.default_namespace.to_string();

        let client = Client::try_from(config)?;

        let kube_client = KubeClient::new(client);

        Ok(KubeState {
            client: kube_client,
            target_namespaces: vec![target_namespace],
            target_api_resources: vec![],
        })
    }

    pub async fn try_from_kubeconfig(config: Kubeconfig) -> Result<Self> {
        let jobs: Vec<(Context, KubeState)> =
            try_join_all(config.contexts.iter().map(|context| async {
                let state = Self::build_state(&config, context).await?;

                anyhow::Ok((context.name.to_string(), state))
            }))
            .await?;

        let inner: BTreeMap<Context, KubeState> = jobs.into_iter().collect();

        Ok(inner.into())
    }

    pub async fn try_from_kubeconfig_with_context(
        config: Kubeconfig,
        context_name: &str,
    ) -> Result<Self> {
        let context = Self::find_context(&config, context_name)?;

        let state = Self::build_state(&config, context).await?;

        let inner = BTreeMap::from([(context.name.to_string(), state)]);

        Ok(inner.into())
    }

    pub async fn ensure_context(
        &mut self,
        kubeconfig: &Kubeconfig,
        context_name: &str,
    ) -> Result<()> {
        if self.inner.contains_key(context_name) {
            return Ok(());
        }

        let context = Self::find_context(kubeconfig, context_name)?;
        let state = Self::build_state(kubeconfig, context).await?;

        self.inner.insert(context.name.to_string(), state);

        Ok(())
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

    const CONFIG_CONTEXT_CLUSTER_MISMATCH: &str = indoc! {
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
                name: dev
            contexts:
              - context:
                  cluster: cluster-1
                  namespace: ns-1
                  user: user-1
                name: dev
            current-context: dev
            kind: Config
            preferences: {}
            users:
              - name: user-1
                user:
                  token: user-1
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
                    client: KubeClient::new(client.clone()),
                    target_namespaces: vec!["ns-1".to_string()],
                    target_api_resources: Default::default(),
                },
            ),
            (
                "cluster-2".to_string(),
                KubeState {
                    client: KubeClient::new(client.clone()),
                    target_namespaces: vec!["ns-2".to_string()],
                    target_api_resources: Default::default(),
                },
            ),
            (
                "cluster-3".to_string(),
                KubeState {
                    client: KubeClient::new(client),
                    target_namespaces: vec!["default".to_string()],
                    target_api_resources: Default::default(),
                },
            ),
        ])
        .into();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn uses_context_cluster_when_names_differ() {
        let kubeconfig = Kubeconfig::from_yaml(CONFIG_CONTEXT_CLUSTER_MISMATCH).unwrap();

        let context = KubeStore::find_context(&kubeconfig, "dev").unwrap();
        let options = KubeStore::kubeconfig_options(context);

        let config = Config::from_custom_kubeconfig(kubeconfig, &options)
            .await
            .unwrap();

        assert_eq!(config.cluster_url, "https://192.168.0.1/");
    }
}
