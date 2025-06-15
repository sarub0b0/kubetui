pub mod ansi_color;
mod clear;
mod line;
mod styled_graphemes;
mod wrap;

mod base;
mod check_list;
mod input;
mod list;
pub mod multiple_select;
pub mod single_select;
pub mod table;
mod text;

pub use base::*;
pub use check_list::*;
pub use clear::*;
pub use input::*;
pub use list::*;
pub use multiple_select::MultipleSelect;
pub use single_select::SingleSelect;
pub use table::*;
pub use text::*;

use std::{collections::BTreeMap, hash::Hash};

use enum_dispatch::enum_dispatch;
use ratatui::{
    crossterm::event::{KeyEvent, MouseEvent},
    layout::Rect,
    Frame,
};

use super::event::EventResult;

use self::styled_graphemes::StyledGraphemes;

#[derive(Debug, Clone, Default, Hash, PartialEq, Eq)]
pub struct LiteralItem {
    pub metadata: Option<BTreeMap<String, String>>,
    pub item: String,
}

impl LiteralItem {
    pub fn new(item: impl Into<String>, metadata: Option<BTreeMap<String, String>>) -> Self {
        Self {
            item: item.into(),
            metadata,
        }
    }
}

impl Ord for LiteralItem {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let lhs = self.item.styled_graphemes_symbols().concat();
        let rhs = other.item.styled_graphemes_symbols().concat();
        lhs.cmp(&rhs)
    }
}

impl PartialOrd for LiteralItem {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct TableItem {
    pub metadata: Option<BTreeMap<String, String>>,
    pub item: Vec<String>,
}

impl TableItem {
    pub fn new(item: impl Into<Vec<String>>, metadata: Option<BTreeMap<String, String>>) -> Self {
        Self {
            item: item.into(),
            metadata,
        }
    }
}

impl<T> From<T> for LiteralItem
where
    T: ToString,
{
    fn from(item: T) -> Self {
        Self::new(item.to_string(), None)
    }
}

impl From<Vec<String>> for TableItem {
    fn from(item: Vec<String>) -> Self {
        Self::new(item, None)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct CheckListItem {
    pub label: String,
    pub checked: bool,
    pub required: bool,
    pub metadata: Option<BTreeMap<String, String>>,
}

#[derive(Debug, Clone)]
pub enum Item {
    Single(LiteralItem),
    Array(Vec<LiteralItem>),
    Table(Vec<TableItem>),
}

impl Item {
    pub fn single(self) -> LiteralItem {
        if let Self::Single(v) = self {
            v
        } else {
            panic!("called single() on {:?}", self)
        }
    }

    pub fn array(self) -> Vec<LiteralItem> {
        if let Self::Array(v) = self {
            v
        } else {
            panic!("called array() on {:?}", self)
        }
    }

    pub fn table(self) -> Vec<TableItem> {
        if let Self::Table(v) = self {
            v
        } else {
            panic!("called double_array() on {:?}", self)
        }
    }

    pub fn as_array(&self) -> &[LiteralItem] {
        if let Self::Array(v) = self {
            v
        } else {
            panic!("called as_array() on {:?}", self)
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum SelectedItem {
    Literal {
        metadata: Option<BTreeMap<String, String>>,
        item: String,
    },
    TableRow {
        metadata: Option<BTreeMap<String, String>>,
        item: Vec<String>,
    },
    Array(Vec<LiteralItem>),
    CheckListItem {
        label: String,
        checked: bool,
        required: bool,
        metadata: Option<BTreeMap<String, String>>,
    },
}

impl From<LiteralItem> for SelectedItem {
    fn from(item: LiteralItem) -> Self {
        Self::Literal {
            metadata: item.metadata,
            item: item.item,
        }
    }
}

impl From<TableItem> for SelectedItem {
    fn from(item: TableItem) -> Self {
        Self::TableRow {
            metadata: item.metadata,
            item: item.item,
        }
    }
}

impl From<Vec<LiteralItem>> for SelectedItem {
    fn from(item: Vec<LiteralItem>) -> Self {
        Self::Array(item)
    }
}

impl From<CheckListItem> for SelectedItem {
    fn from(item: CheckListItem) -> Self {
        Self::CheckListItem {
            label: item.label,
            checked: item.checked,
            required: item.required,
            metadata: item.metadata,
        }
    }
}

#[enum_dispatch]
pub trait WidgetTrait {
    // Getter
    fn id(&self) -> &str;
    fn widget_base(&self) -> &WidgetBase;
    fn widget_base_mut(&mut self) -> &mut WidgetBase;
    fn can_activate(&self) -> bool;
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
    fn render(&mut self, f: &mut Frame, is_active: bool, is_mouse_over: bool);
}

#[allow(clippy::large_enum_variant)]
#[enum_dispatch(WidgetTrait, RenderTrait)]
#[derive(Debug)]
pub enum Widget<'a> {
    List(List<'a>),
    Text(Text),
    Table(Table<'a>),
    SingleSelect(SingleSelect<'a>),
    MultipleSelect(MultipleSelect<'a>),
    Input(InputForm),
    CheckList(CheckList),
}

#[allow(dead_code)]
impl<'a> Widget<'a> {
    // as_*
    pub fn as_list(&self) -> &List<'a> {
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

    pub fn as_table(&self) -> &Table<'a> {
        if let Self::Table(w) = self {
            w
        } else {
            panic!("called as_table() on {:?}", self)
        }
    }

    pub fn as_single_select(&self) -> &SingleSelect<'a> {
        if let Self::SingleSelect(w) = self {
            w
        } else {
            panic!("called as_single_select() on {:?}", self)
        }
    }

    pub fn as_multiple_select(&self) -> &MultipleSelect<'a> {
        if let Self::MultipleSelect(w) = self {
            w
        } else {
            panic!("called as_multiple_select() on {:?}", self)
        }
    }

    pub fn as_check_list(&self) -> &CheckList {
        if let Self::CheckList(w) = self {
            w
        } else {
            panic!("called as_check_list() on {:?}", self)
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

    pub fn as_mut_text(&mut self) -> &mut Text {
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

    pub fn as_mut_check_list(&mut self) -> &mut CheckList {
        if let Self::CheckList(w) = self {
            w
        } else {
            panic!("called as_mut_check_list() on {:?}", self)
        }
    }
}
