use crossterm::event::{KeyEvent, MouseEvent};

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Paragraph},
    Frame,
};

use derivative::*;

use std::{collections::BTreeSet, rc::Rc};

use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};

use crate::{
    message::UserEvent,
    ui::{
        event::{Callback, EventResult, InnerCallback},
        util::RectContainsPoint,
        widget::{input::InputForm, *},
        Window,
    },
};

#[derive(Derivative)]
#[derivative(Debug, Default)]
struct SelectForm<'a> {
    list_items: BTreeSet<LiteralItem>,
    list_widget: List<'a>,
    filter: String,
    chunk: Rect,
    #[derivative(Debug = "ignore")]
    matcher: SkimMatcherV2,
}

impl<'a> SelectForm<'a> {
    fn render(&mut self, f: &mut Frame) {
        self.list_widget.render(f, true, false);
    }

    fn filter_items(&self, items: &BTreeSet<LiteralItem>) -> Vec<LiteralItem> {
        struct MatchedItem {
            score: i64,
            item: LiteralItem,
        }

        let mut ret: Vec<MatchedItem> = items
            .iter()
            .filter_map(|item| {
                self.matcher
                    .fuzzy_match(&item.item.styled_graphemes_symbols().concat(), &self.filter)
                    .map(|score| MatchedItem {
                        score,
                        item: item.clone(),
                    })
            })
            .collect();

        ret.sort_by(|a, b| b.score.cmp(&a.score));

        ret.into_iter().map(|i| i.item).collect()
    }

    fn update_chunk(&mut self, chunk: Rect) {
        self.chunk = chunk;
        self.list_widget.update_chunk(chunk);
    }

    fn update_widget_item(&mut self, items: Item) {
        self.list_items = items.clone().array().into_iter().collect();

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
#[derivative(Debug)]
pub struct SingleSelect<'a> {
    id: String,
    widget_config: WidgetConfig,
    chunk_index: usize,
    input_widget: InputForm,
    select_widget: SelectForm<'a>,
    layout: Layout,
    chunk: Rect,
    inner_chunks: Rc<[Rect]>,
    #[derivative(Debug = "ignore")]
    callbacks: Vec<(UserEvent, InnerCallback)>,
    #[derivative(Debug = "ignore")]
    block_injection: Option<RenderBlockInjection>,
}

impl Default for SingleSelect<'_> {
    fn default() -> Self {
        Self {
            id: Default::default(),
            widget_config: Default::default(),
            chunk_index: Default::default(),
            input_widget: Default::default(),
            select_widget: Default::default(),
            layout: Default::default(),
            chunk: Default::default(),
            inner_chunks: Rc::new([Rect::default()]),
            callbacks: Default::default(),
            block_injection: Default::default(),
        }
    }
}

