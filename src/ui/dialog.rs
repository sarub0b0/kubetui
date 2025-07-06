use ratatui::{
    Frame,
    crossterm::event::{KeyEvent, MouseEvent},
    layout::{Margin, Rect},
    style::Style,
};

use super::{
    event::EventResult,
    widget::{RenderTrait, StyledClear, Text, Widget, WidgetTrait},
};

#[derive(Debug, Default, Clone)]
pub struct DialogTheme {
    pub base_style: Style,

    pub size: DialogSize,
}

impl DialogTheme {
    pub fn base_style(mut self, style: impl Into<Style>) -> Self {
        self.base_style = style.into();
        self
    }

    pub fn size(mut self, size: impl Into<DialogSize>) -> Self {
        self.size = size.into();
        self
    }
}

pub struct DialogBuilder<'a> {
    /// wiget to display in dialog
    widget: Widget<'a>,

    /// dialog theme
    theme: DialogTheme,
}

impl Default for DialogBuilder<'_> {
    fn default() -> Self {
        Self {
            widget: Widget::Text(Text::default()),
            theme: DialogTheme::default(),
        }
    }
}

impl<'a> DialogBuilder<'a> {
    #[must_use]
    pub fn widget(mut self, widget: Widget<'a>) -> Self {
        self.widget = widget;
        self
    }

    pub fn theme(mut self, theme: DialogTheme) -> Self {
        self.theme = theme;
        self
    }

    pub fn build(self) -> Dialog<'a> {
        Dialog {
            widget: self.widget,
            chunk: Default::default(),
            chunk_size: self.theme.size,
            base_style: self.theme.base_style,
        }
    }
}

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
#[derive(Debug, Clone, Copy)]
pub struct DialogSize {
    /// content width percentage (0.0 ~ 100.0)
    pub width: f32,
    /// content height percentage (0.0 ~ 100.0)
    pub height: f32,
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
    base_style: Style,
}

impl<'a> Dialog<'a> {
    #[allow(dead_code)]
    pub fn new(widget: Widget<'a>) -> Self {
        Self {
            widget,
            chunk: Default::default(),
            chunk_size: Default::default(),
            base_style: Style::default(),
        }
    }

    pub fn builder() -> DialogBuilder<'a> {
        DialogBuilder::default()
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
        f.render_widget(StyledClear::new(self.base_style), self.chunk);

        self.widget.render(f, true, false)
    }

    pub fn on_key_event(&mut self, ev: KeyEvent) -> EventResult {
        self.widget.on_key_event(ev)
    }
    pub fn on_mouse_event(&mut self, ev: MouseEvent) -> EventResult {
        self.widget.on_mouse_event(ev)
    }
}
