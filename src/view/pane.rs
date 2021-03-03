use crate::widget::*;

use tui::layout::{Constraint, Direction, Layout, Rect};

pub struct Pane<'a> {
    widget: Widget<'a>,
    chunk_index: usize,
    title: String,
    id: String,
    chunk: Rect,
}

impl<'a> Pane<'a> {
    pub fn new(
        title: impl Into<String>,
        widget: Widget<'a>,
        chunk_index: usize,
        id: impl Into<String>,
    ) -> Self {
        Self {
            title: title.into(),
            widget,
            chunk_index,
            id: id.into(),
            chunk: Rect::default(),
        }
    }

    pub fn widget(&self) -> &Widget {
        &self.widget
    }

    pub fn widget_mut(&mut self) -> &mut Widget<'a> {
        &mut self.widget
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn chunk_index(&self) -> usize {
        self.chunk_index
    }

    pub fn next_item(&mut self, index: usize) {
        self.widget.select_next(index)
    }

    pub fn prev_item(&mut self, index: usize) {
        self.widget.select_prev(index)
    }

    pub fn set_items(&mut self, items: Vec<String>) {
        self.widget.set_items(items);
    }

    pub fn is_selected(&self, rhs: &Pane) -> bool {
        return std::ptr::eq(self, rhs);
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn update_chunk(&mut self, chunk: Rect) {
        self.chunk = chunk;
    }

    pub fn chunk(&self) -> Rect {
        self.chunk
    }
}
