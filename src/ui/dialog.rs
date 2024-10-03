use ratatui::{
    crossterm::event::{KeyEvent, MouseEvent},
    layout::{Margin, Rect},
    widgets::Clear,
    Frame,
};

use super::{
    event::EventResult,
    widget::{RenderTrait, Widget, WidgetTrait},
};

/// Dialogの大きさを決めるための構造体
///
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
struct DialogSize {
    /// content width percentage (0.0 ~ 100.0)
    width: f32,
    /// content height percentage (0.0 ~ 100.0)
    height: f32,
}

impl Default for DialogSize {
    fn default() -> Self {
        Self {
            width: 85.0,
            height: 85.0,
        }
    }
}

impl DialogSize {
    fn chunk(&self, parent_chunk: Rect) -> Rect {
        let horizontal_margin =
            (parent_chunk.width as f32 * ((100.0 - self.width) / 2.0 / 100.0)).round() as u16;
        let vertical_margin =
            (parent_chunk.height as f32 * ((100.0 - self.height) / 2.0 / 100.0)).round() as u16;

        parent_chunk.inner(Margin {
            vertical: vertical_margin,
            horizontal: horizontal_margin,
        })
    }
}

pub struct Dialog<'a> {
    widget: Widget<'a>,
    chunk: Rect,
    chunk_size: DialogSize,
}

impl<'a> Dialog<'a> {
    pub fn new(widget: Widget<'a>) -> Self {
        Self {
            widget,
            chunk: Default::default(),
            chunk_size: Default::default(),
        }
    }

    pub fn chunk(&self) -> Rect {
        self.chunk
    }

    pub fn id(&self) -> &str {
        self.widget.id()
    }

    pub fn update_chunk(&mut self, parent_chunk: Rect) {
        let chunk = self.chunk_size.chunk(parent_chunk);

        self.chunk = chunk;

        self.widget.update_chunk(chunk.inner(Margin {
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

    pub fn render(&mut self, f: &mut Frame) {
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
