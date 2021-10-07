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

use enum_dispatch::enum_dispatch;

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

#[enum_dispatch]
pub trait WidgetTrait {
    // Getter
    fn id(&self) -> &str;
    fn title(&self) -> &str;
    fn focusable(&self) -> bool;
    fn chunk(&self) -> Rect;
    fn widget_item(&self) -> Option<WidgetItem>;

    // Setter
    fn select_index(&mut self, _: usize);
    fn select_next(&mut self, _: usize);
    fn select_prev(&mut self, _: usize);
    fn select_first(&mut self);
    fn select_last(&mut self);
    fn append_widget_item(&mut self, _: WidgetItem);
    fn update_widget_item(&mut self, _: WidgetItem);
    fn update_chunk(&mut self, _: Rect);
    fn clear(&mut self);
    fn on_mouse_event(&mut self, _: MouseEvent) -> EventResult;
    fn on_key_event(&mut self, _: KeyEvent) -> EventResult;
    fn update_title(&mut self, _: impl Into<String>);
}

#[enum_dispatch]
pub trait RenderTrait {
    fn render<B: Backend>(&mut self, f: &mut Frame<B>, focused: bool);
}

#[enum_dispatch(WidgetTrait, RenderTrait)]
#[derive(Debug)]
pub enum Widget<'a> {
    List(List<'a>),
    Text(Text<'a>),
    Table(Table<'a>),
    SingleSelect(SingleSelect<'a>),
    MultipleSelect(MultipleSelect<'a>),
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

    pub fn as_single_select(&self) -> &SingleSelect {
        if let Self::SingleSelect(w) = self {
            w
        } else {
            panic!("called as_single_select() on {:?}", self)
        }
    }

    pub fn as_multiple_select(&self) -> &MultipleSelect {
        if let Self::MultipleSelect(w) = self {
            w
        } else {
            panic!("called as_multiple_select() on {:?}", self)
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

    pub fn as_mut_single_select(&mut self) -> &mut SingleSelect<'a> {
        if let Self::SingleSelect(w) = self {
            w
        } else {
            panic!("called as_mut_single_select() on {:?}", self)
        }
    }

    pub fn as_mut_multiple_select(&mut self) -> &mut MultipleSelect<'a> {
        if let Self::MultipleSelect(w) = self {
            w
        } else {
            panic!("called as_mut_multiple_select() on {:?}", self)
        }
    }
}
