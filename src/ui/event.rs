use crate::define_callback;

use super::{Window, WindowAction};

define_callback!(pub Callback, Fn(&mut Window) -> EventResult);

pub enum EventResult {
    Nop,
    Ignore,
    Callback(Callback),
    WindowAction(WindowAction),
}
