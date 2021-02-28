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
    Namespace(Option<Vec<String>>),
    LogRequest(String),
    LogResponse(Vec<String>),
}
