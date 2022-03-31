use async_trait::async_trait;
use serde::de::DeserializeOwned;

use http::header::{HeaderValue, ACCEPT};
use kube::{api::Request, Client};

use crate::error::{anyhow, Error, Result};

const TABLE_REQUEST_HEADER: &str = "application/json;as=Table;v=v1;g=meta.k8s.io,application/json;as=Table;v=v1beta1;g=meta.k8s.io,application/json";

fn remove_slash(path: &str) -> &str {
    if let Some(path) = path.strip_prefix('/') {
        path
    } else {
        path
    }
}

#[derive(Clone)]
pub struct KubeClient {
    client: Client,
    server_url: String,
}

impl KubeClient {
    pub fn new(client: Client, server_url: impl Into<String>) -> Self {
        Self {
            client,
            server_url: server_url.into(),
        }
    }

    pub fn client_clone(&self) -> Client {
        self.client.clone()
    }

    #[allow(dead_code)]
    pub fn as_client(&self) -> &Client {
        &self.client
    }

    #[allow(dead_code)]
    pub fn as_mut_client(&mut self) -> &mut Client {
        &mut self.client
    }

    #[allow(dead_code)]
    pub fn as_server_url(&self) -> &String {
        &self.server_url
    }

    #[allow(dead_code)]
    pub fn as_mut_server_url(&mut self) -> &mut String {
        &mut self.server_url
    }

    async fn inner_request<T>(&self, path: &str, header: &str) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let request = Request::new(&self.server_url);

        let mut request = request.get(remove_slash(path))?;

        request
            .headers_mut()
            .insert(ACCEPT, HeaderValue::from_str(header)?);

        #[cfg(feature = "logging")]
        ::log::debug!("HTTP request {:?}", request);

        let ret = self.client.request(request).await;

        ret.map_err(|e| anyhow!(Error::Kube(e)))
    }
}

#[async_trait]
pub trait KubeClientRequest: Send + Sync {
    async fn table_request<T: DeserializeOwned + 'static>(&self, path: &str) -> Result<T>;
    async fn request<T: DeserializeOwned + 'static>(&self, path: &str) -> Result<T>;

    async fn request_text(&self, path: &str) -> Result<String>;
}

#[async_trait]
impl KubeClientRequest for KubeClient {
    async fn table_request<T: DeserializeOwned + 'static>(&self, path: &str) -> Result<T> {
        self.inner_request(path, TABLE_REQUEST_HEADER).await
    }

    async fn request<T: DeserializeOwned + 'static>(&self, path: &str) -> Result<T> {
        self.inner_request(path, "application/json").await
    }

    async fn request_text(&self, path: &str) -> Result<String> {
        let request = Request::new(&self.server_url);

        let mut request = request.get(remove_slash(path))?;

        request
            .headers_mut()
            .insert(ACCEPT, HeaderValue::from_str("application/json")?);

        #[cfg(feature = "logging")]
        ::log::debug!("HTTP request {:?}", request);

        let ret = self.client.request_text(request).await;

        ret.map_err(|e| anyhow!(Error::Kube(e)))
    }
}

#[cfg(test)]
pub mod mock {
    use super::{DeserializeOwned, KubeClientRequest, Result};
    use mockall::mock;

    mock! {
        pub TestKubeClient {}
        impl Clone for TestKubeClient {
            fn clone(&self) -> Self;
        }

        #[async_trait::async_trait]
        impl KubeClientRequest for TestKubeClient {
            async fn table_request<T: DeserializeOwned + 'static>(&self, path: &str) -> Result<T>;
            async fn request<T: DeserializeOwned + 'static>(&self, path: &str) -> Result<T>;
            async fn request_text(&self, path: &str) -> Result<String>;
        }
    }

    #[macro_export]
    macro_rules! mock_expect {
        ($client:ident, request, [$(($ty:ty, $with:expr, $ret:expr)),*]) => {
            $(
                $client.expect_request::<$ty>().with($with).returning(|_| $ret);
            )*
        };
        ($client:ident, table_request, [$(($ty:ty, $with:expr, $ret:expr)),*]) => {
            $(
                $client.expect_table_request::<$ty>().with($with).returning(|_| $ret);
            )*
        };
        ($client:ident, request_text, [$(($with:expr, $ret:expr)),*]) => {
            $(
                $client.expect_request_text().with($with).returning(|_| $ret);
            )*
        };

        ($client:ident, request, $ty:ty, $with:expr, $ret:expr) => {
            $client.expect_request::<$ty>().with($with).returning(|_| $ret);
        };
        ($client:ident, table_request, $ty:ty, $with:expr, $ret:expr) => {
            $client.expect_table_request::<$ty>().with($with).returning(|_| $ret);
        };
        ($client:ident, request_text, $with:expr, $ret:expr) => {
            $client.expect_request_text().with($with).returning(|_| $ret);
        };
    }
}
