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
                    .fuzzy_match(item, &self.filter)
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

    fn update_widget_item(&mut self, items: Item) {
        self.list_items = items.clone().array();
        self.list_widget.update_widget_item(items);

        let filter = self.filter.clone();
        self.update_filter(&filter);
    }

    fn widget_item(&self) -> Option<Item> {
        self.list_widget.widget_item()
    }

    fn update_filter(&mut self, filter: &str) {
        self.filter = filter.to_string();
        self.list_widget
            .update_widget_item(Item::Array(self.filter_items(&self.list_items)));

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
    pub fn builder() -> SingleSelectBuilder {
        SingleSelectBuilder::default()
    }

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
}

impl WidgetTrait for SingleSelect<'_> {
    fn id(&self) -> &str {
        &self.id
    }

    fn title(&self) -> &str {
        &self.title
    }

    fn title_mut(&mut self) -> &mut String {
        &mut self.title
    }

    fn append_title(&self) -> &Option<String> {
        unimplemented!()
    }

    fn append_title_mut(&mut self) -> &mut Option<String> {
        unimplemented!()
    }

    fn focusable(&self) -> bool {
        true
    }

    fn widget_item(&self) -> Option<Item> {
        self.selected_widget.widget_item()
    }

    fn chunk(&self) -> Rect {
        self.chunk
    }

    fn select_index(&mut self, _: usize) {
        todo!()
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

    fn append_widget_item(&mut self, _: Item) {}

    fn update_widget_item(&mut self, items: Item) {
        self.input_widget.clear();
        self.selected_widget.update_filter("");
        self.selected_widget.update_widget_item(items);
    }

    fn update_append_title(&mut self, _: impl Into<String>) {
        unimplemented!()
    }

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

    fn update_chunk(&mut self, chunk: Rect) {
        self.chunk = chunk;

        let inner_chunks = self.layout.split(default_focus_block().inner(self.chunk));

        self.input_widget
            .update_chunk(inner_chunks[LAYOUT_INDEX_FOR_INPUT_FORM]);

        self.selected_widget
            .update_chunk(inner_chunks[LAYOUT_INDEX_FOR_SELECT_FORM]);
    }

    fn clear(&mut self) {
        unimplemented!()
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

#[derive(Derivative)]
#[derivative(Debug, Default)]
pub struct SingleSelectBuilder {
    id: String,
    title: String,
    #[derivative(Debug = "ignore")]
    actions: Vec<(UserEvent, InnerCallback)>,
    #[derivative(Debug = "ignore")]
    on_select: Option<Box<dyn Fn(&mut Window, &String) -> EventResult>>,
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

    pub fn action<F, E: Into<UserEvent>>(mut self, ev: E, cb: F) -> Self
    where
        F: Fn(&mut Window) -> EventResult + 'static,
    {
        self.actions.push((ev.into(), Rc::new(cb)));
        self
    }

    pub fn on_select<F>(mut self, f: F) -> Self
    where
        F: Fn(&mut Window, &String) -> EventResult + 'static,
    {
        self.on_select = Some(Box::new(f));
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

        let list_widget = if let Some(on_select) = self.on_select {
            List::builder().on_select(on_select)
        } else {
            List::builder()
        }
        .title("Items")
        .build();

        let selected_widget = SelectForm {
            list_widget,
            ..Default::default()
        };

        SingleSelect {
            id: self.id,
            title: self.title,
            layout,
            selected_widget,
            callbacks: self.actions,
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

        select_form.update_widget_item(Item::Array(vec![
            "abb".to_string(),
            "abc".to_string(),
            "hoge".to_string(),
        ]));

        select_form.update_filter("ab");

        let res = select_form.list_widget.items().clone();

        assert_eq!(res, vec!["abb", "abc"])
    }
}
