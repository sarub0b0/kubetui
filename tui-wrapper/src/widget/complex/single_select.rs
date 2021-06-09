use crate::{
    crossterm::event::{KeyEvent, MouseEvent},
    event::EventResult,
    tui::{
        backend::Backend,
        layout::{Constraint, Direction, Layout, Rect},
        text::Span,
        widgets::Paragraph,
        Frame,
    },
    util::{contains, default_focus_block, focus_block, focus_title_style},
    widget::*,
};

use derivative::*;
use std::rc::Rc;

use event::UserEvent;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

use super::input::InputForm;
use crate::event::InnerCallback;
use crate::Window;

#[derive(Derivative)]
#[derivative(Debug, Default)]
struct SelectForm<'a> {
    list_items: Vec<String>,
    list_widget: List<'a>,
    filter: String,
    chunk: Rect,
    #[derivative(Debug = "ignore")]
    matcher: SkimMatcherV2,
}

impl<'a> SelectForm<'a> {
    fn render<B: Backend>(&mut self, f: &mut Frame<B>) {
        self.list_widget.render(f, true);
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
        self.list_widget.update_chunk(chunk);
    }

    fn update_widget_item(&mut self, items: WidgetItem) {
        self.list_items = items.clone().array();
        self.list_widget.update_widget_item(items);

        let filter = self.filter.clone();
        self.update_filter(&filter);
    }

    fn widget_item(&self) -> Option<WidgetItem> {
        self.list_widget.widget_item()
    }

    fn update_filter(&mut self, filter: &str) {
        self.filter = filter.to_string();
        self.list_widget
            .update_widget_item(WidgetItem::Array(self.filter_items(&self.list_items)));

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

    fn on_mouse_event(&mut self, ev: MouseEvent) -> EventResult {
        self.list_widget.on_mouse_event(ev)
    }

    fn on_key_event(&mut self, ev: KeyEvent) -> EventResult {
        self.list_widget.on_key_event(ev)
    }
}

const LAYOUT_INDEX_FOR_INPUT_FORM: usize = 0;
const LAYOUT_INDEX_FOR_STATUS: usize = 1;
const LAYOUT_INDEX_FOR_SELECT_FORM: usize = 2;

#[derive(Derivative)]
#[derivative(Debug, Default)]
pub struct SingleSelect<'a> {
    id: String,
    title: String,
    chunk_index: usize,
    input_widget: InputForm<'a>,
    selected_widget: SelectForm<'a>,
    layout: Layout,
    chunk: Rect,
    #[derivative(Debug = "ignore")]
    callbacks: Vec<(UserEvent, InnerCallback)>,
}

impl<'a> SingleSelect<'a> {
    pub fn id(&self) -> &str {
        &self.id
    }

    fn render_status<B: Backend>(&mut self, f: &mut Frame<B>) {
        let status = self.selected_widget.status();
        f.render_widget(
            Paragraph::new(format!("[{}/{}]", status.0, status.1)),
            self.layout.split(default_focus_block().inner(self.chunk))[LAYOUT_INDEX_FOR_STATUS],
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

    pub fn move_cursor_top(&mut self) {
        self.input_widget.move_cursor_top();
    }

    pub fn move_cursor_end(&mut self) {
        self.input_widget.move_cursor_end();
    }

    pub fn match_callback(&self, ev: UserEvent) -> Option<InnerCallback> {
        self.callbacks
            .iter()
            .find_map(|(cb_ev, cb)| if *cb_ev == ev { Some(cb.clone()) } else { None })
    }

    pub fn add_action<F, E: Into<UserEvent>>(&mut self, ev: E, cb: F)
    where
        F: Fn(&mut Window) -> EventResult + 'static,
    {
        self.callbacks.push((ev.into(), Rc::new(cb)));
    }

    pub fn on_select<F>(mut self, f: F) -> Self
    where
        F: Fn(&mut Window, &String) -> EventResult + 'static,
    {
        self.selected_widget.list_widget = self.selected_widget.list_widget.on_select(f);
        self
    }
}

impl WidgetTrait for SingleSelect<'_> {
    fn focusable(&self) -> bool {
        true
    }

    fn select_next(&mut self, i: usize) {
        self.selected_widget.list_widget.select_next(i)
    }

    fn select_prev(&mut self, i: usize) {
        self.selected_widget.list_widget.select_prev(i)
    }

    fn select_first(&mut self) {
        self.selected_widget.list_widget.select_first()
    }

    fn select_last(&mut self) {
        self.selected_widget.list_widget.select_last()
    }

    fn update_widget_item(&mut self, items: WidgetItem) {
        self.input_widget.clear();
        self.selected_widget.update_filter("");
        self.selected_widget.update_widget_item(items);
    }

    fn append_widget_item(&mut self, _: WidgetItem) {}

    fn widget_item(&self) -> Option<WidgetItem> {
        self.selected_widget.widget_item()
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
                return self.selected_widget.on_key_event(ev);
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

impl RenderTrait for SingleSelect<'_> {
    fn render<B>(&mut self, f: &mut Frame<'_, B>, selected: bool)
    where
        B: Backend,
    {
        f.render_widget(
            focus_block("", selected).title(Span::styled(
                format!(" {} ", self.title()),
                focus_title_style(selected),
            )),
            self.chunk,
        );
        self.input_widget.render(f);
        self.render_status(f);
        self.selected_widget.render(f);
    }
}

#[derive(Debug, Default)]
pub struct SingleSelectBuilder {
    id: String,
    title: String,
}

impl SingleSelectBuilder {
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn build(self) -> SingleSelect<'static> {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(1),
                Constraint::Min(3),
            ]);

        let mut selected_widget = SelectForm::default();
        selected_widget.list_widget = ListBuilder::default().title("Items").build();

        SingleSelect {
            id: self.id,
            title: self.title,
            layout,
            selected_widget,
            ..Default::default()
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

        select_form.update_widget_item(WidgetItem::Array(vec![
            "abb".to_string(),
            "abc".to_string(),
            "hoge".to_string(),
        ]));

        select_form.update_filter("ab");

        let res = select_form.list_widget.items().clone();

        assert_eq!(res, vec!["abb", "abc"])
    }
}
