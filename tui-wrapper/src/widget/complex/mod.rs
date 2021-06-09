use crossterm::event::{KeyEvent, MouseEvent};
use tui::{backend::Backend, layout::Rect, Frame};

mod input;
mod multiple_select;
mod single_select;

pub use multiple_select::{MultipleSelect, MultipleSelectBuilder};
pub use single_select::{SingleSelect, SingleSelectBuilder};

use crate::event::EventResult;

use super::{RenderTrait, WidgetItem, WidgetTrait};

#[derive(Debug)]
pub enum ComplexWidget<'a> {
    SingleSelect(Box<SingleSelect<'a>>),
    MultipleSelect(Box<MultipleSelect<'a>>),
}

impl<'a> From<SingleSelect<'a>> for ComplexWidget<'a> {
    fn from(w: SingleSelect<'a>) -> Self {
        Self::SingleSelect(Box::new(w))
    }
}

impl<'a> From<MultipleSelect<'a>> for ComplexWidget<'a> {
    fn from(w: MultipleSelect<'a>) -> Self {
        Self::MultipleSelect(Box::new(w))
    }
}

impl WidgetTrait for ComplexWidget<'_> {
    fn focusable(&self) -> bool {
        match self {
            Self::SingleSelect(w) => w.focusable(),
            Self::MultipleSelect(w) => w.focusable(),
        }
    }

    fn select_next(&mut self, i: usize) {
        match self {
            Self::SingleSelect(w) => w.select_next(i),
            Self::MultipleSelect(w) => w.select_next(i),
        }
    }

    fn select_prev(&mut self, i: usize) {
        match self {
            Self::SingleSelect(w) => w.select_prev(i),
            Self::MultipleSelect(w) => w.select_prev(i),
        }
    }

    fn select_first(&mut self) {
        match self {
            Self::SingleSelect(w) => w.select_first(),
            Self::MultipleSelect(w) => w.select_first(),
        }
    }

    fn select_last(&mut self) {
        match self {
            Self::SingleSelect(w) => w.select_last(),
            Self::MultipleSelect(w) => w.select_last(),
        }
    }

    fn update_widget_item(&mut self, item: WidgetItem) {
        match self {
            Self::SingleSelect(w) => w.update_widget_item(item),
            Self::MultipleSelect(w) => w.update_widget_item(item),
        }
    }

    fn append_widget_item(&mut self, item: WidgetItem) {
        match self {
            Self::SingleSelect(w) => w.update_widget_item(item),
            Self::MultipleSelect(w) => w.update_widget_item(item),
        }
    }

    fn widget_item(&self) -> Option<WidgetItem> {
        match self {
            Self::SingleSelect(w) => w.widget_item(),
            Self::MultipleSelect(w) => w.widget_item(),
        }
    }

    fn update_chunk(&mut self, chunk: Rect) {
        match self {
            Self::SingleSelect(w) => w.update_chunk(chunk),
            Self::MultipleSelect(w) => w.update_chunk(chunk),
        }
    }

    fn clear(&mut self) {
        match self {
            Self::SingleSelect(w) => w.clear(),
            Self::MultipleSelect(w) => w.clear(),
        }
    }

    fn on_mouse_event(&mut self, ev: MouseEvent) -> EventResult {
        match self {
            Self::SingleSelect(w) => w.on_mouse_event(ev),
            Self::MultipleSelect(w) => w.on_mouse_event(ev),
        }
    }

    fn on_key_event(&mut self, ev: KeyEvent) -> EventResult {
        match self {
            Self::SingleSelect(w) => w.on_key_event(ev),
            Self::MultipleSelect(w) => w.on_key_event(ev),
        }
    }

    fn id(&self) -> &str {
        match self {
            Self::SingleSelect(w) => w.id(),
            Self::MultipleSelect(w) => w.id(),
        }
    }

    fn title(&self) -> &str {
        match self {
            Self::SingleSelect(w) => w.title(),
            Self::MultipleSelect(w) => w.title(),
        }
    }

    fn chunk(&self) -> tui::layout::Rect {
        match self {
            Self::SingleSelect(w) => w.chunk(),
            Self::MultipleSelect(w) => w.chunk(),
        }
    }
}

impl RenderTrait for ComplexWidget<'_> {
    fn render<B>(&mut self, f: &mut Frame<'_, B>, selected: bool)
    where
        B: Backend,
    {
        match self {
            Self::SingleSelect(w) => w.render(f, selected),
            Self::MultipleSelect(w) => w.render(f, selected),
        }
    }
}
