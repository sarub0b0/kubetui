use crate::ui::{event::EventResult, key_event_to_code, widget::*};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent};

use ratatui::{
    backend::Backend,
    layout::Rect,
    style::*,
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame,
};

use std::time::{Duration, Instant};

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
        self.pos = self.pos.saturating_add(1);
        self.last_tick = Instant::now();
        self.mode = Mode::Show;
    }

    fn back(&mut self) {
        self.pos = self.pos.saturating_sub(1);
        self.last_tick = Instant::now();
        self.mode = Mode::Show;
    }

    fn update_tick(&mut self) {
        if self.tick_rate <= self.last_tick.elapsed() {
            self.mode.toggle();
            self.last_tick = Instant::now();
        }
    }

    fn cursor_style(&self) -> Style {
        match self.mode {
            Mode::Show => Style::default().add_modifier(Modifier::REVERSED),
            Mode::Hide => Style::default(),
        }
    }

    fn pos(&self) -> usize {
        self.pos
    }
}

impl Default for Cursor {
    fn default() -> Self {
        Self {
            cursor: ' ',
            pos: 0,
            last_tick: Instant::now(),
            tick_rate: Duration::from_millis(500),
            mode: Mode::Show,
        }
    }
}

#[derive(Debug, Default)]
pub struct InputForm {
    content: Vec<char>,
    cursor: Cursor,
    chunk: Rect,
    widget_config: WidgetConfig,
}

impl InputForm {
    pub fn new(widget_config: WidgetConfig) -> Self {
        Self {
            widget_config,
            ..Default::default()
        }
    }

