use super::generate_title;
use crate::widget::*;

use std::default::Default;

use tui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders},
};

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
        let h = 40;
        let w = 60;
        let chunk = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage((100 - h) / 2),
                Constraint::Percentage(h),
                Constraint::Percentage((100 - h) / 2),
            ])
            .split(chunk);

        self.chunk = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - w) / 2),
                Constraint::Percentage(w),
                Constraint::Percentage((100 - w) / 2),
            ])
            .split(chunk[1])[1];
    }

    pub fn block(&self) -> Block {
        Block::default()
            .title(generate_title(&self.title, Color::White, true))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::White))
    }
}
