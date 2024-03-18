use std::rc::Rc;

use super::{Window, WindowAction};

pub trait CallbackFn: Fn(&mut Window) -> EventResult + 'static {}

impl<S> CallbackFn for S where S: Fn(&mut Window) -> EventResult + 'static {}

#[derive(Clone)]
pub struct Callback(Rc<dyn CallbackFn>);

impl Callback {
    pub fn new<F>(f: F) -> Callback
    where
        F: CallbackFn,
    {
        Callback(Rc::new(move |win| f(win)))
    }
}

impl From<Rc<dyn CallbackFn>> for Callback {
    fn from(f: Rc<dyn CallbackFn>) -> Callback {
        Self(f)
    }
}

impl std::ops::Deref for Callback {
    type Target = dyn CallbackFn;

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