    fn block(&self, selected: bool) -> Block<'static> {
        self.widget_config.render_block(selected)
    }

    pub fn render<B: Backend>(&mut self, f: &mut Frame<B>, selected: bool) {
        let spans = self.render_content(selected);
        let block = self.block(selected);
        let chunk = self.chunk;

        let widget = Paragraph::new(spans).block(block);

        f.render_widget(widget, chunk);
    }

    pub fn render_content(&mut self, selected: bool) -> Line<'static> {
        if selected {
            self.cursor.update_tick();
        } else {
            self.cursor.mode = Mode::Hide
        }

        let cursor = &self.cursor;
        let content = &self.content;

        match (content.len(), cursor.pos()) {
            (0, _) => Line::from(Span::styled(
                cursor.cursor.to_string(),
                cursor.cursor_style(),
            )),
            (len, pos) if pos < len => Line::from(
                content
                    .iter()
                    .enumerate()
                    .map(|(i, c)| {
                        if i == pos {
                            Span::styled(c.to_string(), cursor.cursor_style())
                        } else {
                            Span::raw(c.to_string())
                        }
                    })
                    .collect::<Vec<Span>>(),
            ),
            _ => Line::from(vec![
                Span::raw(content.iter().collect::<String>()),
                Span::styled(" ", cursor.cursor_style()),
            ]),
        }
    }

    pub fn update_chunk(&mut self, chunk: Rect) {
        self.chunk = chunk;
    }

    pub fn insert_char(&mut self, c: char) {
        self.content.insert(self.cursor.pos(), c);
        self.cursor.forward();
    }

    pub fn remove_char(&mut self) {
        if !self.content.is_empty() && 0 < self.cursor.pos() {
            self.cursor.back();
            self.content.remove(self.cursor.pos());
        }
    }

    pub fn remove_chars_before_cursor(&mut self) {
        self.content = self.content[self.cursor.pos..].to_vec();
        self.cursor.pos = 0;
    }

    pub fn remove_chars_after_cursor(&mut self) {
        self.content = self.content[..self.cursor.pos].to_vec();
    }

    pub fn forward_cursor(&mut self) {
        if self.cursor.pos() < self.content.len() {
            self.cursor.forward()
        }
    }
    pub fn back_cursor(&mut self) {
        self.cursor.back();
    }

    pub fn content(&self) -> String {
        self.content.iter().collect()
    }

    pub fn clear(&mut self) {
        self.cursor = Cursor::default();
        self.content.clear();
    }

    pub fn move_cursor_top(&mut self) {
        self.cursor.pos = 0;
    }

    pub fn move_cursor_end(&mut self) {
        self.cursor.pos = self.content.len();
    }

    pub fn on_mouse_event(&mut self, _: MouseEvent) -> EventResult {
        EventResult::Ignore
    }

    pub fn on_key_event(&mut self, key: KeyEvent) -> EventResult {
        match key_event_to_code(key) {
            KeyCode::Delete => {
                self.remove_char();
            }

            KeyCode::Char('w') if key.modifiers == KeyModifiers::CONTROL => {
                self.remove_chars_before_cursor();
            }

            KeyCode::Char('k') if key.modifiers == KeyModifiers::CONTROL => {
                self.remove_chars_after_cursor();
            }

            KeyCode::Home => {
                self.move_cursor_top();
            }

            KeyCode::End => {
                self.move_cursor_end();
            }

            KeyCode::Right => {
                self.forward_cursor();
            }

            KeyCode::Left => {
                self.back_cursor();
            }

            KeyCode::Char(c) => {
                self.insert_char(c);
            }
            _ => {
                return EventResult::Ignore;
            }
        }
        EventResult::Nop
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod cursor {
        use super::*;
        use pretty_assertions::assert_eq;
        #[test]
        fn move_forward() {
            let mut cursor = Cursor::default();
            cursor.forward();

            assert_eq!(cursor.pos(), 1);
        }

        #[test]
        fn move_back() {
            let mut cursor = Cursor::default();
            cursor.forward();
            cursor.forward();
            cursor.back();

            assert_eq!(cursor.pos(), 1);
        }
    }

    mod input_form {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn push_char() {
            let mut form = InputForm::default();

            "test".chars().for_each(|c| form.insert_char(c));

            assert_eq!("test".chars().collect::<Vec<char>>(), form.content);
        }

        #[test]
        fn insert_char() {
            let mut form = InputForm::default();

            "test".chars().for_each(|c| form.insert_char(c));

            form.back_cursor();

            form.insert_char('a');

            assert_eq!("tesat".chars().collect::<Vec<char>>(), form.content);

            form.forward_cursor();
            form.forward_cursor();

            form.insert_char('b');
            assert_eq!("tesatb".chars().collect::<Vec<char>>(), form.content);
        }

        #[test]
        fn insert_char_fullwidth() {
            let mut form = InputForm::default();

            let input = "あいうえお";

            input.chars().for_each(|c| form.insert_char(c));

            form.back_cursor();

            form.insert_char('ア');

            assert_eq!("あいうえアお".chars().collect::<Vec<char>>(), form.content);

            form.forward_cursor();
            form.forward_cursor();

            form.insert_char('イ');
            assert_eq!(
                "あいうえアおイ".chars().collect::<Vec<char>>(),
                form.content
            );
        }

        #[test]
        fn render_content_empty() {
            let mut form = InputForm::default();

            assert_eq!(
                form.render_content(true),
                Line::from(Span::styled(
                    " ",
                    Style::default().add_modifier(Modifier::REVERSED)
                ))
            );
        }

        #[test]
        fn render_content_add_char() {
            let mut form = InputForm::default();

            form.insert_char('a');
            form.insert_char('b');

            assert_eq!(
                form.render_content(true),
                Line::from(vec![
                    Span::raw("ab"),
                    Span::styled(" ", Style::default().add_modifier(Modifier::REVERSED))
                ])
            );
        }

        #[test]
        fn render_content_add_char_and_cursor_back() {
            let mut form = InputForm::default();

            form.insert_char('a');
            form.insert_char('b');
            form.back_cursor();

            assert_eq!(
                form.render_content(true),
                Line::from(vec![
                    Span::raw("a"),
                    Span::styled("b", Style::default().add_modifier(Modifier::REVERSED))
                ])
            );
        }
    }
}
