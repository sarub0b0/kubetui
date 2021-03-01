pub mod list;
pub mod namespace;
pub mod text;

pub use self::list::*;
pub use self::namespace::*;
pub use self::text::*;

use std::cell::RefCell;
use std::rc::Rc;

#[derive(Copy, Clone, PartialEq)]
pub enum Type {
    NONE,
    LOG,
    POD,
    NS,
}

pub trait WidgetTrait {
    fn selectable(&self) -> bool;
    fn select_next(&mut self, index: usize);
    fn select_prev(&mut self, index: usize);
    fn select_first(&mut self);
    fn select_last(&mut self);
    fn set_items(&mut self, items: Vec<String>);
}

pub enum Widget<'a> {
    Pod(List<'a>),
    Log(Text<'a>),
    Namespace(List<'a>),
}

impl<'a> Widget<'a> {
    pub fn pod(&self) -> Option<&List> {
        match self {
            Widget::Pod(pod) => Some(pod),
            _ => None,
        }
    }

    pub fn log(&self) -> Option<&Text> {
        match self {
            Widget::Log(log) => Some(log),
            _ => None,
        }
    }

    pub fn pod_mut(&mut self) -> Option<&mut List<'a>> {
        match self {
            Widget::Pod(pod) => Some(pod),
            _ => None,
        }
    }

    pub fn log_mut(&mut self) -> Option<&mut Text<'a>> {
        match self {
            Widget::Log(log) => Some(log),
            _ => None,
        }
    }

    pub fn namespace(&self) -> Option<&List> {
        match self {
            Widget::Namespace(ns) => Some(ns),
            _ => None,
        }
    }
    pub fn namespace_mut(&mut self) -> Option<&mut List<'a>> {
        match self {
            Widget::Namespace(ns) => Some(ns),
            _ => None,
        }
    }
}

impl WidgetTrait for Widget<'_> {
    fn selectable(&self) -> bool {
        match self {
            Widget::Pod(pod) => pod.selectable(),
            Widget::Log(log) => log.selectable(),
            Widget::Namespace(ns) => ns.selectable(),
        }
    }

    fn select_next(&mut self, index: usize) {
        match self {
            Widget::Pod(pod) => pod.select_next(index),
            Widget::Log(log) => log.select_next(index),
            Widget::Namespace(ns) => ns.select_next(index),
        }
    }

    fn select_prev(&mut self, index: usize) {
        match self {
            Widget::Pod(pod) => pod.select_prev(index),
            Widget::Log(log) => log.select_prev(index),
            Widget::Namespace(ns) => ns.select_prev(index),
        }
    }

    fn select_first(&mut self) {
        match self {
            Widget::Pod(pod) => pod.select_first(),
            Widget::Log(log) => log.select_first(),
            Widget::Namespace(ns) => ns.select_first(),
        }
    }

    fn select_last(&mut self) {
        match self {
            Widget::Pod(pod) => pod.select_last(),
            Widget::Log(log) => log.select_last(),
            Widget::Namespace(ns) => ns.select_last(),
        }
    }

    fn set_items(&mut self, items: Vec<String>) {
        match self {
            Widget::Pod(pod) => pod.set_items(items),
            Widget::Log(log) => log.set_items(items),
            Widget::Namespace(ns) => ns.set_items(items),
        }
    }
}
