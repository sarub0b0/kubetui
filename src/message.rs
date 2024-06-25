use ratatui::crossterm::event::{KeyCode, KeyEvent, MouseEvent};

use crate::workers::kube::message::Kube;

#[derive(Debug, PartialEq, Clone, Copy, Eq, Hash)]
pub enum UserEvent {
    Key(KeyEvent),
    Mouse(MouseEvent),
    FocusGained,
    FocusLost,
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

impl From<UserEvent> for Message {
    fn from(value: UserEvent) -> Self {
        Self::User(value)
    }
}

#[derive(Debug)]
pub enum Message {
    Kube(Kube),
    User(UserEvent),
    Tick,
    Error(anyhow::Error),
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
