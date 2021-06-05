pub mod input;
pub mod tick;

pub mod kubernetes;

mod util;

use crate::kubernetes::Kube;
use crossterm::event::{KeyEvent, MouseEvent};

pub enum UserEvent {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize(u16, u16),
}

pub enum Event {
    Kube(Kube),
    User(UserEvent),
    Tick,
}
