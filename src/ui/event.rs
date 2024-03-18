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
    Callback(Callback),
    WindowAction(WindowAction),
}
