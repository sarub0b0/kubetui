use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent};
use tui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders},
};

pub fn key_event_to_code(key: KeyEvent) -> KeyCode {
    use KeyCode::*;

    match key.code {
        Char('p') if key.modifiers == KeyModifiers::CONTROL => Up,
        Char('n') if key.modifiers == KeyModifiers::CONTROL => Down,

        Char('b') if key.modifiers == KeyModifiers::CONTROL => Left,
        Char('f') if key.modifiers == KeyModifiers::CONTROL => Right,

        Char('u') if key.modifiers == KeyModifiers::CONTROL => PageUp,
        Char('d') if key.modifiers == KeyModifiers::CONTROL => PageDown,

        Char('h') if key.modifiers == KeyModifiers::CONTROL => Delete,
        Backspace => Delete,

        Char('a') if key.modifiers == KeyModifiers::CONTROL => Home,
        Char('e') if key.modifiers == KeyModifiers::CONTROL => End,

        Char('[') if key.modifiers == KeyModifiers::CONTROL => Esc,

        _ => key.code,
    }
}
#[inline]
pub fn mouse_pos(ev: MouseEvent) -> (u16, u16) {
    (ev.column, ev.row)
}

#[inline]
pub fn contains(chunk: Rect, point: (u16, u16)) -> bool {
    let (px, py) = point;
    (chunk.left() <= px && px <= chunk.right()) && (chunk.top() <= py && py <= chunk.bottom())
}

fn focus_border_color(focused: bool) -> Color {
    if focused {
        Color::Reset
    } else {
        Color::DarkGray
    }
}

fn focus_border_style(focused: bool) -> Style {
    Style::default().fg(focus_border_color(focused))
}

pub fn focus_title_style(focused: bool) -> Style {
    let style = Style::default();

    if focused {
        style.add_modifier(Modifier::BOLD)
    } else {
        style
    }
}

fn focus_mark_style(focused: bool) -> Style {
    if focused {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    }
}

pub fn generate_title(title: &str, focused: bool) -> Spans {
    let mark = if focused { "+" } else { "─" };
    let margin = if focused { " " } else { "─" };
    Spans::from(vec![
        Span::styled(margin, focus_border_style(focused)),
        Span::styled(mark, focus_mark_style(focused)),
        Span::styled(format!(" {} ", title), focus_title_style(focused)),
    ])
}

pub fn default_focus_block() -> Block<'static> {
    Block::default().borders(Borders::ALL)
}

pub fn focus_block(title: &str, focused: bool) -> Block {
    default_focus_block()
        .title(generate_title(title, focused))
        .title_offset(1)
        .border_style(focus_border_style(focused))
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
