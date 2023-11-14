use crossterm::event::KeyEvent;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Paragraph},
    Frame,
};
use unicode_width::UnicodeWidthStr;

use crate::ui::{
    event::EventResult,
    widget::{config::WidgetConfig, InputForm},
};

#[derive(Debug)]
pub struct SearchForm {
    input_widget: InputForm,
    chunk: Rect,
}

impl Default for SearchForm {
    fn default() -> Self {
        Self {
            input_widget: InputForm::new(WidgetConfig::builder().block(Block::default()).build()),
            chunk: Default::default(),
        }
    }
}

impl SearchForm {
    pub fn update_chunk(&mut self, chunk: Rect) {
        self.chunk = Rect::new(
            chunk.x,
            chunk.y + chunk.height.saturating_sub(1),
            chunk.width,
            1,
        );
    }

    pub fn word(&self) -> String {
        self.input_widget.content()
    }

    pub fn on_key_event(&mut self, ev: KeyEvent) -> EventResult {
        self.input_widget.on_key_event(ev)
    }

    pub fn render(&mut self, f: &mut Frame<'_>, is_active: bool, status: (usize, usize)) {
        let header = "Search: ";

        let content = self.input_widget.render_content(is_active);

        let status = format!(" [{}/{}]", status.0, status.1);

        let content_width = self.chunk.width.saturating_sub(8 + status.width() as u16);

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(8),
                Constraint::Length(content_width),
                Constraint::Length(status.len() as u16),
            ])
            .split(self.chunk);

        f.render_widget(Paragraph::new(header), chunks[0]);

        f.render_widget(Paragraph::new(content), chunks[1]);

        f.render_widget(Paragraph::new(status), chunks[2]);
    }
}
