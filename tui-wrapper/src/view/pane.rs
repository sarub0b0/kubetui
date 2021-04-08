use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::widget::*;

pub struct Pane<'a> {
    widget: Widget<'a>,
    chunk_index: usize,
    title: String,
    id: String,
    chunk: Rect,
}

impl<'a> Pane<'a> {
    pub fn new(
        title: impl Into<String>,
        widget: Widget<'a>,
        chunk_index: usize,
        id: impl Into<String>,
    ) -> Self {
        Self {
            title: title.into(),
            widget,
            chunk_index,
            id: id.into(),
            chunk: Rect::default(),
        }
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

    pub fn chunk_index(&self) -> usize {
        self.chunk_index
    }

    pub fn next_item(&mut self, index: usize) {
        self.widget.select_next(index)
    }

    pub fn prev_item(&mut self, index: usize) {
        self.widget.select_prev(index)
    }

    pub fn set_items(&mut self, items: Vec<String>) {
        self.widget.set_items(items);
    }

    pub fn is_selected(&self, rhs: &Pane) -> bool {
        return std::ptr::eq(self, rhs);
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn update_chunk(&mut self, chunk: Rect) {
        self.chunk = chunk;
    }

    pub fn chunk(&self) -> Rect {
        self.chunk
    }

    pub fn block(&self, selected: bool) -> Block {
        let border_color = if selected {
            Color::White
        } else {
            Color::DarkGray
        };

        Block::default()
            .title(generate_title(&self.title, border_color, selected))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
    }
}

fn generate_title(title: &str, border_color: Color, selected: bool) -> Spans {
    let prefix = if selected { "✔︎ " } else { "──" };
    Spans::from(vec![
        Span::styled("─", Style::default()),
        Span::styled(prefix, Style::default().fg(border_color)),
        Span::styled(
            title,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
    ])
}
