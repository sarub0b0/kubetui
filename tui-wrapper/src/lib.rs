use std::rc::Rc;

pub mod complex_widgets;
pub mod pane;
pub mod sub_window;
pub mod tab;
pub mod widget;
pub mod window;

mod util;

use util::*;

pub use complex_widgets::{MultipleSelect, SingleSelect};
pub use pane::Pane;
pub use sub_window::SubWindow;
pub use tab::Tab;
pub use util::key_event_to_code;
pub use window::*;

pub use crossterm;
pub use tui;

pub struct Callback(Rc<dyn Fn(&mut Window)>);

impl Callback {
    fn from_fn<F>(f: F) -> Callback
    where
        F: 'static + Fn(&mut Window),
    {
        Callback(Rc::new(move |win| {
            f(win);
        }))
    }
}

impl std::ops::Deref for Callback {
    type Target = dyn Fn(&mut Window) + 'static;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

pub struct EventResult {
    pub cb: Option<Callback>,
}

impl EventResult {
    pub fn none() -> Self {
        Self { cb: None }
    }
}
