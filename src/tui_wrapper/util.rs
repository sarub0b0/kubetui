use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent};

use ratatui::layout::{Constraint, Direction, Layout, Rect};

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

pub trait MousePosition {
    fn position(&self) -> (u16, u16);
}

impl MousePosition for MouseEvent {
    fn position(&self) -> (u16, u16) {
        (self.column, self.row)
    }
}

pub trait RectContainsPoint {
    fn contains_point(&self, point: (u16, u16)) -> bool;
}

impl RectContainsPoint for Rect {
    fn contains_point(&self, (x, y): (u16, u16)) -> bool {
        (self.left() <= x && x < self.right()) && (self.top() <= y && y < self.bottom())
    }
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
