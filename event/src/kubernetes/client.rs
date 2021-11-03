use serde::de::DeserializeOwned;

use http::header::{HeaderValue, ACCEPT};
use kube::{api::Request, Client};

use crate::error::{anyhow, Error, Result};

const TABLE_REQUEST_HEADER: &str = "application/json;as=Table;v=v1;g=meta.k8s.io,application/json;as=Table;v=v1beta1;g=meta.k8s.io,application/json";

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

    #[allow(dead_code)]
    pub async fn table_request<T>(&self, path: &str) -> Result<T>
    where
        T: DeserializeOwned,
    {
        self.inner_request(path, TABLE_REQUEST_HEADER).await
    }

    pub async fn request<T>(&self, path: &str) -> Result<T>
    where
        T: DeserializeOwned,
    {
        self.inner_request(path, "application/json").await
    }

    async fn inner_request<T>(&self, path: &str, header: &str) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let request = Request::new(&self.server_url);

        let mut request = request.get(path)?;

        request
            .headers_mut()
            .insert(ACCEPT, HeaderValue::from_str(header)?);

        #[cfg(feature = "logging")]
        ::log::debug!("HTTP request {:?}", request);

        let ret = self.client.request(request).await;

        ret.map_err(|e| anyhow!(Error::Kube(e)))
    }

    pub async fn request_text(&self, path: &str) -> Result<String> {
        let request = Request::new(&self.server_url);

        let mut request = request.get(path)?;

        request
            .headers_mut()
            .insert(ACCEPT, HeaderValue::from_str("application/json")?);

        #[cfg(feature = "logging")]
        ::log::debug!("HTTP request {:?}", request);

        let ret = self.client.request_text(request).await;

        ret.map_err(|e| anyhow!(Error::Kube(e)))
    }
}
