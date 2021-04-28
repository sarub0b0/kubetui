use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::*,
    text::{Span, Spans},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use std::time::{Duration, Instant};

use super::focus_block;
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
        self.cursor.update_tick();

        let spans = Self::render_content(self.content.as_str(), &self.cursor);

        let widget = Paragraph::new(spans).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(Span::styled("Filter", Style::reset()))
                .title_offset(1),
        );

        f.render_widget(widget, self.chunk);
    }

    fn render_content(content: &str, cursor: &Cursor) -> Spans<'a> {
        match (content.len(), cursor.pos()) {
            (0, _) => Spans::from(Span::styled(" ", cursor.cursor_style())),
            (len, pos) if pos < len => Spans::from(
                content
                    .chars()
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
            _ => Spans::from(vec![
                Span::raw(content.to_string()),
                Span::styled(" ", cursor.cursor_style()),
            ]),
        }
    }

    fn update_chunk(&mut self, chunk: Rect) {
        self.chunk = chunk;
    }

    fn insert_char(&mut self, c: char) {
        self.content.insert(self.cursor.pos(), c);
        self.cursor.forward();
    }

    fn remove_char(&mut self) {
        if !self.content.is_empty() && 0 < self.cursor.pos() {
            self.cursor.back();
            self.content.remove(self.cursor.pos());
        }
    }

    fn forward_cursor(&mut self) {
        if self.cursor.pos() < self.content.len() {
            self.cursor.forward()
        }
    }
    fn back_cursor(&mut self) {
        self.cursor.back();
    }

    fn content(&self) -> &str {
        self.content.as_str()
    }
}

#[derive(Debug)]
struct SelectForm<'a> {
    list_items: Vec<String>,
    selected_items: Vec<String>,
    filter: String,
    list_widget: Widget<'a>,
    selected_widget: Widget<'a>,
    chunk: Vec<Rect>,
    focus_id: usize,
    layout: Layout,
}

impl Default for SelectForm<'_> {
    fn default() -> Self {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)]);
        Self {
            list_items: Vec::new(),
            selected_items: Vec::new(),
            filter: String::default(),
            list_widget: Widget::List(List::default()),
            selected_widget: Widget::List(List::default()),
            chunk: Vec::new(),
            focus_id: 0,
            layout,
        }
    }
}

impl<'a> SelectForm<'a> {
    fn render<B: Backend>(&mut self, f: &mut Frame<B>) {
        let mut ch_list = self.chunk[0];
        ch_list.width = ch_list.width.saturating_sub(1);

        let sum_width: u16 = self.chunk.iter().map(|c| c.width).sum();

        let is_odd_width = is_odd(sum_width);

        let sub = if is_odd(ch_list.height) { 0 } else { 1 };

        let arrow = if is_odd_width { "←→ " } else { "↔︎ " };

        let ch_arrow = Rect::new(
            ch_list.x + ch_list.width,
            ch_list.y + (ch_list.height / 2).saturating_sub(sub),
            arrow.chars().count() as u16,
            1,
        );

        let mut ch_selected = self.chunk[1];

        let addend = if is_odd_width { 2 } else { 1 };
        ch_selected.x = ch_selected.x.saturating_add(addend);
        ch_selected.width = ch_selected.width.saturating_sub(addend);

        let list = self.list_widget.list().unwrap();

        let w = list.widget(focus_block("Items", self.focus_id == 0));

        f.render_stateful_widget(w, ch_list, &mut list.state().borrow_mut());

        let w = Paragraph::new(arrow)
            .alignment(Alignment::Center)
            .block(Block::default());

        f.render_widget(w, ch_arrow);

        let list = self.selected_widget.list().unwrap();
        let w = list.widget(focus_block("Selected", self.focus_id == 1));

        f.render_stateful_widget(w, ch_selected, &mut list.state().borrow_mut());
    }

    fn filter_items(&self, items: &[String]) -> Vec<String> {
        items
            .iter()
            .filter_map(|item| {
                if item.starts_with(&self.filter) {
                    Some(item.to_string())
                } else {
                    None
                }
            })
            .collect()
    }

    fn update_chunk(&mut self, chunk: Rect) {
        self.chunk = self.layout.split(chunk);
    }

    fn select_next(&mut self) {
        self.focused_form_mut().select_next(1);
    }

    fn select_prev(&mut self) {
        self.focused_form_mut().select_prev(1);
    }

