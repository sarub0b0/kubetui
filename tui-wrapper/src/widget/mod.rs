mod ansi_color;
mod spans;
mod wrap;

pub mod list;
pub mod table;
pub mod text;

pub use list::*;
pub use table::*;
pub use text::*;

use crossterm::event::{KeyEvent, MouseEvent};
use tui::{backend::Backend, layout::Rect, widgets::Block, Frame};

use crate::EventResult;

#[derive(Debug)]
pub enum WidgetItem {
    Single(String),
    Array(Vec<String>),
    DoubleArray(Vec<Vec<String>>),
}

impl WidgetItem {
    pub fn single(self) -> String {
        if let Self::Single(v) = self {
            v
        } else {
            panic!("called single() on {:?}", self)
        }
    }

    pub fn array(self) -> Vec<String> {
        if let Self::Array(v) = self {
            v
        } else {
            panic!("called array() on {:?}", self)
        }
    }

    pub fn double_array(self) -> Vec<Vec<String>> {
        if let Self::DoubleArray(v) = self {
            v
        } else {
            panic!("called double_array() on {:?}", self)
        }
    }

    pub fn as_array(&self) -> &[String] {
        if let Self::Array(v) = self {
            v
        } else {
            panic!("called as_array() on {:?}", self)
        }
    }
}

pub trait WidgetTrait {
    fn selectable(&self) -> bool {
        false
    }
    fn select_next(&mut self, _: usize) {}
    fn select_prev(&mut self, _: usize) {}
    fn select_first(&mut self) {}
    fn select_last(&mut self) {}
    fn set_items(&mut self, _: WidgetItem) {}
    fn append_items(&mut self, _: WidgetItem) {}
    fn get_item(&self) -> Option<WidgetItem> {
        None
    }
    fn update_chunk(&mut self, _: Rect) {}
    fn clear(&mut self) {}
    fn on_mouse_event(&mut self, _: MouseEvent) -> EventResult {
        EventResult::Ignore
    }
    fn on_key_event(&mut self, _: KeyEvent) -> EventResult {
        EventResult::Ignore
    }
}

#[derive(Debug)]
pub enum Widget<'a> {
    List(List<'a>),
    Text(Text<'a>),
    Table(Table<'a>),
}

impl Default for Widget<'_> {
    fn default() -> Self {
        Widget::Text(Text::new(Vec::new()))
    }
}

impl<'a> Widget<'a> {
    // as_*
    pub fn as_list(&self) -> &List {
        if let Self::List(w) = self {
            w
        } else {
            panic!("called as_list() on {:?}", self)
        }
    }

    pub fn as_text(&self) -> &Text {
        if let Self::Text(w) = self {
            w
        } else {
            panic!("called as_text() on {:?}", self)
        }
    }

    pub fn as_table(&self) -> &Table {
        if let Self::Table(w) = self {
            w
        } else {
            panic!("called as_table() on {:?}", self)
        }
    }

    // as_mut_*
    pub fn as_mut_list(&mut self) -> &mut List<'a> {
        if let Self::List(w) = self {
            w
        } else {
            panic!("called as_mut_list() on {:?}", self)
        }
    }

    pub fn as_mut_text(&mut self) -> &mut Text<'a> {
        if let Self::Text(w) = self {
            w
        } else {
            panic!("called as_mut_text() on {:?}", self)
        }
    }

    pub fn as_mut_table(&mut self) -> &mut Table<'a> {
        if let Self::Table(w) = self {
            w
        } else {
            panic!("called as_mut_table() on {:?}", self)
        }
    }
}

impl WidgetTrait for Widget<'_> {
    fn selectable(&self) -> bool {
        match self {
            Widget::List(w) => w.selectable(),
            Widget::Text(w) => w.selectable(),
            Widget::Table(w) => w.selectable(),
        }
    }

    fn select_next(&mut self, index: usize) {
        match self {
            Widget::List(w) => w.select_next(index),
            Widget::Text(w) => w.select_next(index),
            Widget::Table(w) => w.select_next(index),
        }
    }

    fn select_prev(&mut self, index: usize) {
        match self {
            Widget::List(w) => w.select_prev(index),
            Widget::Text(w) => w.select_prev(index),
            Widget::Table(w) => w.select_prev(index),
        }
    }

    fn select_first(&mut self) {
        match self {
            Widget::List(w) => w.select_first(),
            Widget::Text(w) => w.select_first(),
            Widget::Table(w) => w.select_first(),
        }
    }

    fn select_last(&mut self) {
        match self {
            Widget::List(w) => w.select_last(),
            Widget::Text(w) => w.select_last(),
            Widget::Table(w) => w.select_last(),
        }
    }

    fn set_items(&mut self, items: WidgetItem) {
        match self {
            Widget::List(w) => w.set_items(items),
            Widget::Text(w) => w.set_items(items),
            Widget::Table(w) => w.set_items(items),
        }
    }

    fn update_chunk(&mut self, area: Rect) {
        match self {
            Widget::List(w) => w.update_chunk(area),
            Widget::Text(w) => w.update_chunk(area),
            Widget::Table(w) => w.update_chunk(area),
        }
    }

    fn clear(&mut self) {
        match self {
            Widget::List(w) => w.clear(),
            Widget::Text(w) => w.clear(),
            Widget::Table(w) => w.clear(),
        }
    }

    fn get_item(&self) -> Option<WidgetItem> {
        match self {
            Widget::List(w) => w.get_item(),
            Widget::Text(w) => w.get_item(),
            Widget::Table(w) => w.get_item(),
        }
    }

    fn append_items(&mut self, items: WidgetItem) {
        match self {
            Widget::List(w) => w.append_items(items),
            Widget::Text(w) => w.append_items(items),
            Widget::Table(w) => w.append_items(items),
        }
    }

    fn on_mouse_event(&mut self, ev: MouseEvent) -> EventResult {
        match self {
            Widget::List(w) => w.on_mouse_event(ev),
            Widget::Text(w) => w.on_mouse_event(ev),
            Widget::Table(w) => w.on_mouse_event(ev),
        }
    }

    fn on_key_event(&mut self, ev: KeyEvent) -> EventResult {
        match self {
            Widget::List(w) => w.on_key_event(ev),
            Widget::Text(w) => w.on_key_event(ev),
            Widget::Table(w) => w.on_key_event(ev),
        }
    }
}

pub trait RenderTrait {
    fn render<B: Backend>(&mut self, f: &mut Frame<B>, block: Block, chunk: Rect);
}

impl RenderTrait for Widget<'_> {
    fn render<B>(&mut self, f: &mut Frame<B>, block: Block, chunk: Rect)
    where
        B: Backend,
    {
        match self {
            Widget::List(w) => w.render(f, block, chunk),
            Widget::Text(w) => w.render(f, block, chunk),
            Widget::Table(w) => w.render(f, block, chunk),
        }
    }
}
