use crossterm::event::KeyEvent;
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::tui_wrapper::{
    event::EventResult,
    widget::{config::WidgetConfig, InputForm},
};

#[derive(Debug)]
pub struct FilterForm {
    input_widget: InputForm,
    chunk: Rect,
}

impl Default for FilterForm {
    fn default() -> Self {
        Self {
            input_widget: InputForm::new(WidgetConfig::default()),
            chunk: Default::default(),
        }
    }
}

impl FilterForm {
    pub fn update_chunk(&mut self, chunk: Rect) {
        self.chunk = Rect::new(chunk.x, chunk.y, chunk.width, 3);
    }

    pub fn word(&self) -> String {
        self.input_widget.content()
    }

    pub fn on_key_event(&mut self, ev: KeyEvent) -> EventResult {
        self.input_widget.on_key_event(ev)
    }

    pub fn render<B>(&mut self, f: &mut Frame<'_, B>, selected: bool)
    where
        B: Backend,
    {
        let header = "FILTER: ";

        let content = self.input_widget.render_content(selected);

        let content_width = self.chunk.width.saturating_sub(8);

        let block = Block::default()
            .border_type(BorderType::Plain)
            .borders(Borders::ALL);

        let inner_chunk = block.inner(self.chunk);

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(8), Constraint::Length(content_width)])
            .split(inner_chunk);

        f.render_widget(block, self.chunk);

        f.render_widget(Paragraph::new(header), chunks[0]);

        f.render_widget(Paragraph::new(content), chunks[1]);
    }
}
