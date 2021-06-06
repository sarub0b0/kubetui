use crate::{
    crossterm::event::{KeyCode, KeyEvent, MouseEvent},
    event::EventResult,
    tui::{
        backend::Backend,
        layout::{Alignment, Constraint, Direction, Layout, Rect},
        style::*,
        text::Span,
        widgets::{Block, Paragraph},
        Frame,
    },
    util::{
        contains, default_focus_block, focus_block, focus_title_style, key_event_to_code, mouse_pos,
    },
    widget::*,
    Window,
};

use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

use unicode_width::UnicodeWidthStr;

use derivative::*;
use std::collections::HashSet;

use super::input::InputForm;

#[derive(Derivative)]
#[derivative(Debug)]
struct SelectForm<'a> {
    list_items: HashSet<String>,
    selected_items: HashSet<String>,
    filter: String,
    list_widget: List<'a>,
    selected_widget: List<'a>,
    chunk: Rect,
    focus_id: usize,
    direction: Direction,
    #[derivative(Debug = "ignore")]
    matcher: SkimMatcherV2,
}

impl Default for SelectForm<'_> {
    fn default() -> Self {
        Self {
            list_items: HashSet::new(),
            selected_items: HashSet::new(),
            filter: String::default(),
            list_widget: List::default(),
            selected_widget: List::default(),
            chunk: Rect::default(),
            focus_id: 0,
            matcher: SkimMatcherV2::default(),
            direction: Direction::Vertical,
        }
    }
}

impl<'a> SelectForm<'a> {
    fn chunks_and_arrow(&self) -> ([Rect; 3], String) {
        match self.direction {
            Direction::Horizontal => {
                let arrow = if is_odd(self.chunk.width) {
                    "-->"
                } else {
                    "->"
                };

                let (cx, cy, cw, ch) = (
                    self.chunk.x,
                    self.chunk.y,
                    self.chunk.width / 2 - 1,
                    self.chunk.height,
                );

                let left_chunk = Rect::new(cx, cy, cw, ch);
                let center_chunk =
                    Rect::new(left_chunk.x + cw, cy + ch / 2, arrow.width() as u16, ch / 2);
                let right_chunk = Rect::new(center_chunk.x + arrow.width() as u16, cy, cw, ch);

                ([left_chunk, center_chunk, right_chunk], arrow.to_string())
            }
            Direction::Vertical => {
                let margin = if is_odd(self.chunk.height) { 0 } else { 1 };

                let (cx, cy, cw, ch) = (
                    self.chunk.x,
                    self.chunk.y,
                    self.chunk.width,
                    self.chunk.height / 2,
                );

                let left_chunk = Rect::new(cx, cy, cw, ch);
                let center_chunk = Rect::new(cx, cy + ch, cw, 1);
                let right_chunk = Rect::new(cx, center_chunk.y + 1, cw, ch - margin);

                ([left_chunk, center_chunk, right_chunk], "↓".to_string())
            }
        }
    }

