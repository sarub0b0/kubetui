use crossterm::event::{KeyEvent, MouseEvent};
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    widgets::Clear,
    Frame,
};

use super::{
    event::EventResult,
    widget::{RenderTrait, Widget, WidgetTrait},
};

/// ┌─────────────────────────────────────────────────┐
/// │ margin                 ▲                        │
/// │                        │ top                    │
/// │                        ▼                        │
/// │          ┌───────────────────────────┐          │
/// │          │ content        ▲          │          │
/// │          │                │          │          │
/// │   left   │                │ height   │  right   │
/// │◄────────►│                │          │◄────────►│
/// │          │◄───────────────┼─────────►│          │
/// │          │    width       │          │          │
/// │          │                ▼          │          │
/// │          └───────────────────────────┘          │
/// │                        ▲                        │
/// │                        │ bottom                 │
/// │                        ▼                        │
/// └─────────────────────────────────────────────────┘
#[derive(Debug)]
pub struct PopupChunkSize {
    pub margin_left: Constraint,
    pub margin_right: Constraint,
    pub margin_top: Constraint,
    pub margin_bottom: Constraint,
    pub content_width: Constraint,
    pub content_height: Constraint,
}

impl Default for PopupChunkSize {
    fn default() -> Self {
        Self {
            margin_left: Constraint::Percentage(10),
            margin_right: Constraint::Percentage(10),
            margin_top: Constraint::Percentage(10),
            margin_bottom: Constraint::Percentage(10),
            content_width: Constraint::Percentage(80),
            content_height: Constraint::Percentage(80),
        }
    }
}

impl PopupChunkSize {
    fn chunk(&self, parent_chunk: Rect) -> Rect {
        let chunk = Layout::default()
            .direction(Direction::Vertical)
            .constraints([self.margin_top, self.content_height, self.margin_bottom])
            .split(parent_chunk);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([self.margin_left, self.content_width, self.margin_right])
            .split(chunk[1])[1]
    }
}

pub struct Popup<'a> {
    pub widget: Widget<'a>,
    pub chunk: Rect,
    pub chunk_size: PopupChunkSize,
}

impl<'a> Popup<'a> {
    pub fn new(widget: Widget<'a>) -> Self {
        Self {
            widget,
            chunk: Default::default(),
            chunk_size: Default::default(),
        }
    }

    pub fn chunk_size(mut self, chunk_size: PopupChunkSize) -> Self {
        self.chunk_size = chunk_size;
        self
    }

    pub fn id(&self) -> &str {
        self.widget.id()
    }

    pub fn update_chunk(&mut self, parent_chunk: Rect) {
        let chunk = self.chunk_size.chunk(parent_chunk);

        self.chunk = chunk;

        self.widget.update_chunk(chunk.inner(&Margin {
            vertical: 1,
            horizontal: 1,
        }));
    }

    pub fn widget(&self) -> &Widget<'a> {
        &self.widget
    }

    pub fn widget_mut(&mut self) -> &mut Widget<'a> {
        &mut self.widget
    }

    pub fn render<B: Backend>(&mut self, f: &mut Frame<B>) {
        f.render_widget(Clear, self.chunk);

        self.widget.render(f, true, false)
    }

    pub fn on_key_event(&mut self, ev: KeyEvent) -> EventResult {
        self.widget.on_key_event(ev)
    }
    pub fn on_mouse_event(&mut self, ev: MouseEvent) -> EventResult {
        self.widget.on_mouse_event(ev)
    }
}
