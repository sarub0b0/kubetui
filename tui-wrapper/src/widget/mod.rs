mod ansi;
pub mod list;
pub mod table;
pub mod text;

pub use self::list::*;
pub use self::text::*;
pub use table::*;

use tui::{backend::Backend, layout::Rect, widgets::Block, Frame};

pub enum WidgetItem {
    Array(Vec<String>),
    DoubleArray(Vec<Vec<String>>),
}

impl WidgetItem {
    fn get_array(self) -> Vec<String> {
        match self {
            WidgetItem::Array(item) => item,
            _ => Vec::new(),
        }
    }
    fn get_double_array(self) -> Vec<Vec<String>> {
        match self {
            WidgetItem::DoubleArray(item) => item,
            _ => Vec::new(),
        }
    }
}

pub trait WidgetTrait {
    fn selectable(&self) -> bool;
    fn select_next(&mut self, index: usize);
    fn select_prev(&mut self, index: usize);
    fn select_first(&mut self);
    fn select_last(&mut self);
    fn set_items(&mut self, items: WidgetItem);
    fn update_area(&mut self, area: Rect);
    fn clear(&mut self);
}

#[derive(Debug, Clone)]
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

    pub fn table(&self) -> Option<&Table> {
        match self {
            Widget::Table(table) => Some(table),
            _ => None,
        }
    }

    pub fn table_mut(&mut self) -> Option<&mut Table<'a>> {
        match self {
            Widget::Table(table) => Some(table),
            _ => None,
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

    fn update_area(&mut self, area: Rect) {
        match self {
            Widget::List(w) => w.update_area(area),
            Widget::Text(w) => w.update_area(area),
            Widget::Table(w) => w.update_area(area),
        }
    }

    fn clear(&mut self) {
        match self {
            Widget::List(w) => w.clear(),
            Widget::Text(w) => w.clear(),
            Widget::Table(w) => w.clear(),
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
