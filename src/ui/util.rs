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

pub mod chars {
    use unicode_segmentation::UnicodeSegmentation;
    use unicode_width::UnicodeWidthStr;

    const TAB_WIDTH: usize = 8;

    pub fn convert_tabs_to_spaces<T: AsRef<str>>(s: T) -> String {
        let s = s.as_ref();

        if !s.contains('\t') {
            return s.to_owned();
        }

        let mut result = String::with_capacity(s.len());

        let mut width = 0;
        for s in s.graphemes(true) {
            match s {
                "\t" => {
                    let spaces = TAB_WIDTH - (width % TAB_WIDTH);
                    result.push_str(&" ".repeat(spaces));
                    width += spaces;
                }
                _ => {
                    result.push_str(s);
                    width += s.width();
                }
            }
        }

        result
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        #[rustfmt::skip]
        #[rstest]
        #[case("1\t1",        "1       1")]
        #[case("12\t1",       "12      1")]
        #[case("123\t1",      "123     1")]
        #[case("1234\t1",     "1234    1")]
        #[case("12345\t1",    "12345   1")]
        #[case("123456\t1",   "123456  1")]
        #[case("1234567\t1",  "1234567 1")]
        #[case("12345678\t1", "12345678        1")]
        #[case("1\t\t1",      "1               1")]
        fn contains_tab_chars(#[case] input: &str, #[case] expected: &str) {
            assert_eq!(convert_tabs_to_spaces(input), expected)
        }

        #[test]
        fn not_contains_tab_chars() {
            let input = "hello";
            let expected = "hello";

            assert_eq!(convert_tabs_to_spaces(input), expected);
        }
    }
}
