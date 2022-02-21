mod description;
mod list;

pub use description::*;
pub use list::*;

use std::time;

use async_trait::async_trait;
use crossbeam::channel::Sender;
use futures::future::try_join_all;

use crate::{
    error::{anyhow, Error, Result},
    event::Event,
};

use super::{
    client::KubeClient,
    v1_table::{get_resource_per_namespace, insert_ns, TableRow},
    worker::{PollWorker, Worker},
    Kube, KubeTable, WorkerResult,
};

#[derive(Debug, Clone)]
pub struct RequestData {
    pub name: String,
    pub namespace: String,
}

#[derive(Debug, Clone)]
pub enum Request {
    Pod(RequestData),
    Service(RequestData),
    Ingress(RequestData),
    NetworkPolicy(RequestData),
}

#[derive(Debug)]
pub enum NetworkMessage {
    Poll(Result<KubeTable>),
    Request(Request),
    Response(Result<Vec<String>>),
}

impl From<NetworkMessage> for Kube {
    fn from(m: NetworkMessage) -> Self {
        Self::Network(m)
    }
}

impl From<NetworkMessage> for Event {
    fn from(m: NetworkMessage) -> Self {
        Self::Kube(m.into())
    }
}
