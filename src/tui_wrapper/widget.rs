mod ansi_color;
mod spans;
mod wrap;

pub mod complex;
pub mod config;
pub mod list;
pub mod table;
pub mod text;

use std::collections::BTreeMap;

pub use complex::*;
pub use list::*;
pub use table::*;
pub use text::*;

use crossterm::event::{KeyEvent, MouseEvent};
use tui::{backend::Backend, layout::Rect, Frame};

use self::config::WidgetConfig;

use super::event::EventResult;

use enum_dispatch::enum_dispatch;

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AtomLiteralItem {
    pub metadata: Option<BTreeMap<String, String>>,
    pub item: String,
}

impl AtomLiteralItem {
    pub fn new(item: impl Into<String>, metadata: Option<BTreeMap<String, String>>) -> Self {
        Self {
            item: item.into(),
            metadata,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct AtomTableItem {
    pub metadata: Option<BTreeMap<String, String>>,
    pub item: Vec<String>,
}

impl AtomTableItem {
    pub fn new(item: impl Into<Vec<String>>, metadata: Option<BTreeMap<String, String>>) -> Self {
        Self {
            item: item.into(),
            metadata,
        }
    }
}

impl From<String> for AtomLiteralItem {
    fn from(item: String) -> Self {
        Self::new(item, None)
    }
}

impl From<Vec<String>> for AtomTableItem {
    fn from(item: Vec<String>) -> Self {
        Self::new(item, None)
    }
}

#[derive(Debug, Clone)]
pub enum Item {
    Single(AtomLiteralItem),
    Array(Vec<AtomLiteralItem>),
    Table(Vec<AtomTableItem>),
}

impl Item {
    pub fn single(self) -> AtomLiteralItem {
        if let Self::Single(v) = self {
            v
        } else {
            panic!("called single() on {:?}", self)
        }
    }

    pub fn array(self) -> Vec<AtomLiteralItem> {
        if let Self::Array(v) = self {
            v
        } else {
            panic!("called array() on {:?}", self)
        }
    }

    pub fn table(self) -> Vec<AtomTableItem> {
        if let Self::Table(v) = self {
            v
        } else {
            panic!("called double_array() on {:?}", self)
        }
    }

    pub fn as_array(&self) -> &[AtomLiteralItem] {
        if let Self::Array(v) = self {
            v
        } else {
            panic!("called as_array() on {:?}", self)
        }
    }
}

#[derive(Debug, Clone)]
pub enum SelectedItem {
    Literal {
        metadata: Option<BTreeMap<String, String>>,
        item: String,
    },
    TableRow {
        metadata: Option<BTreeMap<String, String>>,
        item: Vec<String>,
    },
    Array(Vec<AtomLiteralItem>),
}

impl From<AtomLiteralItem> for SelectedItem {
    fn from(item: AtomLiteralItem) -> Self {
        Self::Literal {
            metadata: item.metadata,
            item: item.item,
        }
    }
}

impl From<AtomTableItem> for SelectedItem {
    fn from(item: AtomTableItem) -> Self {
        Self::TableRow {
            metadata: item.metadata,
            item: item.item,
        }
    }
}

impl From<Vec<AtomLiteralItem>> for SelectedItem {
    fn from(item: Vec<AtomLiteralItem>) -> Self {
        Self::Array(item)
    }
}

#[enum_dispatch]
pub trait WidgetTrait {
    // Getter
    fn id(&self) -> &str;
    fn widget_config(&self) -> &WidgetConfig;
    fn widget_config_mut(&mut self) -> &mut WidgetConfig;
    fn focusable(&self) -> bool;
    /// selected item
    fn widget_item(&self) -> Option<SelectedItem>;
    fn chunk(&self) -> Rect;

    // Setter
    fn select_index(&mut self, _: usize);
    fn select_next(&mut self, _: usize);
    fn select_prev(&mut self, _: usize);
    fn select_first(&mut self);
    fn select_last(&mut self);
    // Modify Widget Item
    fn append_widget_item(&mut self, _: Item);
    fn update_widget_item(&mut self, _: Item);
    // Widget append title
    // Render widget title -> format!("{}: {}", title, append_title)
    // When clear, append_title clear.

    fn on_mouse_event(&mut self, _: MouseEvent) -> EventResult;
    fn on_key_event(&mut self, _: KeyEvent) -> EventResult;

    fn update_chunk(&mut self, _: Rect);
    // コンテンツの初期化
    fn clear(&mut self);
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
