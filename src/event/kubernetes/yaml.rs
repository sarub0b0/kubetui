use std::sync::{atomic::AtomicBool, Arc};

use crossbeam::channel::Sender;

use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use serde::Deserialize;

use super::{
    api_resources::{ApiResources, SharedApiResources},
    client::KubeClientRequest,
    worker::Worker,
    Kube,
};
use crate::{
    error::{Error, Result},
    event::Event,
};

#[derive(Debug, Clone)]
pub struct YamlResourceListItem {
    pub kind: String,
    pub name: String,
    pub namespace: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct YamlResourceList {
    pub items: Vec<YamlResourceListItem>,
}

impl YamlResourceList {
    pub fn new(items: Vec<YamlResourceListItem>) -> Self {
        YamlResourceList { items }
    }
}

#[derive(Debug)]
pub enum YamlRequest {
    APIs,
    Resource(String),
    Yaml {
        kind: String,
        name: String,
        namespace: String,
    },
}

impl From<YamlRequest> for Event {
    fn from(req: YamlRequest) -> Self {
        Event::Kube(Kube::Yaml(YamlMessage::Request(req)))
    }
}

#[derive(Debug)]
pub enum YamlResponse {
    APIs(Result<Vec<String>>),
    Resource(Result<YamlResourceList>),
    Yaml(Result<Vec<String>>),
}

impl From<YamlResponse> for Event {
    fn from(res: YamlResponse) -> Self {
        Event::Kube(Kube::Yaml(YamlMessage::Response(res)))
    }
}

#[derive(Debug)]
pub enum YamlMessage {
    Request(YamlRequest),
    Response(YamlResponse),
}

impl From<YamlMessage> for Kube {
    fn from(m: YamlMessage) -> Self {
        Self::Yaml(m)
    }
}

impl From<YamlMessage> for Event {
    fn from(m: YamlMessage) -> Self {
        Self::Kube(m.into())
    }
}

pub mod fetch_resource_list {
    use crate::event::kubernetes::yaml::fetch_resource_list::not_namespaced::FetchResourceListNotNamespaced;
    use crate::event::kubernetes::TargetNamespaces;

    use self::multiple_namespace::FetchResourceListMultipleNamespaces;

    use self::single_namespace::FetchResourceListSingleNamespace;

    use super::*;

    #[derive(Default, Debug, Clone, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct List {
        items: Vec<Item>,
    }

