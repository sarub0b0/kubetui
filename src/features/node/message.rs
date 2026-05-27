use anyhow::Result;

use crate::{kube::table::KubeTable, message::Message, workers::kube::message::Kube};

use super::NodeColumns;

#[derive(Debug)]
pub enum NodeMessage {
    Request(NodeColumns),
    Poll(Result<KubeTable>),
    /// Replace the active labelSelector value. `None` clears it (the
    /// poller stops sending ?labelSelector= in its request URL).
    Filter(Option<String>),
}

impl From<NodeMessage> for Message {
    fn from(m: NodeMessage) -> Message {
        Message::Kube(Kube::Node(m))
    }
}

/// Messages for the per-selection Node detail worker.
#[derive(Debug)]
pub enum NodeDetailMessage {
    /// Start (or restart) the detail worker for the given node name.
    Request { name: String },
    /// A tick of YAML + related-Pods lines (or an error).
    Response(Result<Vec<String>>),
}

impl From<NodeDetailMessage> for Message {
    fn from(m: NodeDetailMessage) -> Message {
        Message::Kube(Kube::NodeDetail(m))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_detail_message_request_converts_into_kube_message() {
        let msg: Message = NodeDetailMessage::Request {
            name: "node-a".to_string(),
        }
        .into();
        assert!(matches!(
            msg,
            Message::Kube(Kube::NodeDetail(NodeDetailMessage::Request { .. }))
        ));
    }
}
