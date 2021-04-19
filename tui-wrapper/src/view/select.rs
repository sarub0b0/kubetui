use tui::layout::Rect;
use tui::text::Span;
use tui::Frame;
use tui::{
    backend::Backend,
    style::*,
    widgets::{Block, Borders, Paragraph},
};

use std::time::{Duration, Instant};

use crate::widget::*;

#[derive(Debug)]
enum Mode {
    Show,
    Hide,
}

impl Mode {
    fn toggle(&mut self) {
        match self {
            Mode::Show => *self = Mode::Hide,
            Mode::Hide => *self = Mode::Show,
        }
    }
}

#[derive(Debug)]
struct Cursor {
    cursor: char,
    pos: usize,
    last_tick: Instant,
    tick_rate: Duration,
    mode: Mode,
}

impl Cursor {
    fn forward(&mut self) {
        self.pos.saturating_add(1);
    }

    fn back(&mut self) {
        self.pos.saturating_sub(1);
    }

    fn cursor(&mut self) -> Span {
        if self.tick_rate <= self.last_tick.elapsed() {
            self.mode.toggle();
            self.last_tick = Instant::now();
        }

        match self.mode {
            Mode::Show => Span::styled(
                self.cursor.to_string(),
                Style::default().add_modifier(Modifier::REVERSED),
            ),
            Mode::Hide => Span::raw(self.cursor.to_string()),
        }
    }
}

impl Default for Cursor {
    fn default() -> Self {
        Self {
            cursor: ' ',
            pos: 0,
            last_tick: Instant::now(),
            tick_rate: Duration::from_millis(1000),
            mode: Mode::Show,
        }
    }
}

#[derive(Debug)]
pub struct SelectForm<'a> {
    title: String,
    input_widget: Widget<'a>,
    list_widget: Widget<'a>,
    selected_widget: Widget<'a>,
    chunk: Rect,
    cursor: Cursor,
}

impl<'a> SelectForm<'a> {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            ..Self::default()
        }
    }

    pub fn render<B: Backend>(&mut self, f: &mut Frame<B>) {
        let widget = Paragraph::new(self.cursor.cursor()).block(
            Block::default()
                .borders(Borders::ALL)
                .title(Span::raw(&self.title)),
        );

        f.render_widget(widget, self.chunk);
    }

    pub fn update_chunk(&mut self, chunk: Rect) {
        self.chunk = chunk;
    }
}

// input widget
impl SelectForm<'_> {}

impl Default for SelectForm<'_> {
    fn default() -> Self {
        Self {
            title: String::default(),
            input_widget: Widget::Text(Text::default()),
            list_widget: Widget::List(List::default()),
            selected_widget: Widget::List(List::default()),
            chunk: Rect::default(),
            cursor: Cursor::default(),
        }
    }
}