    fn focused_form_mut(&mut self) -> &mut Widget<'a> {
        if self.focus_id == 0 {
            &mut self.list_widget
        } else {
            &mut self.selected_widget
        }
    }

    fn unfocused_form_mut(&mut self) -> &mut Widget<'a> {
        if self.focus_id == 1 {
            &mut self.list_widget
        } else {
            &mut self.selected_widget
        }
    }

    fn toggle_focus(&mut self) {
        if self.focus_id == 0 {
            self.focus_id = 1
        } else {
            self.focus_id = 0
        }
    }

    fn toggle_select_unselect(&mut self) {
        // 1. フィルタされているアイテムをフォーカスしているリストからアイテムを取り出す
        // 2. 取得したアイテムをフォーカスしているリストから削除
        // 3  フォーカスしていないリストに追加
        let selected_item = if let Some(list) = self.focused_form_mut().list_mut() {
            if let Some(index) = list.selected() {
                Some(list.items()[index].to_string())
            } else {
                None
            }
        } else {
            None
        };

        if let Some(selected_item) = selected_item {
            // 1. 選択されたアイテムを探して
            // 2. 一覧から削除
            // 3. 選択中リストに追加
            let find = self
                .focused_items()
                .iter()
                .enumerate()
                .find(|(_i, s)| s == &&selected_item)
                .map(|(i, _)| i);

            if let Some(index) = find {
                let item = self.focused_item_mut().remove(index);
                self.unfocused_item_mut().push(item);
                self.unfocused_item_mut().sort();
            }

            let focused_item = if self.focus_id == 0 {
                self.filter_items(&self.list_items)
            } else {
                self.selected_items.clone()
            };

            let unfocused_item = if self.focus_id == 1 {
                self.filter_items(&self.list_items)
            } else {
                self.selected_items.clone()
            };

            if let Some(list) = self.focused_form_mut().list_mut() {
                list.set_items(WidgetItem::Array(focused_item));
            }
            if let Some(list) = self.unfocused_form_mut().list_mut() {
                list.set_items(WidgetItem::Array(unfocused_item));
            }
        }
    }

    fn focused_items(&self) -> &Vec<String> {
        if self.focus_id == 0 {
            &self.list_items
        } else {
            &self.selected_items
        }
    }

    fn focused_item_mut(&mut self) -> &mut Vec<String> {
        if self.focus_id == 0 {
            &mut self.list_items
        } else {
            &mut self.selected_items
        }
    }

    fn unfocused_item_mut(&mut self) -> &mut Vec<String> {
        if self.focus_id == 1 {
            &mut self.list_items
        } else {
            &mut self.selected_items
        }
    }

    fn set_items(&mut self, items: Vec<String>) {
        self.list_items = items;
        self.list_widget
            .set_items(WidgetItem::Array(self.list_items.clone()));
    }

    fn update_filter(&mut self, filter: &str) {
        self.filter = filter.to_string();
        self.focus_id = 0;

        self.list_widget
            .set_items(WidgetItem::Array(self.filter_items(&self.list_items)));
    }
}

const LAYOUT_INDEX_FOR_INPUT_FORM: usize = 0;
const LAYOUT_INDEX_FOR_SELECT_FORM: usize = 1;

#[derive(Debug)]
pub struct Select<'a> {
    title: String,
    input_widget: InputForm<'a>,
    selected_widget: SelectForm<'a>,
    layout: Layout,
    block: Block<'a>,
    chunk: Rect,
}

impl<'a> Select<'a> {
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

        self.input_widget
            .update_chunk(inner_chunks[LAYOUT_INDEX_FOR_INPUT_FORM]);

        self.selected_widget
            .update_chunk(inner_chunks[LAYOUT_INDEX_FOR_SELECT_FORM]);
    }

    pub fn render<B: Backend>(&mut self, f: &mut Frame<B>) {
        f.render_widget(self.block.clone().title(self.title.as_str()), self.chunk);
        self.input_widget.render(f);
        self.selected_widget.render(f);
    }

    pub fn insert_char(&mut self, c: char) {
        self.input_widget.insert_char(c);
        self.selected_widget
            .update_filter(self.input_widget.content());
    }

    pub fn remove_char(&mut self) {
        self.input_widget.remove_char();
        self.selected_widget
            .update_filter(self.input_widget.content());
    }

    pub fn forward_cursor(&mut self) {
        self.input_widget.forward_cursor();
    }

    pub fn back_cursor(&mut self) {
        self.input_widget.back_cursor();
    }

    pub fn toggle_focus(&mut self) {
        self.selected_widget.toggle_focus();
    }

    pub fn select_next(&mut self) {
        self.selected_widget.select_next();
    }

    pub fn select_prev(&mut self) {
        self.selected_widget.select_prev();
    }

    pub fn toggle_select_unselect(&mut self) {
        self.selected_widget.toggle_select_unselect();
    }

    pub fn set_items(&mut self, items: Vec<String>) {
        self.selected_widget.set_items(items);
    }
}

impl Default for Select<'_> {
    fn default() -> Self {
        Self {
            title: String::default(),
            input_widget: InputForm::default(),
            selected_widget: SelectForm::default(),
            chunk: Rect::default(),
            layout: Layout::default(),
            block: Block::default(),
        }
    }
}

#[inline]
fn is_odd(num: u16) -> bool {
    num & 1 != 0
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

            let input = "test";

            input.chars().for_each(|c| form.insert_char(c));

            assert_eq!(input, form.content);
        }

        #[test]
        fn insert_char() {
            let mut form = InputForm::default();

            let input = "test";

            input.chars().for_each(|c| form.insert_char(c));

            form.back_cursor();

            form.insert_char('a');

            assert_eq!("tesat", form.content);

            form.forward_cursor();
            form.forward_cursor();

            form.insert_char('b');
            assert_eq!("tesatb", form.content);
        }

        #[test]
        fn render_content_empty() {
            let form = InputForm::default();

            assert_eq!(
                InputForm::render_content(form.content.as_str(), &form.cursor),
                Spans::from(Span::styled(
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
                InputForm::render_content(form.content.as_str(), &form.cursor),
                Spans::from(vec![
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
                InputForm::render_content(form.content.as_str(), &form.cursor),
                Spans::from(vec![
                    Span::raw("a"),
                    Span::styled("b", Style::default().add_modifier(Modifier::REVERSED))
                ])
            );
        }
    }

    mod select {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn focus_toggle() {
            let mut select = Select::default();

            select.toggle_focus();
            assert_eq!(select.selected_widget.focus_id, 1);

            select.toggle_focus();
            assert_eq!(select.selected_widget.focus_id, 0);
        }
    }
}
