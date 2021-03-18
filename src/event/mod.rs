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
    Pod(Vec<String>),
    GetNamespaceRequest,
    GetNamespaceResponse(Option<Vec<String>>),
    SetNamespace(String),
    // Pod Logs
    LogRequest(String),
    LogResponse(Vec<String>),
    // ConfigMap & Secret
    StartConfigsGet,
    StopConfigsGet,
    Configs(Vec<String>),
}
