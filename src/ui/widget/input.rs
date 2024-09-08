use std::time::{Duration, Instant};

use derivative::Derivative;
use ratatui::{
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent},
    layout::{Constraint, Direction, Layout, Rect},
    style::*,
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame,
};

use crate::{
    message::UserEvent,
    ui::{
        event::{Callback, CallbackFn, EventResult},
        key_event_to_code,
        widget::*,
    },
};

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
    symbol: char,
    last_tick: Instant,
    tick_rate: Duration,
    mode: Mode,
}

impl Cursor {
    fn update_tick(&mut self) {
        if self.tick_rate <= self.last_tick.elapsed() {
            self.mode.toggle();
            self.last_tick = Instant::now();
        }
    }

    fn style(&self) -> Style {
        match self.mode {
            Mode::Show => Style::default().add_modifier(Modifier::REVERSED),
            Mode::Hide => Style::default(),
        }
    }

    fn reset(&mut self) {
        self.last_tick = Instant::now();
        self.mode = Mode::Show;
    }
}

impl Default for Cursor {
    fn default() -> Self {
        Self {
            symbol: ' ',
            last_tick: Instant::now(),
            tick_rate: Duration::from_millis(500),
            mode: Mode::Show,
        }
    }
}

#[derive(Debug, Default)]
struct Content {
    chars: Vec<char>,
    cursor_pos: usize,
    cursor: Cursor,
}

impl Content {
    fn len(&self) -> usize {
        self.chars.len()
    }

    fn clear(&mut self) {
        *self = Self::default();
    }

    fn max_cursor_pos(&self) -> usize {
        self.chars.len()
    }

    fn update_content(&mut self, s: String) {
        self.chars = s.chars().collect();
        self.cursor_end();
    }

    fn cursor_forward(&mut self, addend: usize) {
        self.cursor.reset();

        self.cursor_pos = self
            .cursor_pos
            .saturating_add(addend)
            .min(self.max_cursor_pos());
    }

    fn cursor_back(&mut self, subst: usize) {
        self.cursor.reset();

        self.cursor_pos = self.cursor_pos.saturating_sub(subst);
    }

    fn cursor_top(&mut self) {
        self.cursor.reset();

        self.cursor_pos = 0;
    }

    fn cursor_end(&mut self) {
        self.cursor.reset();

        self.cursor_pos = self.max_cursor_pos();
    }

    fn insert_char(&mut self, c: char) {
        self.cursor.reset();

        self.chars.insert(self.cursor_pos, c);
        self.cursor_forward(1);
    }

    fn remove_char(&mut self) {
        self.cursor.reset();

        if self.chars.is_empty() {
            return;
        }

        if self.cursor_pos == 0 {
            return;
        }

        self.cursor_back(1);
        self.chars.remove(self.cursor_pos);
    }

    fn remove_chars_before_cursor(&mut self) {
        self.cursor.reset();

        self.chars = self.chars[self.cursor_pos..].to_vec();
        self.cursor_pos = 0;
    }

    fn remove_chars_after_cursor(&mut self) {
        self.cursor.reset();

        self.chars = self.chars[..self.cursor_pos].to_vec();
    }

    fn rendered_content(&mut self, is_active: bool) -> Line<'static> {
        if is_active {
            self.cursor.update_tick();
        } else {
            self.cursor.mode = Mode::Hide
        }

        if self.chars.is_empty() {
            return Line::from(Span::styled(
                self.cursor.symbol.to_string(),
                self.cursor.style(),
            ));
        }

        if self.cursor_pos < self.chars.len() {
            return Line::from(
                self.chars
                    .iter()
                    .enumerate()
                    .map(|(i, c)| {
                        if i == self.cursor_pos {
                            Span::styled(c.to_string(), self.cursor.style())
                        } else {
                            Span::raw(c.to_string())
                        }
                    })
                    .collect::<Vec<Span>>(),
            );
        }

        Line::from(vec![
            Span::raw(self.chars.iter().collect::<String>()),
            Span::styled(self.cursor.symbol.to_string(), self.cursor.style()),
        ])
    }
}

#[derive(Derivative)]
#[derivative(Debug, Default)]
pub struct InputFormBuilder {
    id: String,
    widget_config: WidgetConfig,
    prefix: Line<'static>,
    suffix: Line<'static>,
    #[derivative(Debug = "ignore")]
    actions: Vec<(UserEvent, Callback)>,
}

impl InputFormBuilder {
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    pub fn widget_config(mut self, widget_config: WidgetConfig) -> Self {
        self.widget_config = widget_config;
        self
    }

    pub fn prefix(mut self, prefix: impl Into<Line<'static>>) -> Self {
        self.prefix = prefix.into();
        self
    }

    pub fn suffix(mut self, suffix: impl Into<Line<'static>>) -> Self {
        self.suffix = suffix.into();
        self
    }