    fn render<B: Backend>(&mut self, f: &mut Frame<B>, _: bool) {
        let (chunks, arrow) = self.chunks_and_arrow();

        let arrow = Paragraph::new(Span::styled(
            arrow,
            Style::default().add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Center)
        .block(Block::default());

        self.list_widget.render(f, self.focus_id == 0);

        f.render_widget(arrow, chunks[1]);

        self.selected_widget.render(f, self.focus_id == 1);
    }

    fn update_layout(&mut self, chunk: Rect) {
        if 65 < chunk.width {
            self.direction = Direction::Horizontal;
        } else {
            self.direction = Direction::Vertical;
        };
    }

    fn filter_items(&self, items: &HashSet<String>) -> Vec<String> {
        let mut ret: Vec<String> = items
            .iter()
            .filter_map(|item| {
                self.matcher
                    .fuzzy_match(&item, &self.filter)
                    .map(|_| item.to_string())
            })
            .collect();
        ret.sort();
        ret
    }

    fn update_chunk(&mut self, chunk: Rect) {
        self.update_layout(chunk);

        self.chunk = chunk;

        let (chunks, _) = self.chunks_and_arrow();

        self.list_widget.update_chunk(chunks[0]);
        self.selected_widget.update_chunk(chunks[2]);
    }

    fn select_next(&mut self, i: usize) {
        self.focused_form_mut().select_next(i);
    }

    fn select_prev(&mut self, i: usize) {
        self.focused_form_mut().select_prev(i);
    }

    fn select_first(&mut self) {
        self.focused_form_mut().select_first();
    }

    fn select_last(&mut self) {
        self.focused_form_mut().select_last();
    }

    fn focused_form_mut(&mut self) -> &mut List<'a> {
        if self.focus_id == 0 {
            &mut self.list_widget
        } else {
            &mut self.selected_widget
        }
    }

    fn unfocused_form_mut(&mut self) -> &mut List<'a> {
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

    fn focus(&mut self, id: usize) {
        self.focus_id = id;
    }

    fn toggle_select_unselect(&mut self) {
        // 1. フィルタされているアイテムをフォーカスしているリストからアイテムを取り出す
        // 2. 取得したアイテムをフォーカスしているリストから削除
        // 3  フォーカスしていないリストに追加
        let list = self.focused_form_mut();
        let selected_item = list.selected().map(|index| list.items()[index].to_string());

        if let Some(selected_item) = selected_item {
            self.swap_item(&selected_item)
        }
    }

    fn swap_item(&mut self, item: &str) {
        // 1. 選択されたアイテムを探して
        // 2. 一覧から削除
        // 3. 選択中リストに追加
        self.focused_item_mut().remove(item);
        self.unfocused_item_mut().insert(item.to_string());

        let mut focused_item = if self.focus_id == 0 {
            self.filter_items(&self.list_items)
        } else {
            self.selected_items.clone().into_iter().collect()
        };

        let mut unfocused_item = if self.focus_id == 1 {
            self.filter_items(&self.list_items)
        } else {
            self.selected_items.clone().into_iter().collect()
        };

        focused_item.sort();
        unfocused_item.sort();

        self.focused_form_mut()
            .set_items(WidgetItem::Array(focused_item));

        self.unfocused_form_mut()
            .set_items(WidgetItem::Array(unfocused_item));
    }

    fn focused_item_mut(&mut self) -> &mut HashSet<String> {
        if self.focus_id == 0 {
            &mut self.list_items
        } else {
            &mut self.selected_items
        }
    }

    fn unfocused_item_mut(&mut self) -> &mut HashSet<String> {
        if self.focus_id == 1 {
            &mut self.list_items
        } else {
            &mut self.selected_items
        }
    }

    fn set_items(&mut self, items: Vec<String>) {
        // マージ処理
        let new_items: HashSet<String> = items.into_iter().collect();
        let mut old_items = self.list_items.clone();

        self.selected_items.iter().for_each(|item| {
            old_items.insert(item.to_string());
        });

        // 新しく増えたアイテム
        // アイテムリストに追加
        let add_items = new_items.difference(&old_items);
        add_items.for_each(|item| {
            self.list_items.insert(item.to_string());
        });

        // 消えたアイテム
        // 両方のリストから削除
        let del_items = old_items.difference(&new_items);
        del_items.for_each(|item| {
            self.list_items.remove(item);
            self.selected_items.remove(item);
        });

        let filter = self.filter.clone();

        self.update_filter(&filter);

        let mut items: Vec<String> = self.selected_items.clone().into_iter().collect();
        items.sort();

        self.selected_widget.set_items(WidgetItem::Array(items));
    }

