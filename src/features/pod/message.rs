use anyhow::Result;

use crate::{kube::table::KubeTable, message::Message, workers::kube::message::Kube};

use super::{kube::LogConfig, PodColumns};

#[derive(Debug)]
pub enum LogMessage {
    Request(LogConfig),
    Response(Result<Vec<String>>),
    ToggleJsonPrettyPrint,
    SetMaxLines(Option<usize>),
    StreamError(String),
    /// Non-fatal informational notice tied to a namespace. Used to surface
    /// per-namespace setup-time failures (e.g. resource not found) without
    /// failing the whole log query when multiple namespaces are selected.
    /// Rendered as a yellow inline line in the log view with `[kubetui]`
    /// prefix.
    Notice {
        namespace: String,
        message: String,
    },
}

impl From<LogMessage> for Message {
    fn from(m: LogMessage) -> Message {
        Message::Kube(Kube::Log(m))
    }
}

#[derive(Debug)]
pub enum PodMessage {
    Request(PodColumns),
    Poll(Result<KubeTable>),
    Filter(Option<String>),
}

impl From<PodMessage> for Message {
    fn from(m: PodMessage) -> Message {
        Message::Kube(Kube::Pod(m))
    }
}