    pub fn actions<E, F>(mut self, ev: E, cb: F) -> Self
    where
        E: Into<UserEvent>,
        F: CallbackFn,
    {
        self.actions.push((ev.into(), Callback::new(cb)));
        self
    }

    pub fn build(self) -> InputForm {
        InputForm {
            id: self.id,
            widget_config: self.widget_config,
            prefix: self.prefix,
            suffix: self.suffix,
            actions: self.actions,
            ..Default::default()
        }
    }
}

/// 検索・フィルタリング用の入力フォーム
/// 複数行は扱わない
#[derive(Derivative)]
#[derivative(Debug, Default)]
pub struct InputForm {
    id: String,
    content: Content,
    chunk: Rect,
    content_chunk: Rect,
    prefix_chunk: Rect,
    suffix_chunk: Rect,
    layout: Layout,
    prefix: Line<'static>,
    suffix: Line<'static>,
    widget_config: WidgetConfig,
    scroll: usize,
    #[derivative(Debug = "ignore")]
    actions: Vec<(UserEvent, Callback)>,
}

impl InputForm {
    pub fn builder() -> InputFormBuilder {
        InputFormBuilder::default()
    }

    fn layout(&self, prefix: u16, suffix: u16) -> Layout {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(prefix),
                Constraint::Percentage(100),
                Constraint::Length(suffix),
            ])
    }

    fn block(&self, is_active: bool, is_mouse_over: bool) -> Block<'static> {
        self.widget_config
            .render_block(self.can_activate() && is_active, is_mouse_over)
    }

    pub fn update_chunk(&mut self, chunk: Rect) {
        self.chunk = chunk;

        let inner_chunk = self.widget_config().block().inner(chunk);

        let prefix_width = self.prefix.width() as u16;
        let suffix_width = self.suffix.width() as u16;

        let chunks = self.layout(prefix_width, suffix_width).split(inner_chunk);

        self.prefix_chunk = chunks[0];
        self.content_chunk = chunks[1];
        self.suffix_chunk = chunks[2];

        self.adjust_scroll_for_cursor();
    }

    pub fn update_prefix(&mut self, prefix: impl Into<Line<'static>>) {
        self.prefix = prefix.into();

        self.update_chunk(self.chunk);
    }

    pub fn update_suffix(&mut self, suffix: impl Into<Line<'static>>) {
        self.suffix = suffix.into();

        self.update_chunk(self.chunk);
    }

    pub fn update_content(&mut self, content: String) {
        self.clear();

        self.content.update_content(content);

        self.move_cursor_end();
    }

    pub fn insert_char(&mut self, c: char) {
        self.content.insert_char(c);
        self.scroll_right(1);
    }

    pub fn remove_char(&mut self) {
        self.content.remove_char();
        self.scroll_left(1);
    }

    pub fn remove_chars_before_cursor(&mut self) {
        self.content.remove_chars_before_cursor();
        self.scroll = 0;
    }

    pub fn remove_chars_after_cursor(&mut self) {
        self.content.remove_chars_after_cursor();
    }

    pub fn forward_cursor(&mut self) {
        self.content.cursor_forward(1);
        self.scroll_right(1);
    }
    pub fn back_cursor(&mut self) {
        self.content.cursor_back(1);
        self.scroll_left(1);
    }

    pub fn content(&self) -> String {
        self.content.chars.iter().collect()
    }

    pub fn clear(&mut self) {
        self.content.clear();
        self.scroll = 0;
    }

    pub fn move_cursor_top(&mut self) {
        self.content.cursor_top();
        self.scroll = 0;
    }

    pub fn move_cursor_end(&mut self) {
        self.content.cursor_end();
        self.scroll = self.max_scroll();
    }

    /// カーソルがフォーム内の右端に来るようスクロール位置を調整する
    fn adjust_scroll_for_cursor(&mut self) {
        if self.is_cursor_right_inside_render_area() {
            return;
        }

        self.scroll = self
            .content
            .cursor_pos
            .saturating_sub(self.content_width())
            .saturating_add(1)
            .min(self.max_scroll());
    }

    fn scroll_right(&mut self, addend: usize) {
        if self.is_cursor_right_inside_render_area() {
            return;
        }

        self.scroll = self.scroll.saturating_add(addend).min(self.max_scroll());
    }

    fn scroll_left(&mut self, subst: usize) {
        if self.is_cursor_left_inside_render_area() {
            return;
        }

        self.scroll = self.scroll.saturating_sub(subst);
    }

    fn is_cursor_left_inside_render_area(&self) -> bool {
        self.scroll <= self.content.cursor_pos
    }

    fn is_cursor_right_inside_render_area(&self) -> bool {
        self.content.cursor_pos < self.scroll.saturating_add(self.content_width())
    }

    fn max_scroll(&self) -> usize {
        const CURSOR_LEN: usize = 1;

        (self.content.len() + CURSOR_LEN).saturating_sub(self.content_width())
    }

    fn content_width(&self) -> usize {
        self.content_chunk.width as usize
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
                if let Some(cb) = self.match_action(UserEvent::Key(key)) {
                    return EventResult::Callback(cb.clone());
                }
                return EventResult::Ignore;
            }
        }

        EventResult::Nop
    }

    fn match_action(&self, ev: UserEvent) -> Option<&Callback> {
        self.actions
            .iter()
            .find_map(|(cb_ev, cb)| if *cb_ev == ev { Some(cb) } else { None })
    }
}

