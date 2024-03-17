use std::rc::Rc;

use super::{Window, WindowAction};

pub(super) type InnerCallback = Rc<dyn Fn(&mut Window) -> EventResult>;

pub struct Callback(Rc<dyn Fn(&mut Window) -> EventResult>);

impl Callback {
    pub fn from_fn<F>(f: F) -> Callback
    where
        F: 'static + Fn(&mut Window) -> EventResult,
    {
        Callback(Rc::new(move |win| f(win)))
    }
}

impl From<Rc<dyn Fn(&mut Window) -> EventResult>> for Callback {
    fn from(f: Rc<dyn Fn(&mut Window) -> EventResult>) -> Callback {
        Self(f)
    }
}

impl std::ops::Deref for Callback {
    type Target = dyn Fn(&mut Window) -> EventResult + 'static;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

pub enum EventResult {
    Nop,
    Ignore,
    Callback(Option<Callback>),
    WindowAction(WindowAction),
}

impl EventResult {
    pub fn exec(&self, w: &mut Window) -> EventResult {
        if let Self::Callback(Some(cb)) = self {
            cb(w)
        } else {
            EventResult::Ignore
        }
    }
}

pub fn exec_to_window_event(ev: EventResult, w: &mut Window) -> WindowAction {
    match ev {
        EventResult::Nop => {}
        EventResult::Ignore => {}
        ev @ EventResult::Callback(_) => {
            return exec_to_window_event(ev.exec(w), w);
        }
        EventResult::WindowAction(ev) => return ev,
    }
    WindowAction::Continue
}