    fn update_filter(&mut self, filter: &str) {
        self.filter = filter.to_string();

        self.list_widget
            .set_items(WidgetItem::Array(self.filter_items(&self.list_items)));

        let current_pos = self.list_widget.state().selected();

        if let Some(pos) = current_pos {
            if self.list_widget.items().len() <= pos {
                self.list_widget.select_last()
            }
        }
    }

    fn status(&self) -> (usize, usize) {
        let mut pos = self.list_widget.state().selected().unwrap_or(0);

        let size = self.list_widget.items().len();

        if 0 < size {
            pos += 1;
        }

        (pos, size)
    }

    fn to_vec_selected_items(&self) -> Vec<String> {
        let mut vec: Vec<String> = self.selected_items.clone().into_iter().collect();
        vec.sort();
        vec
    }

    fn selected_items(&self) -> &HashSet<String> {
        &self.selected_items
    }

    fn on_mouse_event(&mut self, ev: MouseEvent) -> EventResult {
        let pos = mouse_pos(ev);

        let (chunks, _) = self.chunks_and_arrow();

        if contains(chunks[0], pos) {
            self.focus(0);
            self.list_widget.on_mouse_event(ev)
        } else if contains(chunks[2], pos) {
            self.focus(1);
            self.selected_widget.on_mouse_event(ev)
        } else {
            EventResult::Nop
        }
    }

    fn on_key_event(&mut self, ev: KeyEvent) -> EventResult {
        self.focused_form_mut().on_key_event(ev)
    }
}

const LAYOUT_INDEX_FOR_INPUT_FORM: usize = 0;
const LAYOUT_INDEX_FOR_STATUS: usize = 1;
const LAYOUT_INDEX_FOR_SELECT_FORM: usize = 2;

#[derive(Derivative)]
#[derivative(Debug, Default)]
pub struct MultipleSelect<'a> {
    id: String,
    title: String,
    chunk_index: usize,
    input_widget: InputForm<'a>,
    selected_widget: SelectForm<'a>,
    layout: Layout,
    chunk: Rect,
}

impl RenderTrait for MultipleSelect<'_> {
    fn render<B: Backend>(&mut self, f: &mut Frame<B>, selected: bool) {
        let title = self.title.to_string();
        let block = focus_block(&title, selected);
        let inner_chunk = block.inner(self.chunk);

        f.render_widget(
            block.title(Span::styled(
                format!(" {} ", self.title()),
                focus_title_style(selected),
            )),
            self.chunk,
        );

        self.input_widget.render(f);

        let status = self.selected_widget.status();
        f.render_widget(
            Paragraph::new(format!("[{}/{}]", status.0, status.1)),
            self.layout.split(inner_chunk)[LAYOUT_INDEX_FOR_STATUS],
        );
        self.selected_widget.render(f, selected);
    }
}

impl<'a> MultipleSelect<'a> {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
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
            .constraints([
                Constraint::Length(3),
                Constraint::Length(1),
                Constraint::Min(3),
            ]);

        let mut selected_widget = SelectForm::default();

        selected_widget.list_widget = selected_widget.list_widget.set_title("Item");
        selected_widget.selected_widget = selected_widget.selected_widget.set_title("Selected");

