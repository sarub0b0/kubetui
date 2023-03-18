mod data;
mod list;

pub use data::get_config;
pub use list::ConfigsPollWorker;

use super::{Event, Kube, KubeClient, KubeTable};

use crate::error::Result;

#[derive(Debug)]
pub enum ConfigMessage {
    List(Result<KubeTable>),
    DataRequest {
        namespace: String,
        kind: String,
        name: String,
    },
    DataResponse(Result<Vec<String>>),
}

impl From<ConfigMessage> for Kube {
    fn from(msg: ConfigMessage) -> Self {
        Kube::Config(msg)
    }
}

impl From<ConfigMessage> for Event {
    fn from(msg: ConfigMessage) -> Self {
        Event::Kube(msg.into())
    }
}
