use super::Type;
use crate::widget::*;

use tui::layout::{Constraint, Direction, Layout, Rect};

pub struct Pane<'a> {
    widget: Widgets<'a>,
    chunk_index: usize,
    title: String,
    ty: Type,
    chunk: Rect,
}

impl<'a> Pane<'a> {
    pub fn new(
        title: impl Into<String>,
        widget: Widgets<'a>,
        chunk_index: usize,
        ty: Type,
    ) -> Self {
        Self {
            title: title.into(),
            widget,
            chunk_index,
            ty,
            chunk: Rect::default(),
        }
    }

    pub fn widget(&self) -> &Widgets {
        &self.widget
    }

    pub fn widget_mut(&mut self) -> &mut Widgets<'a> {
        &mut self.widget
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn chunk_index(&self) -> usize {
        self.chunk_index
    }

    pub fn next_item(&mut self) {
        self.widget.next()
    }

    pub fn prev_item(&mut self) {
        self.widget.prev()
    }

    pub fn is_selected(&self, rhs: &Pane) -> bool {
        return std::ptr::eq(self, rhs);
    }

    pub fn ty(&self) -> Type {
        self.ty
    }

    pub fn update_chunk(&mut self, chunk: Rect) {
        self.chunk = chunk;
    }

    pub fn chunk(&self) -> Rect {
        self.chunk
    }
}