        Self {
            id: id.into(),
            title: title.into(),
            layout,
            selected_widget,
            ..Self::default()
        }
    }

    pub fn on_select<F>(mut self, f: F) -> Self
    where
        F: Fn(&mut Window, &String) -> EventResult + 'static + Clone,
    {
        self.selected_widget.list_widget = self.selected_widget.list_widget.on_select(f.clone());
        self.selected_widget.selected_widget = self.selected_widget.selected_widget.on_select(f);
        self
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn insert_char(&mut self, c: char) {
        self.input_widget.insert_char(c);
        self.selected_widget
            .update_filter(self.input_widget.content());
        self.selected_widget.focus(0);
    }

    pub fn remove_char(&mut self) {
        self.input_widget.remove_char();
        self.selected_widget
            .update_filter(self.input_widget.content());
        self.selected_widget.focus(0);
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

    pub fn toggle_select_unselect(&mut self) {
        self.selected_widget.toggle_select_unselect();
    }

    pub fn selected_items(&self) -> &HashSet<String> {
        self.selected_widget.selected_items()
    }

    pub fn clear_filter(&mut self) {
        self.input_widget.clear();
        self.selected_widget.update_filter("");
    }

    pub fn remove_chars_before_cursor(&mut self) {
        self.input_widget.remove_chars_before_cursor();
        self.selected_widget
            .update_filter(self.input_widget.content());
        self.selected_widget.focus(0);
    }

    pub fn remove_chars_after_cursor(&mut self) {
        self.input_widget.remove_chars_after_cursor();
        self.selected_widget
            .update_filter(self.input_widget.content());
        self.selected_widget.focus(0);
    }

    pub fn move_cursor_top(&mut self) {
        self.input_widget.move_cursor_top();
    }

    pub fn move_cursor_end(&mut self) {
        self.input_widget.move_cursor_end();
    }
}

impl WidgetTrait for MultipleSelect<'_> {
    fn focusable(&self) -> bool {
        true
    }

    fn select_next(&mut self, i: usize) {
        self.selected_widget.select_next(i);
    }

    fn select_prev(&mut self, i: usize) {
        self.selected_widget.select_prev(i);
    }

    fn select_first(&mut self) {
        self.selected_widget.select_first()
    }

    fn select_last(&mut self) {
        self.selected_widget.select_last()
    }

    fn set_items(&mut self, items: WidgetItem) {
        self.clear_filter();
        self.selected_widget.set_items(items.array());
    }

    fn append_items(&mut self, _: WidgetItem) {}

    fn get_item(&self) -> Option<WidgetItem> {
        Some(WidgetItem::Array(
            self.selected_widget.to_vec_selected_items(),
        ))
    }

    fn update_chunk(&mut self, chunk: Rect) {
        self.chunk = chunk;

        let inner_chunks = self.layout.split(default_focus_block().inner(self.chunk));

        self.input_widget
            .update_chunk(inner_chunks[LAYOUT_INDEX_FOR_INPUT_FORM]);

        self.selected_widget
            .update_chunk(inner_chunks[LAYOUT_INDEX_FOR_SELECT_FORM]);
    }

    fn clear(&mut self) {}

    fn on_mouse_event(&mut self, ev: MouseEvent) -> EventResult {
        let pos = (ev.column, ev.row);

        let chunks = self.layout.split(default_focus_block().inner(self.chunk));

        if contains(chunks[LAYOUT_INDEX_FOR_INPUT_FORM], pos) {
            self.input_widget.on_mouse_event(ev)
        } else if contains(chunks[LAYOUT_INDEX_FOR_SELECT_FORM], pos) {
            self.selected_widget.on_mouse_event(ev)
        } else {
            EventResult::Nop
        }
    }

    fn on_key_event(&mut self, ev: KeyEvent) -> EventResult {
        match self.input_widget.on_key_event(ev) {
            EventResult::Ignore => {
                if let KeyCode::Tab | KeyCode::BackTab = key_event_to_code(ev) {
                    self.toggle_focus();
                } else {
                    return self.selected_widget.on_key_event(ev);
                }
            }
            _ => {
                self.selected_widget
                    .update_filter(self.input_widget.content());
            }
        }
        EventResult::Nop
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn title(&self) -> &str {
        &self.title
    }

    fn chunk(&self) -> Rect {
        self.chunk
    }
}

#[inline]
fn is_odd(num: u16) -> bool {
    num & 1 != 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn focus_toggle() {
        let mut select = MultipleSelect::default();

        select.toggle_focus();
        assert_eq!(select.selected_widget.focus_id, 1);

        select.toggle_focus();
        assert_eq!(select.selected_widget.focus_id, 0);
    }
}
