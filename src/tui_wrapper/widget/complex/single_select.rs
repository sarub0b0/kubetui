use crossterm::event::{KeyEvent, MouseEvent};

use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::Paragraph,
    Frame,
};

use derivative::*;
use std::rc::Rc;
use tui::widgets::Block;

use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

use super::input::InputForm;

use crate::{
    event::UserEvent,
    tui_wrapper::{
        event::{EventResult, InnerCallback},
        util::contains,
        widget::*,
        Window,
    },
};

#[derive(Derivative)]
#[derivative(Debug, Default)]
struct SelectForm<'a> {
    list_items: Vec<LiteralItem>,
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

    fn filter_items(&self, items: &[LiteralItem]) -> Vec<LiteralItem> {
        let mut ret: Vec<LiteralItem> = items
            .iter()
            .filter_map(|item| {
                self.matcher
                    .fuzzy_match(&item.item, &self.filter)
                    .map(|_| item.clone())
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

    fn widget_item(&self) -> Option<SelectedItem> {
        self.list_widget.widget_item()
    }

    fn update_filter(&mut self, filter: impl Into<String>) {
        self.filter = filter.into();
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

type RenderBlockInjection = Rc<dyn Fn(&SingleSelect, bool) -> Block<'static>>;
type RenderBlockInjectionForList = Box<dyn Fn(&List, bool) -> Block<'static>>;

#[derive(Derivative)]
#[derivative(Debug, Default)]
pub struct SingleSelect<'a> {
    id: String,
    widget_config: WidgetConfig,
    chunk_index: usize,
    input_widget: InputForm<'a>,
    selected_widget: SelectForm<'a>,
    layout: Layout,
    chunk: Rect,
    inner_chunks: Vec<Rect>,
    #[derivative(Debug = "ignore")]
    callbacks: Vec<(UserEvent, InnerCallback)>,
    #[derivative(Debug = "ignore")]
    block_injection: Option<RenderBlockInjection>,
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
            self.inner_chunks[LAYOUT_INDEX_FOR_STATUS],
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

    fn focusable(&self) -> bool {
        true
    }

    fn widget_item(&self) -> Option<SelectedItem> {
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

    fn on_mouse_event(&mut self, ev: MouseEvent) -> EventResult {
        let pos = (ev.column, ev.row);

        let chunks = &self.inner_chunks;

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

        let inner_chunk = self.widget_config.block().inner(chunk);

        self.inner_chunks = self.layout.split(inner_chunk);

        self.input_widget
            .update_chunk(self.inner_chunks[LAYOUT_INDEX_FOR_INPUT_FORM]);

        self.selected_widget
            .update_chunk(self.inner_chunks[LAYOUT_INDEX_FOR_SELECT_FORM]);
    }

    fn clear(&mut self) {
        *(self.widget_config.append_title_mut()) = None;
        unimplemented!()
    }

    fn widget_config(&self) -> &WidgetConfig {
        &self.widget_config
    }

    fn widget_config_mut(&mut self) -> &mut WidgetConfig {
        &mut self.widget_config
    }
}

impl RenderTrait for SingleSelect<'_> {
    fn render<B>(&mut self, f: &mut Frame<'_, B>, selected: bool)
    where
        B: Backend,
    {
        let block = if let Some(block_injection) = &self.block_injection {
            (block_injection)(&*self, selected)
        } else {
            self.widget_config
                .render_block(self.focusable() && selected)
        };

        f.render_widget(block, self.chunk);
        self.input_widget.render(f, true);
        self.render_status(f);
        self.selected_widget.render(f);
    }
}

#[derive(Derivative)]
#[derivative(Debug, Default)]
pub struct SingleSelectBuilder {
    id: String,
    widget_config: WidgetConfig,
    #[derivative(Debug = "ignore")]
    actions: Vec<(UserEvent, InnerCallback)>,
    #[derivative(Debug = "ignore")]
    on_select: Option<Box<dyn Fn(&mut Window, &LiteralItem) -> EventResult>>,
    #[derivative(Debug = "ignore")]
    block_injection: Option<RenderBlockInjection>,
    #[derivative(Debug = "ignore")]
    block_injection_for_list: Option<RenderBlockInjectionForList>,
}

impl SingleSelectBuilder {
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    pub fn widget_config(mut self, widget_config: &WidgetConfig) -> Self {
        self.widget_config = widget_config.clone();
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
        F: Fn(&mut Window, &LiteralItem) -> EventResult + 'static,
    {
        self.on_select = Some(Box::new(f));
        self
    }

    pub fn block_injection<F>(mut self, block_injection: F) -> Self
    where
        F: Fn(&SingleSelect, bool) -> Block<'static> + 'static,
    {
        self.block_injection = Some(Rc::new(block_injection));
        self
    }

    pub fn block_injection_for_list<F>(mut self, block_injection: F) -> Self
    where
        F: Fn(&List, bool) -> Block<'static> + 'static,
    {
        self.block_injection_for_list = Some(Box::new(block_injection));
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

        let mut list_widget_builder = if let Some(on_select) = self.on_select {
            List::builder().on_select(on_select)
        } else {
            List::builder()
        }
        .widget_config(&WidgetConfig::builder().title("Items").build());

        if let Some(block_injection) = self.block_injection_for_list {
            list_widget_builder = list_widget_builder.block_injection(block_injection);
        }

        let selected_widget = SelectForm {
            list_widget: list_widget_builder.build(),
            ..Default::default()
        };

        SingleSelect {
            id: self.id,
            widget_config: self.widget_config,
            layout,
            selected_widget,
            callbacks: self.actions,
            block_injection: self.block_injection,
            input_widget: InputForm::new(WidgetConfig::builder().title("Filter").build()),
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
            "abb".to_string().into(),
            "abc".to_string().into(),
            "hoge".to_string().into(),
        ]));

        select_form.update_filter("ab");

        let res = select_form.list_widget.items().clone();

        let expected: Vec<LiteralItem> = vec!["abb".to_string().into(), "abc".to_string().into()];

        assert_eq!(res, expected)
    }
}