#[allow(dead_code)]
impl<'a> SingleSelect<'a> {
    pub fn builder() -> SingleSelectBuilder {
        SingleSelectBuilder::default()
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    fn render_status(&mut self, f: &mut Frame) {
        let status = self.select_widget.status();
        f.render_widget(
            Paragraph::new(format!("[{}/{}]", status.0, status.1)),
            self.inner_chunks[LAYOUT_INDEX_FOR_STATUS],
        );
    }

    pub fn insert_char(&mut self, c: char) {
        self.input_widget.insert_char(c);
        self.select_widget
            .update_filter(self.input_widget.content());
    }

    pub fn remove_char(&mut self) {
        self.input_widget.remove_char();
        self.select_widget
            .update_filter(self.input_widget.content());
    }

    pub fn remove_chars_before_cursor(&mut self) {
        self.input_widget.remove_chars_before_cursor();
        self.select_widget
            .update_filter(self.input_widget.content());
    }

    pub fn remove_chars_after_cursor(&mut self) {
        self.input_widget.remove_chars_after_cursor();
        self.select_widget
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

    pub fn clear_filter(&mut self) {
        self.input_widget.clear();

        self.select_widget
            .update_filter(self.input_widget.content());
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

    fn can_activate(&self) -> bool {
        true
    }

    fn widget_item(&self) -> Option<SelectedItem> {
        self.select_widget.widget_item()
    }

    fn chunk(&self) -> Rect {
        self.chunk
    }

    fn select_index(&mut self, _: usize) {
        todo!()
    }

    fn select_next(&mut self, i: usize) {
        self.select_widget.list_widget.select_next(i)
    }

    fn select_prev(&mut self, i: usize) {
        self.select_widget.list_widget.select_prev(i)
    }

    fn select_first(&mut self) {
        self.select_widget.list_widget.select_first()
    }

    fn select_last(&mut self) {
        self.select_widget.list_widget.select_last()
    }

    fn append_widget_item(&mut self, _: Item) {}

    fn update_widget_item(&mut self, items: Item) {
        self.input_widget.clear();
        self.select_widget.update_filter("");
        self.select_widget.update_widget_item(items);
    }

    fn on_mouse_event(&mut self, ev: MouseEvent) -> EventResult {
        let pos = (ev.column, ev.row);

        let chunks = &self.inner_chunks;

        if chunks[LAYOUT_INDEX_FOR_INPUT_FORM].contains_point(pos) {
            self.input_widget.on_mouse_event(ev)
        } else if chunks[LAYOUT_INDEX_FOR_SELECT_FORM].contains_point(pos) {
            self.select_widget.on_mouse_event(ev)
        } else {
            EventResult::Nop
        }
    }

    fn on_key_event(&mut self, ev: KeyEvent) -> EventResult {
        let event_result = match self.input_widget.on_key_event(ev) {
            EventResult::Ignore => self.select_widget.on_key_event(ev),
            _ => {
                self.select_widget
                    .update_filter(self.input_widget.content());

                EventResult::Nop
            }
        };

        if let EventResult::Ignore = event_result {
            if let Some(cb) = self.match_callback(UserEvent::Key(ev)) {
                return EventResult::Callback(Callback::from(cb));
            }
        }

        event_result
    }

    fn update_chunk(&mut self, chunk: Rect) {
        self.chunk = chunk;

        let inner_chunk = self.widget_config.block().inner(chunk);

        self.inner_chunks = self.layout.split(inner_chunk);

        self.input_widget
            .update_chunk(self.inner_chunks[LAYOUT_INDEX_FOR_INPUT_FORM]);

        self.select_widget
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
    fn render(&mut self, f: &mut Frame<'_>, is_active: bool, is_mouse_over: bool) {
        let block = if let Some(block_injection) = &self.block_injection {
            (block_injection)(&*self, is_active)
        } else {
            self.widget_config
                .render_block(self.can_activate() && is_active, is_mouse_over)
        };

        f.render_widget(block, self.chunk);
        self.input_widget.render(f, true, false);
        self.render_status(f);
        self.select_widget.render(f);
    }
}

type OnSelectCallback = Box<dyn Fn(&mut Window, &LiteralItem) -> EventResult>;

#[derive(Derivative)]
#[derivative(Debug, Default)]
pub struct SingleSelectBuilder {
    id: String,
    widget_config: WidgetConfig,
    #[derivative(Debug = "ignore")]
    actions: Vec<(UserEvent, InnerCallback)>,
    #[derivative(Debug = "ignore")]
    on_select: Option<OnSelectCallback>,
    #[derivative(Debug = "ignore")]
    block_injection: Option<RenderBlockInjection>,
    #[derivative(Debug = "ignore")]
    block_injection_for_list: Option<RenderBlockInjectionForList>,
}

#[allow(dead_code)]
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

        let select_widget = SelectForm {
            list_widget: list_widget_builder.build(),
            ..Default::default()
        };

        SingleSelect {
            id: self.id,
            widget_config: self.widget_config,
            layout,
            select_widget,
            callbacks: self.actions,
            block_injection: self.block_injection,
            input_widget: InputForm::builder()
                .widget_config(WidgetConfig::builder().title("Filter").build())
                .build(),
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
            "\x1b[90mabb\x1b[39m".to_string().into(),
            "abc".to_string().into(),
            "hoge".to_string().into(),
        ]));

        select_form.update_filter("ab");

        let res = select_form.list_widget.items().clone();

        let expected: Vec<LiteralItem> = vec![
            "\x1b[90mabb\x1b[39m".to_string().into(),
            "abc".to_string().into(),
        ];

        assert_eq!(res, expected)
    }
}
