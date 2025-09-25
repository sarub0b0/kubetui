use anyhow::Result;
use async_trait::async_trait;
use http::header::ACCEPT;
use kube::{core::request::Error as KubeRequestError, Client};
use serde::de::DeserializeOwned;

use crate::logger;

const TABLE_REQUEST_HEADER: &str = "application/json;as=Table;v=v1;g=meta.k8s.io,application/json;as=Table;v=v1beta1;g=meta.k8s.io,application/json";

#[derive(Clone)]
pub struct KubeClient {
    client: Client,
    server_url: String,
}

impl KubeClient {
    pub fn new(client: Client, server_url: impl Into<String>) -> Self {
        let url: String = server_url.into();
        let server_url = if let Some(url) = url.strip_suffix('/') {
            url.to_string()
        } else {
            url
        };
        Self { client, server_url }
    }

    #[allow(dead_code)]
    pub fn as_client(&self) -> &Client {
        &self.client
    }

    pub fn to_client(&self) -> Client {
        self.client.clone()
    }

    #[allow(dead_code)]
    pub fn as_server_url(&self) -> &String {
        &self.server_url
    }
}

#[async_trait]
pub trait KubeClientRequest: Send + Sync {
    async fn request_table<T: DeserializeOwned + 'static>(&self, url_path: &str) -> Result<T>;

    async fn request<T: DeserializeOwned + 'static>(&self, url_path: &str) -> Result<T>;

    async fn request_text(&self, url_path: &str) -> Result<String>;

    fn client(&self) -> &Client;
}

#[async_trait]
impl KubeClientRequest for KubeClient {
    async fn request_table<T: DeserializeOwned + 'static>(&self, url_path: &str) -> Result<T> {
        logger!(debug, "Requesting table from path: {}", url_path);

        let request = http::Request::get(url_path)
            .header(ACCEPT, TABLE_REQUEST_HEADER)
            .body(vec![])
            .map_err(KubeRequestError::BuildRequest)
            .inspect_err(|e| logger!(error, "Failed to build request ({}): {}", url_path, e))?;

        logger!(debug, "HTTP request {:?}", request);

        let ret = self.client.request(request).await;

        ret.map_err(Into::into)
            .inspect_err(|e| logger!(error, "Request error ({}): {}", url_path, e))
    }

    async fn request<T: DeserializeOwned + 'static>(&self, url_path: &str) -> Result<T> {
        logger!(debug, "Requesting from path: {}", url_path);

        let request = http::Request::get(url_path)
            .header(ACCEPT, "application/json")
            .body(vec![])
            .map_err(KubeRequestError::BuildRequest)
            .inspect_err(|e| logger!(error, "Failed to build request ({}): {}", url_path, e))?;

        logger!(debug, "HTTP request {:?}", request);

        let ret = self.client.request(request).await;

        ret.map_err(Into::into)
            .inspect_err(|e| logger!(error, "Request error ({}): {}", url_path, e))
    }

    async fn request_text(&self, url_path: &str) -> Result<String> {
        logger!(debug, "Requesting text from path: {}", url_path);

        let request = http::Request::get(url_path)
            .header(ACCEPT, "application/json")
            .body(vec![])
            .map_err(KubeRequestError::BuildRequest)
            .inspect_err(|e| logger!(error, "Failed to build request ({}): {}", url_path, e))?;

        logger!(debug, "HTTP request {:?}", request);

        let ret = self.client.request_text(request).await;

        logger!(debug, "Response text: {:?}", ret);

        ret.map_err(Into::into)
            .inspect_err(|e| logger!(error, "Request error ({}): {}", url_path, e))
    }

    fn client(&self) -> &Client {
        &self.client
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
            async fn request_table<T: DeserializeOwned + 'static>(&self, url_path: &str) -> Result<T>;
            async fn request<T: DeserializeOwned + 'static>(&self, url_path: &str) -> Result<T>;
            async fn request_text(&self, url_path: &str) -> Result<String>;
            fn client(&self) -> &kube::Client;
        }
    }

    #[macro_export]
    macro_rules! mock_expect {
        ($client:ident, request, [$(($ty:ty, $with:expr, $ret:expr)),*]) => {
            $(
                $client.expect_request::<$ty>().with($with).returning(|_| $ret);
            )*
        };
        ($client:ident, request_table, [$(($ty:ty, $with:expr, $ret:expr)),*]) => {
            $(
                $client.expect_request_table::<$ty>().with($with).returning(|_| $ret);
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
        ($client:ident, request_table, $ty:ty, $with:expr, $ret:expr) => {
            $client.expect_request_table::<$ty>().with($with).returning(|_| $ret);
        };
        ($client:ident, request_text, $with:expr, $ret:expr) => {
            $client.expect_request_text().with($with).returning(|_| $ret);
        };
    }
}
