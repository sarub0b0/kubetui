pub mod error;
pub mod input;
pub mod tick;

pub mod kubernetes;

mod util;

use self::kubernetes::Kube;
use crossterm::event::{KeyCode, KeyEvent, MouseEvent};

#[derive(Debug, PartialEq, Clone, Copy)]
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
    Error(error::Error),
}

use std::panic;
#[macro_export]
macro_rules! panic_set_hook {
    ($t:tt) => {
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
