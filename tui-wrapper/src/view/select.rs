use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::*,
    text::Span,
    widgets::{Block, Borders, Paragraph},
    Frame,
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
struct InputForm<'a> {
    content: String,
    cursor: Cursor,
    widget: Widget<'a>,
    width: usize,
    chunk: Rect,
}

impl Default for InputForm<'_> {
    fn default() -> Self {
        Self {
            content: String::default(),
            cursor: Cursor::default(),
            widget: Widget::Text(Text::default()),
            width: 1,
            chunk: Rect::default(),
        }
    }
}

impl<'a> InputForm<'a> {
    fn render<B: Backend>(&mut self, f: &mut Frame<B>) {
        let widget = Paragraph::new(self.cursor.cursor()).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Filter")
                .title_offset(1),
        );

        f.render_widget(widget, self.chunk);
    }

    fn update_chunk(&mut self, chunk: Rect) {
        self.chunk = chunk;
    }
}

#[derive(Debug)]
pub struct SelectForm<'a> {
    title: String,
    input_widget: InputForm<'a>,
    list_widget: Widget<'a>,
    selected_widget: Widget<'a>,
    layout: Layout,
    block: Block<'a>,
    chunk: Rect,
}

impl<'a> SelectForm<'a> {
    pub fn new(title: impl Into<String>) -> Self {
        // split [InputForm, SelectForms]
        // ---------------------
        // |     InputForm     |
        // |-------------------|
        // |         |         |
        // | Select  | Select  |
        // |         |         |
        // |         |         |
        // ---------------------
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(3)]);

        Self {
            title: title.into(),
            layout,
            ..Self::default()
        }
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = block;
        self
    }

    pub fn update_chunk(&mut self, chunk: Rect) {
        self.chunk = chunk;

        let inner_chunks = self.layout.split(self.block.inner(self.chunk));
        self.input_widget.update_chunk(inner_chunks[0]);
    }

    pub fn render<B: Backend>(&mut self, f: &mut Frame<B>) {
        f.render_widget(self.block.clone().title(self.title.as_str()), self.chunk);
        self.input_widget.render(f);
    }
}

// input widget
impl SelectForm<'_> {}

impl Default for SelectForm<'_> {
    fn default() -> Self {
        Self {
            title: String::default(),
            input_widget: InputForm::default(),
            list_widget: Widget::List(List::default()),
            selected_widget: Widget::List(List::default()),
            chunk: Rect::default(),
            layout: Layout::default(),
            block: Block::default(),
        }
    }
}
