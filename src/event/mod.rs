pub mod input;
pub mod kubernetes;
pub mod tick;

use crossterm::event::KeyEvent;

pub enum Event {
    Input(KeyEvent),
    Kube(Kube),
    Tick,
    Resize(u16, u16),
    Mouse,
}

pub enum Kube {
    GetNamespaceRequest,
    GetNamespaceResponse(Option<Vec<String>>),
    SetNamespace(String),
    // Pod Logs
    Pod(Vec<String>),
    LogRequest(String),
    LogResponse(Vec<String>),
    LogStreamRequest(String),
    LogStreamResponse(String),
    // ConfigMap & Secret
    Configs(Vec<String>),
    ConfigRequest(String),
    ConfigResponse(Vec<String>),
}
