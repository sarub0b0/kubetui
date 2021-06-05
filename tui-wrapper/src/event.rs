use std::rc::Rc;

use crate::WindowEvent;

use super::Window;

pub struct Callback(Rc<dyn Fn(&mut Window)>);

impl Callback {
    pub fn from_fn<F>(f: F) -> Callback
    where
        F: 'static + Fn(&mut Window),
    {
        Callback(Rc::new(move |win| {
            f(win);
        }))
    }
}

impl From<Rc<dyn Fn(&mut Window)>> for Callback {
    fn from(f: Rc<dyn Fn(&mut Window)>) -> Callback {
        Self(f)
    }
}

impl std::ops::Deref for Callback {
    type Target = dyn Fn(&mut Window) + 'static;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

pub enum EventResult {
    Nop,
    Ignore,
    Callback(Option<Callback>),
    WindowEvent(WindowEvent),
}

impl EventResult {
    pub fn exec(&self, w: &mut Window) {
        if let Self::Callback(Some(cb)) = self {
            cb(w)
        }
    }
}