    #[derive(Default, Debug, Clone, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Item {
        metadata: ObjectMeta,
    }

    mod not_namespaced {
        use anyhow::Result;

        use crate::{
            event::kubernetes::{
                api_resources::APIInfo, client::KubeClientRequest, yaml::YamlResourceListItem,
            },
            logger,
        };

        use super::List;

        pub(super) struct FetchResourceListNotNamespaced<'a, C: KubeClientRequest> {
            client: &'a C,
            api: &'a APIInfo,
            kind: &'a str,
        }

        impl<'a, C: KubeClientRequest> FetchResourceListNotNamespaced<'a, C> {
            pub(super) fn new(client: &'a C, api: &'a APIInfo, kind: &'a str) -> Self {
                Self { client, api, kind }
            }

            pub(super) async fn fetch(&self) -> Result<Vec<YamlResourceListItem>> {
                let path = format!("{}/{}", self.api.api_url(), self.kind);
                logger!(info, "Fetching resource [{}]", path);

                let res: List = self.client.request(&path).await?;

                logger!(info, "Fetched resource - {:?}", res);

                Ok(res
                    .items
                    .into_iter()
                    .filter_map(|item| {
                        item.metadata.name.map(|name| YamlResourceListItem {
                            namespace: "".to_string(),
                            name: name.to_string(),
                            kind: self.api.resource_full_name(),
                            value: name,
                        })
                    })
                    .collect())
            }
        }
    }

    mod single_namespace {
        use anyhow::Result;

        use crate::{
            event::kubernetes::{
                api_resources::APIInfo, client::KubeClientRequest, yaml::YamlResourceListItem,
            },
            logger,
        };

        use super::List;

        pub(super) struct FetchResourceListSingleNamespace<'a, C: KubeClientRequest> {
            client: &'a C,
            ns: &'a str,
            api: &'a APIInfo,
            kind: &'a str,
        }

        impl<'a, C: KubeClientRequest> FetchResourceListSingleNamespace<'a, C> {
            pub(super) fn new(client: &'a C, ns: &'a str, api: &'a APIInfo, kind: &'a str) -> Self {
                Self {
                    client,
                    ns,
                    api,
                    kind,
                }
            }

            pub(super) async fn fetch(&self) -> Result<Vec<YamlResourceListItem>> {
                let path = format!(
                    "{}/namespaces/{}/{}",
                    self.api.api_url(),
                    self.ns,
                    self.kind
                );

                logger!(info, "Fetching resource [{}]", path);

                let res: List = self.client.request(&path).await?;

                logger!(info, "Fetched resource - {:?}", res);

                Ok(res
                    .items
                    .into_iter()
                    .filter_map(|item| {
                        item.metadata.name.map(|name| YamlResourceListItem {
                            namespace: self.ns.to_string(),
                            name: name.to_string(),
                            kind: self.api.resource_full_name(),
                            value: name,
                        })
                    })
                    .collect())
            }
        }
    }

    mod multiple_namespace {

        use anyhow::Result;
        use futures::future::try_join_all;
        use unicode_segmentation::UnicodeSegmentation;

        use crate::event::kubernetes::{
            api_resources::APIInfo, client::KubeClientRequest, yaml::YamlResourceListItem,
        };

        use super::single_namespace::FetchResourceListSingleNamespace;

        pub(super) struct FetchResourceListMultipleNamespaces<'a, C: KubeClientRequest> {
            client: &'a C,
            namespaces: &'a [String],
            api: &'a APIInfo,
            kind: &'a str,
        }

        impl<'a, C: KubeClientRequest> FetchResourceListMultipleNamespaces<'a, C> {
            pub(super) fn new(
                client: &'a C,
                namespaces: &'a [String],
                api: &'a APIInfo,
                kind: &'a str,
            ) -> Self {
                Self {
                    client,
                    namespaces,
                    api,
                    kind,
                }
            }

            pub(super) async fn fetch(&self) -> Result<Vec<YamlResourceListItem>> {
                let jobs = try_join_all(self.namespaces.iter().map(|ns| async move {
                    FetchResourceListSingleNamespace::new(self.client, ns, self.api, self.kind)
                        .fetch()
                        .await
                }))
                .await?;

                let namespace_digit = self
                    .namespaces
                    .iter()
                    .map(|ns| ns.graphemes(true).count())
                    .max()
                    .unwrap_or(0);

                let list = jobs
                    .into_iter()
                    .flat_map(|items| {
                        items
                            .into_iter()
                            .map(|mut item| {
                                item.value = format!(
                                    "{:digit$}  {}",
                                    item.namespace,
                                    item.name,
                                    digit = namespace_digit
                                );
                                item
                            })
                            .collect::<Vec<YamlResourceListItem>>()
                    })
                    .collect();

                Ok(list)
            }
        }
    }

    pub struct FetchResourceList<'a, C: KubeClientRequest> {
        client: &'a C,
        req: String,
        target_namespaces: &'a TargetNamespaces,
        api_resources: &'a ApiResources,
    }

    impl<'a, C: KubeClientRequest> FetchResourceList<'a, C> {
        pub fn new(
            client: &'a C,
            req: String,
            api_resources: &'a ApiResources,
            target_namespaces: &'a TargetNamespaces,
        ) -> Self {
            Self {
                client,
                req,
                api_resources,
                target_namespaces,
            }
        }

        /// 選択されているリソースのリストを取得する
        ///
        /// ネームスペースが１つのとき OR namespaced が false のとき
        ///   リソース一覧を返す
        ///
        /// ネームスペースが２つ以上のとき
        ///   ネームスペースを頭につけたリソース一覧を返す
        ///
        pub async fn fetch(&self) -> Result<YamlResourceList> {
            let kind = &self.req;

            let api = self
                .api_resources
                .get(kind)
                .ok_or_else(|| Error::Raw(format!("Can't get {} from API resource", kind)))?;

            let kind = &api.api_resource.name;
            let list = if api.api_resource.namespaced {
                if self.target_namespaces.len() == 1 {
                    FetchResourceListSingleNamespace::new(
                        self.client,
                        &self.target_namespaces[0],
                        api,
                        kind,
                    )
                    .fetch()
                    .await?
                } else {
                    FetchResourceListMultipleNamespaces::new(
                        self.client,
                        self.target_namespaces,
                        api,
                        kind,
                    )
                    .fetch()
                    .await?
                }
            } else {
                FetchResourceListNotNamespaced::new(self.client, api, kind)
                    .fetch()
                    .await?
            };

            Ok(YamlResourceList::new(list))
        }
    }
}

