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
    LogStreamRequest(String),
    LogStreamResponse(Vec<String>),
    // ConfigMap & Secret
    Configs(Vec<String>),
    ConfigRequest(String),
    ConfigResponse(Vec<String>),
}
