pub mod pane;
pub mod tab;
pub mod widget;

pub use pane::Pane;
pub use tab::Tab;

use tui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders},
};

pub fn focus_border_color(selected: bool) -> Color {
    if selected {
        Color::Reset
    } else {
        Color::DarkGray
    }
}

pub fn focus_border_style(selected: bool) -> Style {
    Style::default().fg(focus_border_color(selected))
}

pub fn focus_title_style(selected: bool) -> Style {
    let mut style = Style::default().fg(Color::White);

    if selected {
        style = style.add_modifier(Modifier::BOLD);
    }
    style
}

pub fn generate_title(title: &str, selected: bool) -> Spans {
    let prefix = if selected { " ✔︎ " } else { "───" };
    Spans::from(vec![
        Span::styled("─", Style::default()),
        Span::styled(prefix, focus_border_style(selected)),
        Span::styled(title, focus_title_style(selected)),
    ])
}

pub fn focus_block(title: &str, selected: bool) -> Block {
    Block::default()
        .borders(Borders::ALL)
        .title(generate_title(title, selected))
        .border_style(focus_border_style(selected))
}

pub fn child_window_chunk(width_rate: u16, height_rate: u16, chunk: Rect) -> Rect {
    let w = width_rate;
    let h = height_rate;
    let chunk = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - h) / 2),
            Constraint::Percentage(h),
            Constraint::Percentage((100 - h) / 2),
        ])
        .split(chunk);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - w) / 2),
            Constraint::Percentage(w),
            Constraint::Percentage((100 - w) / 2),
        ])
        .split(chunk[1])[1]
}
