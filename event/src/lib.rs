pub mod input;
pub mod tick;

pub mod kubernetes;

mod util;

use crate::kubernetes::Kube;
use crossterm::event::{KeyEvent, MouseEvent};

pub enum Event {
    Input(KeyEvent),
    Kube(Kube),
    Tick,
    Resize(u16, u16),
    Mouse(MouseEvent),
}
