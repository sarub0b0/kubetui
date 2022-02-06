pub mod input;
pub mod tick;

pub mod kubernetes;

mod util;

use crossterm::event::{KeyCode, KeyEvent, MouseEvent};

use crate::error::Error;

use self::kubernetes::Kube;

#[derive(Debug, PartialEq, Clone, Copy, Eq, Hash)]
pub enum UserEvent {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize(u16, u16),
}

impl UserEvent {
    pub fn from_key(code: KeyCode) -> Self {
        UserEvent::Key(KeyEvent::from(code))
    }
}

impl From<char> for UserEvent {
    fn from(c: char) -> Self {
        UserEvent::Key(KeyEvent::from(KeyCode::Char(c)))
    }
}
impl From<KeyCode> for UserEvent {
    fn from(code: KeyCode) -> Self {
        UserEvent::Key(KeyEvent::from(code))
    }
}
pub enum Event {
    Kube(Kube),
    User(UserEvent),
    Tick,
    Error(Error),
}

#[macro_export]
macro_rules! panic_set_hook {
    ($t:tt) => {
        use std::panic;
        let default_hook = panic::take_hook();

        panic::set_hook(Box::new(move |info| {
            $t;

            default_hook(info);
        }));
    };
}

// #[macro_export]
// macro_rules! log {
//     ($($arg:tt)+) => {
//         #[cfg(feature = "logging")]
//         ::log::error!($($arg)+);
//     };
// }
