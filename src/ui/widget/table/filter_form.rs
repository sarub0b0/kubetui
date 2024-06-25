use ratatui::{
    crossterm::event::KeyEvent,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Paragraph},
    Frame,
};

use crate::ui::{
    event::EventResult,
    widget::{config::WidgetConfig, input::InputForm, RenderTrait},
};

#[derive(Debug, Default)]
struct Chunk {
    block: Rect,
    header: Rect,
}

#[derive(Debug)]
pub struct FilterForm {
    widget_config: WidgetConfig,
    input_widget: InputForm,
    chunk: Chunk,
    layout: Layout,
}

impl Default for FilterForm {
    fn default() -> Self {
        Self {
            widget_config: WidgetConfig::default(),
            input_widget: InputForm::builder()
                .widget_config(WidgetConfig::builder().block(Block::default()).build())
                .build(),
            chunk: Chunk::default(),
            layout: Layout::default()
                .direction(Direction::Horizontal)
                //            "FILTER: " len is 8.
                .constraints([Constraint::Length(8), Constraint::Percentage(100)]),
        }
    }
}

pub const FILTER_HEIGHT: u16 = 3;

impl FilterForm {
    pub fn update_chunk(&mut self, chunk: Rect) {
        let block_chunk = Rect::new(chunk.x, chunk.y, chunk.width, FILTER_HEIGHT);

        let inner_chunk = self.widget_config.block().inner(block_chunk);

        let chunks = self.layout.split(inner_chunk);

        let header_chunk = chunks[0];

        self.chunk = Chunk {
            block: block_chunk,
            header: header_chunk,
        };

        let content_chunk = chunks[1];

        self.input_widget.update_chunk(content_chunk);
    }

    pub fn word(&self) -> String {
        self.input_widget.content()
    }

    pub fn on_key_event(&mut self, ev: KeyEvent) -> EventResult {
        self.input_widget.on_key_event(ev)
    }

    pub fn clear(&mut self) {
        self.input_widget.clear();
    }

    pub fn render(&mut self, f: &mut Frame<'_>, is_active: bool) {
        f.render_widget(
            self.widget_config.render_block(is_active, false),
            self.chunk.block,
        );

        f.render_widget(Paragraph::new("FILTER: "), self.chunk.header);

        self.input_widget.render(f, is_active, false);
    }
}
