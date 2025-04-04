use anyhow::Result;

use crate::{features::StyledApiResource, message::Message, workers::kube::message::Kube};

use super::kube::ApiResource;

#[derive(Debug)]
pub enum ApiRequest {
    Get,
    Set(Vec<ApiResource>),
}

#[derive(Debug)]
pub enum ApiResponse {
    Get(Result<Vec<StyledApiResource>>),
    Poll(Result<Vec<String>>),
}

#[derive(Debug)]
pub enum ApiMessage {
    Request(ApiRequest),
    Response(ApiResponse),
}

impl From<ApiRequest> for Message {
    fn from(f: ApiRequest) -> Self {
        Self::Kube(Kube::Api(ApiMessage::Request(f)))
    }
}

impl From<ApiResponse> for Message {
    fn from(f: ApiResponse) -> Self {
        Self::Kube(Kube::Api(ApiMessage::Response(f)))
    }
}

impl From<ApiMessage> for Kube {
    fn from(f: ApiMessage) -> Self {
        Self::Api(f)
    }
}

impl From<ApiMessage> for Message {
    fn from(f: ApiMessage) -> Self {
        Self::Kube(f.into())
    }
}
