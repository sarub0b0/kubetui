use anyhow::Result;

use crate::{kube::table::KubeTable, message::Message, workers::kube::message::Kube};

use super::NodeColumns;

#[derive(Debug)]
pub enum NodeMessage {
    Request(NodeColumns),
    Poll(Result<KubeTable>),
}

impl From<NodeMessage> for Message {
    fn from(m: NodeMessage) -> Message {
        Message::Kube(Kube::Node(m))
    }
}
