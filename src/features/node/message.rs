use anyhow::Result;

use crate::{kube::table::KubeTable, message::Message, workers::kube::message::Kube};

#[derive(Debug)]
pub enum NodeMessage {
    Poll(Result<KubeTable>),
}

impl From<NodeMessage> for Message {
    fn from(m: NodeMessage) -> Message {
        Message::Kube(Kube::Node(m))
    }
}
