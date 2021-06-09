mod ansi_color;
mod spans;
mod wrap;

pub mod complex;
pub mod list;
pub mod table;
pub mod text;

pub use complex::*;
pub use list::*;
pub use table::*;
pub use text::*;

use crossterm::event::{KeyEvent, MouseEvent};
use tui::{backend::Backend, layout::Rect, Frame};

use super::event::EventResult;

#[derive(Debug, Clone)]
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
    fn id(&self) -> &str;
    fn title(&self) -> &str;

    fn focusable(&self) -> bool {
        false
    }
    fn chunk(&self) -> Rect;
    fn select_next(&mut self, _: usize) {}
    fn select_prev(&mut self, _: usize) {}
    fn select_first(&mut self) {}
    fn select_last(&mut self) {}
    fn append_widget_item(&mut self, _: WidgetItem) {}
    fn update_widget_item(&mut self, _: WidgetItem) {}
    fn widget_item(&self) -> Option<WidgetItem> {
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
    List(Box<List<'a>>),
    Text(Box<Text<'a>>),
    Table(Box<Table<'a>>),
    Complex(Box<ComplexWidget<'a>>),
}

impl<'a> From<List<'a>> for Widget<'a> {
    fn from(w: List<'a>) -> Self {
        Self::List(Box::new(w))
    }
}

impl<'a> From<Text<'a>> for Widget<'a> {
    fn from(w: Text<'a>) -> Self {
        Self::Text(Box::new(w))
    }
}

impl<'a> From<Table<'a>> for Widget<'a> {
    fn from(w: Table<'a>) -> Self {
        Self::Table(Box::new(w))
    }
}

impl<'a> From<ComplexWidget<'a>> for Widget<'a> {
    fn from(w: ComplexWidget<'a>) -> Self {
        Self::Complex(Box::new(w))
    }
}

impl<'a> From<SingleSelect<'a>> for Widget<'a> {
    fn from(w: SingleSelect<'a>) -> Self {
        Self::Complex(Box::new(ComplexWidget::from(w)))
    }
}

impl<'a> From<MultipleSelect<'a>> for Widget<'a> {
    fn from(w: MultipleSelect<'a>) -> Self {
        Self::Complex(Box::new(ComplexWidget::from(w)))
    }
}

impl Default for Widget<'_> {
    fn default() -> Self {
        Widget::Text(Box::new(Text::default()))
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

    pub fn as_complex(&self) -> &ComplexWidget {
        if let Self::Complex(w) = self {
            w
        } else {
            panic!("called as_complex() on {:?}", self)
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

    pub fn as_mut_complex(&mut self) -> &mut ComplexWidget<'a> {
        if let Self::Complex(w) = self {
            w
        } else {
            panic!("called as_mut_complex() on {:?}", self)
        }
    }
}

impl WidgetTrait for Widget<'_> {
    fn focusable(&self) -> bool {
        match self {
            Widget::List(w) => w.focusable(),
            Widget::Text(w) => w.focusable(),
            Widget::Table(w) => w.focusable(),
            Widget::Complex(w) => w.focusable(),
        }
    }

    fn select_next(&mut self, index: usize) {
        match self {
            Widget::List(w) => w.select_next(index),
            Widget::Text(w) => w.select_next(index),
            Widget::Table(w) => w.select_next(index),
            Widget::Complex(w) => w.select_next(index),
        }
    }

    fn select_prev(&mut self, index: usize) {
        match self {
            Widget::List(w) => w.select_prev(index),
            Widget::Text(w) => w.select_prev(index),
            Widget::Table(w) => w.select_prev(index),
            Widget::Complex(w) => w.select_prev(index),
        }
    }

    fn select_first(&mut self) {
        match self {
            Widget::List(w) => w.select_first(),
            Widget::Text(w) => w.select_first(),
            Widget::Table(w) => w.select_first(),
            Widget::Complex(w) => w.select_first(),
        }
    }

    fn select_last(&mut self) {
        match self {
            Widget::List(w) => w.select_last(),
            Widget::Text(w) => w.select_last(),
            Widget::Table(w) => w.select_last(),
            Widget::Complex(w) => w.select_last(),
        }
    }

    fn update_widget_item(&mut self, items: WidgetItem) {
        match self {
            Widget::List(w) => w.update_widget_item(items),
            Widget::Text(w) => w.update_widget_item(items),
            Widget::Table(w) => w.update_widget_item(items),
            Widget::Complex(w) => w.update_widget_item(items),
        }
    }

    fn update_chunk(&mut self, area: Rect) {
        match self {
            Widget::List(w) => w.update_chunk(area),
            Widget::Text(w) => w.update_chunk(area),
            Widget::Table(w) => w.update_chunk(area),
            Widget::Complex(w) => w.update_chunk(area),
        }
    }

    fn clear(&mut self) {
        match self {
            Widget::List(w) => w.clear(),
            Widget::Text(w) => w.clear(),
            Widget::Table(w) => w.clear(),
            Widget::Complex(w) => w.clear(),
        }
    }

    fn widget_item(&self) -> Option<WidgetItem> {
        match self {
            Widget::List(w) => w.widget_item(),
            Widget::Text(w) => w.widget_item(),
            Widget::Table(w) => w.widget_item(),
            Widget::Complex(w) => w.widget_item(),
        }
    }

    fn append_widget_item(&mut self, items: WidgetItem) {
        match self {
            Widget::List(w) => w.append_widget_item(items),
            Widget::Text(w) => w.append_widget_item(items),
            Widget::Table(w) => w.append_widget_item(items),
            Widget::Complex(w) => w.append_widget_item(items),
        }
    }

    fn on_mouse_event(&mut self, ev: MouseEvent) -> EventResult {
        match self {
            Widget::List(w) => w.on_mouse_event(ev),
            Widget::Text(w) => w.on_mouse_event(ev),
            Widget::Table(w) => w.on_mouse_event(ev),
            Widget::Complex(w) => w.on_mouse_event(ev),
        }
    }

    fn on_key_event(&mut self, ev: KeyEvent) -> EventResult {
        match self {
            Widget::List(w) => w.on_key_event(ev),
            Widget::Text(w) => w.on_key_event(ev),
            Widget::Table(w) => w.on_key_event(ev),
            Widget::Complex(w) => w.on_key_event(ev),
        }
    }

    fn title(&self) -> &str {
        match self {
            Widget::List(w) => w.title(),
            Widget::Text(w) => w.title(),
            Widget::Table(w) => w.title(),
            Widget::Complex(w) => w.title(),
        }
    }

    fn chunk(&self) -> Rect {
        match self {
            Widget::List(w) => w.chunk(),
            Widget::Text(w) => w.chunk(),
            Widget::Table(w) => w.chunk(),
            Widget::Complex(w) => w.chunk(),
        }
    }

    fn id(&self) -> &str {
        match self {
            Widget::List(w) => w.id(),
            Widget::Text(w) => w.id(),
            Widget::Table(w) => w.id(),
            Widget::Complex(w) => w.id(),
        }
    }
}

pub trait RenderTrait {
    fn render<B: Backend>(&mut self, f: &mut Frame<B>, focused: bool);
}

impl RenderTrait for Widget<'_> {
    fn render<B>(&mut self, f: &mut Frame<B>, focused: bool)
    where
        B: Backend,
    {
        match self {
            Widget::List(w) => w.render(f, focused),
            Widget::Text(w) => w.render(f, focused),
            Widget::Table(w) => w.render(f, focused),
            Widget::Complex(w) => w.render(f, focused),
        }
    }
}
