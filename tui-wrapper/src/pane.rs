use crossterm::event::MouseEvent;
use tui::{backend::Backend, layout::Rect, widgets::Block, Frame};

use super::focus_block;
use super::widget::*;

#[derive(Debug, Clone)]
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

    pub fn select_next_item(&mut self, index: usize) {
        self.widget.select_next(index)
    }

    pub fn select_prev_item(&mut self, index: usize) {
        self.widget.select_prev(index)
    }

    pub fn select_first_item(&mut self) {
        self.widget.select_first()
    }

    pub fn select_last_item(&mut self) {
        self.widget.select_last()
    }

    pub fn set_items(&mut self, items: WidgetItem) {
        self.widget.set_items(items);
    }

    pub fn append_items(&mut self, items: WidgetItem) {
        self.widget.append_items(items);
    }

    pub fn is_selected(&self, rhs: &Pane) -> bool {
        std::ptr::eq(self, rhs)
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn update_chunk(&mut self, chunk: Rect) {
        self.chunk = chunk;

        self.widget.update_chunk(self.block(false).inner(chunk));
    }

    pub fn chunk(&self) -> Rect {
        self.chunk
    }

    pub fn block(&self, selected: bool) -> Block {
        focus_block(&self.title, selected)
    }

    pub fn clear(&mut self) {
        self.widget.clear()
    }

    pub fn on_mouse_event(&mut self, ev: MouseEvent) {
        self.widget.on_mouse_event(ev);
    }
}

impl Pane<'_> {
    pub fn render<B>(&mut self, f: &mut Frame<B>, selected: bool)
    where
        B: Backend,
    {
        self.widget
            .render(f, focus_block(&self.title, selected), self.chunk);
    }
}
