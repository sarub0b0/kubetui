use crate::{
    contains,
    crossterm::event::MouseEvent,
    focus_block,
    tui::{
        backend::Backend,
        layout::{Constraint, Direction, Layout, Rect},
        widgets::{Block, Paragraph},
        Frame,
    },
    widget::*,
    EventResult,
};

use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

use super::input::InputForm;

struct SelectForm<'a> {
    list_items: Vec<String>,
    list_widget: Widget<'a>,
    filter: String,
    chunk: Rect,
    matcher: SkimMatcherV2,
}

impl Default for SelectForm<'_> {
    fn default() -> Self {
        Self {
            list_items: Vec::new(),
            filter: String::default(),
            list_widget: Widget::List(List::default()),
            chunk: Rect::default(),
            matcher: SkimMatcherV2::default(),
        }
    }
}

impl<'a> SelectForm<'a> {
    fn render<B: Backend>(&mut self, f: &mut Frame<B>) {
        self.list_widget
            .render(f, focus_block("Items", true), self.chunk);
    }

    fn select_next(&mut self) {
        self.list_widget.select_next(1);
    }

    fn select_prev(&mut self) {
        self.list_widget.select_prev(1);
    }

    fn filter_items(&self, items: &[String]) -> Vec<String> {
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
        self.chunk = chunk;
        self.list_widget
            .update_chunk(focus_block("", true).inner(chunk));
    }

    fn set_items(&mut self, items: Vec<String>) {
        self.list_items = items.clone();
        self.list_widget.set_items(WidgetItem::Array(items));

        let filter = self.filter.clone();
        self.update_filter(&filter);
    }

    fn get_item(&self) -> Option<WidgetItem> {
        self.list_widget.get_item()
    }

    fn update_filter(&mut self, filter: &str) {
        self.filter = filter.to_string();
        self.list_widget
            .set_items(WidgetItem::Array(self.filter_items(&self.list_items)));

        let list = self.list_widget.as_mut_list();
        let current_pos = list.state().selected();

        if let Some(pos) = current_pos {
            if list.items().len() <= pos {
                list.select_last()
            }
        }
    }

    fn status(&self) -> (usize, usize) {
        let list = self.list_widget.as_list();

        let mut pos = list.state().selected().unwrap_or(0);

        let size = list.items().len();

        if 0 < size {
            pos += 1;
        }

        (pos, size)
    }

    pub fn on_mouse_event(&mut self, ev: MouseEvent) -> EventResult {
        self.list_widget.on_mouse_event(ev)
    }
}

const LAYOUT_INDEX_FOR_INPUT_FORM: usize = 0;
const LAYOUT_INDEX_FOR_STATUS: usize = 1;
const LAYOUT_INDEX_FOR_SELECT_FORM: usize = 2;

pub struct SingleSelect<'a> {
    id: String,
    title: String,
    input_widget: InputForm<'a>,
    selected_widget: SelectForm<'a>,
    layout: Layout,
    block: Block<'a>,
    chunk: Rect,
}

impl<'a> SingleSelect<'a> {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
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
        self.render_status(f);
        self.selected_widget.render(f);
    }

    fn render_status<B: Backend>(&mut self, f: &mut Frame<B>) {
        let status = self.selected_widget.status();
        f.render_widget(
            Paragraph::new(format!("[{}/{}]", status.0, status.1)),
            self.layout.split(self.block.inner(self.chunk))[LAYOUT_INDEX_FOR_STATUS],
        );
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

    pub fn remove_chars_before_cursor(&mut self) {
        self.input_widget.remove_chars_before_cursor();
        self.selected_widget
            .update_filter(self.input_widget.content());
    }

    pub fn remove_chars_after_cursor(&mut self) {
        self.input_widget.remove_chars_after_cursor();
        self.selected_widget
            .update_filter(self.input_widget.content());
    }

    pub fn forward_cursor(&mut self) {
        self.input_widget.forward_cursor();
    }

    pub fn back_cursor(&mut self) {
        self.input_widget.back_cursor();
    }

    pub fn select_next_item(&mut self) {
        self.selected_widget.select_next();
    }

    pub fn select_prev_item(&mut self) {
        self.selected_widget.select_prev();
    }

    pub fn set_items(&mut self, items: Vec<String>) {
        self.input_widget.clear();
        self.selected_widget.update_filter("");
        self.selected_widget.set_items(items);
    }

    pub fn get_item(&self) -> Option<WidgetItem> {
        self.selected_widget.get_item()
    }

    pub fn move_cursor_top(&mut self) {
        self.input_widget.move_cursor_top();
    }

    pub fn move_cursor_end(&mut self) {
        self.input_widget.move_cursor_end();
    }

    pub fn on_mouse_event(&mut self, ev: MouseEvent) -> EventResult {
        let pos = (ev.column, ev.row);

        let chunks = self.layout.split(self.block.inner(self.chunk));

        if contains(chunks[LAYOUT_INDEX_FOR_INPUT_FORM], pos) {
            self.input_widget.on_mouse_event(ev)
        } else if contains(chunks[LAYOUT_INDEX_FOR_SELECT_FORM], pos) {
            self.selected_widget.on_mouse_event(ev)
        } else {
            EventResult::Nop
        }
    }
}

impl Default for SingleSelect<'_> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn filter() {
        let mut select_form = SelectForm::default();

        select_form.set_items(vec![
            "abb".to_string(),
            "abc".to_string(),
            "hoge".to_string(),
        ]);

        select_form.update_filter("ab");

        let res = select_form.list_widget.as_list().items().clone();

        assert_eq!(res, vec!["abb", "abc"])
    }
}
