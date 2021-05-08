use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::*,
    text::Span,
    widgets::{Block, Paragraph},
    Frame,
};

use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

use std::collections::HashSet;

use tui_wrapper::focus_block;
use tui_wrapper::widget::*;

use super::input::InputForm;

struct SelectForm<'a> {
    list_items: HashSet<String>,
    selected_items: HashSet<String>,
    filter: String,
    list_widget: Widget<'a>,
    selected_widget: Widget<'a>,
    chunk: Vec<Rect>,
    focus_id: usize,
    layout: Layout,
    matcher: SkimMatcherV2,
}

impl Default for SelectForm<'_> {
    fn default() -> Self {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)]);
        Self {
            list_items: HashSet::new(),
            selected_items: HashSet::new(),
            filter: String::default(),
            list_widget: Widget::List(List::default()),
            selected_widget: Widget::List(List::default()),
            chunk: Vec::new(),
            focus_id: 0,
            layout,
            matcher: SkimMatcherV2::default(),
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

        self.list_widget
            .render(f, focus_block("Items", self.focus_id == 0), ch_list);

        let w = Paragraph::new(Span::styled(
            arrow,
            Style::default().add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Center)
        .block(Block::default());

        f.render_widget(w, ch_arrow);

        self.selected_widget
            .render(f, focus_block("Selected", self.focus_id == 1), ch_selected);
    }

    fn filter_items(&self, items: &HashSet<String>) -> Vec<String> {
        let mut ret: Vec<String> = items
            .iter()
            .filter_map(|item| match self.matcher.fuzzy_match(&item, &self.filter) {
                Some(_) => Some(item.to_string()),
                None => None,
            })
            .collect();
        ret.sort();
        ret
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

    fn focus(&mut self, id: usize) {
        self.focus_id = id;
    }

    fn toggle_select_unselect(&mut self) {
        // 1. フィルタされているアイテムをフォーカスしているリストからアイテムを取り出す
        // 2. 取得したアイテムをフォーカスしているリストから削除
        // 3  フォーカスしていないリストに追加
        let list = self.focused_form_mut().as_mut_list();
        let selected_item = if let Some(index) = list.selected() {
            Some(list.items()[index].to_string())
        } else {
            None
        };

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
            .as_mut_list()
            .set_items(WidgetItem::Array(focused_item));

        self.unfocused_form_mut()
            .as_mut_list()
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

    fn set_list_items(&mut self, items: Vec<String>) {
        // マージ処理
        let new_items: HashSet<String> = items.clone().into_iter().collect();
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

        let list = self.list_widget.as_mut_list();
        let current_pos = list.state().selected();

        if let Some(pos) = current_pos {
            let list = self.list_widget.as_mut_list();
            if list.items().len() <= pos {
                list.select_last()
            }
        }
    }

    fn status(&self) -> (usize, usize) {
        let mut pos = self
            .list_widget
            .as_list()
            .state()
            .selected()
            .unwrap_or_else(|| 0);

        let size = self.list_widget.as_list().items().len();

        if 0 < size {
            pos += 1;
        }

        (pos, size)
    }

    fn to_vec_selected_items(&self) -> Vec<String> {
        self.selected_items.clone().into_iter().collect()
    }

    fn selected_items(&self) -> &HashSet<String> {
        &self.selected_items
    }
}

const LAYOUT_INDEX_FOR_INPUT_FORM: usize = 0;
const LAYOUT_INDEX_FOR_STATUS: usize = 1;
const LAYOUT_INDEX_FOR_SELECT_FORM: usize = 2;

pub struct MultipleSelect<'a> {
    id: String,
    title: String,
    input_widget: InputForm<'a>,
    selected_widget: SelectForm<'a>,
    layout: Layout,
    block: Block<'a>,
    chunk: Rect,
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

        Self {
            id: id.into(),
            title: title.into(),
            layout,
            ..Self::default()
        }
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = block;
        self
    }

    pub fn id(&self) -> &str {
        &self.id
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

        let status = self.selected_widget.status();
        f.render_widget(
            Paragraph::new(format!("[{}/{}]", status.0, status.1)),
            self.layout.split(self.block.inner(self.chunk))[LAYOUT_INDEX_FOR_STATUS],
        );
        self.selected_widget.render(f);
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

    pub fn select_next_item(&mut self) {
        self.selected_widget.select_next();
    }

    pub fn select_prev_item(&mut self) {
        self.selected_widget.select_prev();
    }

    pub fn toggle_select_unselect(&mut self) {
        self.selected_widget.toggle_select_unselect();
    }

    pub fn set_list_items(&mut self, items: Vec<String>) {
        self.clear_filter();
        self.selected_widget.set_list_items(items);
    }

    pub fn to_vec_selected_items(&self) -> Vec<String> {
        self.selected_widget.to_vec_selected_items()
    }

    pub fn selected_items(&self) -> &HashSet<String> {
        self.selected_widget.selected_items()
    }

    pub fn clear_filter(&mut self) {
        self.input_widget.clear();
        self.selected_widget.update_filter("");
    }
}

impl Default for MultipleSelect<'_> {
    fn default() -> Self {
        Self {
            id: String::default(),
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
