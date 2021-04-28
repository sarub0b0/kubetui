use super::{child_window_chunk, focus_block};
use crate::widget::*;

use std::default::Default;

use tui::{layout::Rect, widgets::Block};

pub struct Popup<'a> {
    title: String,
    widget: Widget<'a>,
    chunk: Rect,
    id: String,
}

impl<'a> Popup<'a> {
    pub fn new(title: impl Into<String>, widget: Widget<'a>, id: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            widget,
            chunk: Rect::default(),
            id: id.into(),
        }
    }

    pub fn next_item(&mut self) {
        self.widget.select_next(1);
    }

    pub fn prev_item(&mut self) {
        self.widget.select_prev(1);
    }

    pub fn last_item(&mut self) {
        self.widget.select_last();
    }

    pub fn first_item(&mut self) {
        self.widget.select_first();
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

    pub fn chunk(&self) -> Rect {
        self.chunk
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn update_chunk(&mut self, chunk: Rect) {
        self.chunk = child_window_chunk(60, 40, chunk);
    }

    pub fn block(&self) -> Block {
        focus_block(&self.title, true)
    }
}
