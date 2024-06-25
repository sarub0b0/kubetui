use ratatui::{
    crossterm::event::KeyEvent,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Paragraph},
    Frame,
};

use crate::ui::{
    event::EventResult,
    widget::{config::WidgetConfig, InputForm, RenderTrait},
};

const PREFIX: &str = "Search: ";
const PREFIX_LEN: u16 = 8;

#[derive(Debug)]
pub struct SearchForm {
    input_widget: InputForm,
    header_chunk: Rect,
    remaining_chunk: Rect,
}

impl Default for SearchForm {
    fn default() -> Self {
        Self {
            input_widget: InputForm::builder()
                .widget_config(WidgetConfig::builder().block(Block::default()).build())
                .build(),
            header_chunk: Default::default(),
            remaining_chunk: Default::default(),
        }
    }
}

impl SearchForm {
    pub fn update_chunk(&mut self, chunk: Rect) {
        let Rect {
            x,
            y,
            width,
            height,
        } = chunk;

        let y = y + height.saturating_sub(1);

        self.header_chunk = Rect::new(x, y, PREFIX_LEN, 1);

        self.remaining_chunk = Rect::new(x + PREFIX_LEN, y, width.saturating_sub(PREFIX_LEN), 1);
    }

    pub fn word(&self) -> String {
        self.input_widget.content()
    }

    pub fn on_key_event(&mut self, ev: KeyEvent) -> EventResult {
        self.input_widget.on_key_event(ev)
    }

    pub fn render(&mut self, f: &mut Frame<'_>, is_active: bool, status: (usize, usize)) {
        let status = format!(" [{}/{}]", status.0, status.1);

        let content_width = self
            .remaining_chunk
            .width
            .saturating_sub(status.len() as u16);

        let chunks = Self::layout(content_width, status.len() as u16).split(self.remaining_chunk);

        f.render_widget(Paragraph::new(PREFIX), self.header_chunk);

        self.input_widget.update_chunk(chunks[0]);

        self.input_widget.render(f, is_active, false);

        f.render_widget(Paragraph::new(status), chunks[1]);
    }

    fn layout(content_width: u16, status_width: u16) -> Layout {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(content_width),
                Constraint::Length(status_width),
            ])
    }
}