pub mod worker {
    use crate::logger;

    use super::*;

    #[derive(Debug, Clone)]
    pub struct YamlWorkerRequest {
        pub namespace: String,
        pub kind: String,
        pub name: String,
    }

    #[derive(Debug, Clone)]
    pub struct YamlWorker<C>
    where
        C: KubeClientRequest,
    {
        is_terminated: Arc<AtomicBool>,
        tx: Sender<Event>,
        client: C,
        req: YamlWorkerRequest,
        shared_api_resources: SharedApiResources,
    }

    impl<C: KubeClientRequest> YamlWorker<C> {
        pub fn new(
            is_terminated: Arc<AtomicBool>,
            tx: Sender<Event>,
            client: C,
            shared_api_resources: SharedApiResources,
            req: YamlWorkerRequest,
        ) -> Self {
            Self {
                is_terminated,
                tx,
                client,
                req,
                shared_api_resources,
            }
        }
    }

    #[async_trait::async_trait]
    impl<C: KubeClientRequest> Worker for YamlWorker<C> {
        type Output = Result<()>;

        async fn run(&self) -> Self::Output {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(3));

            let YamlWorkerRequest {
                kind,
                name,
                namespace,
            } = &self.req;

            while !self
                .is_terminated
                .load(std::sync::atomic::Ordering::Relaxed)
            {
                interval.tick().await;

                let api_resources = self.shared_api_resources.read().await;

                let fetched_data = fetch_resource_yaml(
                    &self.client,
                    &api_resources,
                    kind.to_string(),
                    name.to_string(),
                    namespace.to_string(),
                )
                .await;

                self.tx.send(YamlResponse::Yaml(fetched_data).into())?;
            }

            Ok(())
        }
    }

    /// 選択されているリソースのyamlを取得する
    pub async fn fetch_resource_yaml<C: KubeClientRequest>(
        client: &C,
        api_resources: &ApiResources,
        kind: String,
        name: String,
        ns: String,
    ) -> Result<Vec<String>> {
        logger!(
            info,
            "Fetching resource target [kind={} ns={} name={}]",
            kind,
            ns,
            name
        );

        let api = api_resources
            .get(&kind)
            .ok_or_else(|| Error::Raw(format!("Can't get {} from API resource", kind)))?;

        // json string data
        let kind = &api.api_resource.name;
        let path = if api.api_resource.namespaced {
            format!("{}/namespaces/{}/{}/{}", api.api_url(), ns, kind, name)
        } else {
            format!("{}/{}/{}", api.api_url(), kind, name)
        };

        logger!(info, "Fetching resource [{}]", path);

        let res = client.request_text(&path).await?;

        logger!(info, "Fetched resource - {}", res);

        // yaml dataに変換
        let yaml_data: serde_yaml::Value = serde_json::from_str(&res)?;

        let yaml_string = serde_yaml::to_string(&yaml_data)?
            .lines()
            .skip(1)
            .map(ToString::to_string)
            .collect();

        Ok(yaml_string)
    }
}