impl WidgetTrait for InputForm {
    fn id(&self) -> &str {
        &self.id
    }

    fn widget_config(&self) -> &WidgetConfig {
        &self.widget_config
    }

    fn widget_config_mut(&mut self) -> &mut WidgetConfig {
        &mut self.widget_config
    }

    fn can_activate(&self) -> bool {
        true
    }

    fn widget_item(&self) -> Option<SelectedItem> {
        if self.content().is_empty() {
            None
        } else {
            Some(SelectedItem::Literal {
                metadata: None,
                item: self.content(),
            })
        }
    }

    fn chunk(&self) -> Rect {
        self.chunk
    }

    fn select_index(&mut self, _: usize) {
        unreachable!();
    }

    fn select_next(&mut self, _: usize) {
        unreachable!();
    }

    fn select_prev(&mut self, _: usize) {
        unreachable!();
    }

    fn select_first(&mut self) {
        unreachable!();
    }

    fn select_last(&mut self) {
        unreachable!();
    }

    fn append_widget_item(&mut self, _: Item) {
        unreachable!();
    }

    fn update_widget_item(&mut self, item: Item) {
        let LiteralItem { item, .. } = item.single();

        self.update_content(item);
    }

    fn on_mouse_event(&mut self, ev: MouseEvent) -> EventResult {
        self.on_mouse_event(ev)
    }

    fn on_key_event(&mut self, ev: KeyEvent) -> EventResult {
        self.on_key_event(ev)
    }

    fn update_chunk(&mut self, chunk: Rect) {
        self.update_chunk(chunk);
    }

    fn clear(&mut self) {
        self.clear()
    }
}

impl RenderTrait for InputForm {
    fn render(&mut self, f: &mut Frame, is_active: bool, is_mouse_over: bool) {
        // ブロックの描画
        let block = self.block(is_active, is_mouse_over);
        f.render_widget(block, self.chunk);

        // プレフィックスの描画
        f.render_widget(Paragraph::new(self.prefix.clone()), self.prefix_chunk);

        // コンテンツの描画
        let content = self.content.rendered_content(is_active);
        let widget = Paragraph::new(content).scroll((0, self.scroll as u16));
        f.render_widget(widget, self.content_chunk);

        // サフィックスの描画
        f.render_widget(Paragraph::new(self.suffix.clone()), self.suffix_chunk);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod content {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn push_char() {
            let mut content = Content::default();

            "test".chars().for_each(|c| content.insert_char(c));

            assert_eq!("test".chars().collect::<Vec<char>>(), content.chars);
        }

        #[test]
        fn insert_char() {
            let mut content = Content::default();

            "test".chars().for_each(|c| content.insert_char(c));

            content.cursor_back(1);

            content.insert_char('a');

            assert_eq!("tesat".chars().collect::<Vec<char>>(), content.chars);

            content.cursor_forward(1);
            content.cursor_forward(1);

            content.insert_char('b');
            assert_eq!("tesatb".chars().collect::<Vec<char>>(), content.chars);
        }

        #[test]
        fn insert_char_fullwidth() {
            let mut content = Content::default();

            let input = "あいうえお";

            input.chars().for_each(|c| content.insert_char(c));

            content.cursor_back(1);

            content.insert_char('ア');

            assert_eq!("あいうえアお".chars().collect::<Vec<char>>(), content.chars);

            content.cursor_forward(1);
            content.cursor_forward(1);

            content.insert_char('イ');
            assert_eq!(
                "あいうえアおイ".chars().collect::<Vec<char>>(),
                content.chars
            );
        }

        #[test]
        fn render_content_empty() {
            let mut content = Content::default();

            assert_eq!(
                content.rendered_content(true),
                Line::from(Span::styled(
                    " ",
                    Style::default().add_modifier(Modifier::REVERSED)
                ))
            );
        }

        #[test]
        fn render_content_add_char() {
            let mut content = Content::default();

            content.insert_char('a');
            content.insert_char('b');

            assert_eq!(
                content.rendered_content(true),
                Line::from(vec![
                    Span::raw("ab"),
                    Span::styled(" ", Style::default().add_modifier(Modifier::REVERSED))
                ])
            );
        }

        #[test]
        fn render_content_add_char_and_cursor_back() {
            let mut content = Content::default();

            content.insert_char('a');
            content.insert_char('b');
            content.cursor_back(1);

            assert_eq!(
                content.rendered_content(true),
                Line::from(vec![
                    Span::raw("a"),
                    Span::styled("b", Style::default().add_modifier(Modifier::REVERSED))
                ])
            );
        }
    }
}
