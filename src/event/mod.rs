pub mod input;
pub mod kubernetes;
pub mod tick;

use crossterm::event::KeyEvent;

use bytes::Bytes;
use futures::{Stream, StreamExt, TryStream};

use kube::Result;

pub enum Event {
    Input(KeyEvent),
    Kube(Kube),
    Tick,
    Resize,
    Mouse,
    Render(Render),
}

pub enum Kube {
    Pod(Vec<String>),
    Namespace(Option<Vec<String>>),
    LogRequest(String),
    LogResponse(Vec<String>),
}

pub enum Render {
    Tab,
    DateTime,
    Panes,
}
