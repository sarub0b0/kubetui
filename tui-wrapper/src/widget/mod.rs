pub mod list;
pub mod text;

pub use self::list::*;
pub use self::text::*;

pub trait WidgetTrait {
    fn selectable(&self) -> bool;
    fn select_next(&mut self, index: usize);
    fn select_prev(&mut self, index: usize);
    fn select_first(&mut self);
    fn select_last(&mut self);
    fn set_items(&mut self, items: Vec<String>);
}

pub enum Widget<'a> {
    List(List<'a>),
    Text(Text<'a>),
}

impl<'a> Widget<'a> {
    pub fn list(&self) -> Option<&List> {
        match self {
            Widget::List(list) => Some(list),
            _ => None,
        }
    }

    pub fn list_mut(&mut self) -> Option<&mut List<'a>> {
        match self {
            Widget::List(list) => Some(list),
            _ => None,
        }
    }

    pub fn text(&self) -> Option<&Text> {
        match self {
            Widget::Text(text) => Some(text),
            _ => None,
        }
    }

    pub fn text_mut(&mut self) -> Option<&mut Text<'a>> {
        match self {
            Widget::Text(text) => Some(text),
            _ => None,
        }
    }
}

impl WidgetTrait for Widget<'_> {
    fn selectable(&self) -> bool {
        match self {
            Widget::List(pod) => pod.selectable(),
            Widget::Text(log) => log.selectable(),
        }
    }

    fn select_next(&mut self, index: usize) {
        match self {
            Widget::List(pod) => pod.select_next(index),
            Widget::Text(log) => log.select_next(index),
        }
    }

    fn select_prev(&mut self, index: usize) {
        match self {
            Widget::List(pod) => pod.select_prev(index),
            Widget::Text(log) => log.select_prev(index),
        }
    }

    fn select_first(&mut self) {
        match self {
            Widget::List(pod) => pod.select_first(),
            Widget::Text(log) => log.select_first(),
        }
    }

    fn select_last(&mut self) {
        match self {
            Widget::List(pod) => pod.select_last(),
            Widget::Text(log) => log.select_last(),
        }
    }

    fn set_items(&mut self, items: Vec<String>) {
        match self {
            Widget::List(pod) => pod.set_items(items),
            Widget::Text(log) => log.set_items(items),
        }
    }
}
