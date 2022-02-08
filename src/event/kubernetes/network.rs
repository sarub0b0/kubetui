use crate::{error::Result, event::Event};

use super::{Kube, KubeTable};

type Name = String;
type Namespace = String;

pub enum Request {
    Pod(Namespace, Name),
    Service(Namespace, Name),
    Ingress(Namespace, Name),
}

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
