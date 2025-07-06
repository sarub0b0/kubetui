mod filter;
mod select;

use std::rc::Rc;

use ratatui::{
    Frame,
    crossterm::event::{KeyEvent, MouseEvent},
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Paragraph},
};

use crate::{
    define_callback,
    message::UserEvent,
    ui::{
        event::{Callback, EventResult},
        util::RectContainsPoint,
        widget::{Item, RenderTrait, SelectedItem, WidgetBase, WidgetTrait},
    },
};

pub use self::filter::{FilterForm, FilterFormTheme};
pub use self::select::{SelectForm, SelectFormTheme};

const LAYOUT_INDEX_FOR_INPUT_FORM: usize = 0;
const LAYOUT_INDEX_FOR_STATUS: usize = 1;
const LAYOUT_INDEX_FOR_SELECT_FORM: usize = 2;

define_callback!(pub RenderBlockInjection, Fn(&SingleSelect, bool) -> Block<'static>);

#[derive(Debug, Default)]
pub struct SingleSelectTheme {
    status_style: Style,
}

impl SingleSelectTheme {
    pub fn status_style(mut self, status_style: impl Into<Style>) -> Self {
        self.status_style = status_style.into();
        self
    }
}

#[derive(Debug, Default)]
pub struct SingleSelectBuilder {
    id: String,
    widget_base: WidgetBase,
    select_form: SelectForm<'static>,
    filter_form: FilterForm,
    theme: SingleSelectTheme,
    actions: Vec<(UserEvent, Callback)>,
    block_injection: Option<RenderBlockInjection>,
}

#[allow(dead_code)]
impl SingleSelectBuilder {
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    pub fn widget_base(mut self, widget_base: WidgetBase) -> Self {
        self.widget_base = widget_base;
        self
    }

    pub fn select_form(mut self, select_form: SelectForm<'static>) -> Self {
        self.select_form = select_form;
        self
    }

    pub fn filter_form(mut self, filter_form: FilterForm) -> Self {
        self.filter_form = filter_form;
        self
    }

    pub fn theme(mut self, theme: SingleSelectTheme) -> Self {
        self.theme = theme;
        self
    }

    pub fn action<F, E>(mut self, ev: E, cb: F) -> Self
    where
        E: Into<UserEvent>,
        F: Into<Callback>,
    {
        self.actions.push((ev.into(), cb.into()));
        self
    }

    pub fn block_injection<F>(mut self, block_injection: F) -> Self
    where
        F: Into<RenderBlockInjection>,
    {
        self.block_injection = Some(block_injection.into());
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

        SingleSelect {
            id: self.id,
            widget_base: self.widget_base,
            layout,
            select_form: self.select_form,
            theme: self.theme,
            callbacks: self.actions,
            block_injection: self.block_injection,
            filter_form: self.filter_form,
            ..Default::default()
        }
    }
}

#[derive(Debug)]
pub struct SingleSelect<'a> {
    id: String,
    widget_base: WidgetBase,
    filter_form: FilterForm,
    select_form: SelectForm<'a>,
    theme: SingleSelectTheme,
    layout: Layout,
    chunk: Rect,
    inner_chunks: Rc<[Rect]>,
    callbacks: Vec<(UserEvent, Callback)>,
    block_injection: Option<RenderBlockInjection>,
}

impl Default for SingleSelect<'_> {
    fn default() -> Self {
        Self {
            id: Default::default(),
            widget_base: Default::default(),
            filter_form: Default::default(),
            select_form: Default::default(),
            theme: Default::default(),
            layout: Default::default(),
            chunk: Default::default(),
            inner_chunks: Rc::new([Rect::default()]),
            callbacks: Default::default(),
            block_injection: Default::default(),
        }
    }
}

// ---------------------
// |      Filter       |
// |-------------------|
// |                   |
// |      Select       |
// |                   |
// |                   |
// ---------------------
#[allow(dead_code)]
impl SingleSelect<'_> {
    pub fn builder() -> SingleSelectBuilder {
        SingleSelectBuilder::default()
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    fn render_status(&mut self, f: &mut Frame) {
        let status = self.select_form.status();
        f.render_widget(
            Paragraph::new(format!("[{}/{}]", status.0, status.1)).style(self.theme.status_style),
            self.inner_chunks[LAYOUT_INDEX_FOR_STATUS],
        );
    }

    pub fn insert_char(&mut self, c: char) {
        self.filter_form.insert_char(c);
        self.select_form.update_filter(self.filter_form.content());
    }

    pub fn remove_char(&mut self) {
        self.filter_form.remove_char();
        self.select_form.update_filter(self.filter_form.content());
    }

    pub fn remove_chars_before_cursor(&mut self) {
        self.filter_form.remove_chars_before_cursor();
        self.select_form.update_filter(self.filter_form.content());
    }

    pub fn remove_chars_after_cursor(&mut self) {
        self.filter_form.remove_chars_after_cursor();
        self.select_form.update_filter(self.filter_form.content());
    }

    pub fn forward_cursor(&mut self) {
        self.filter_form.forward_cursor();
    }

    pub fn back_cursor(&mut self) {
        self.filter_form.back_cursor();
    }

    pub fn move_cursor_top(&mut self) {
        self.filter_form.move_cursor_top();
    }

    pub fn move_cursor_end(&mut self) {
        self.filter_form.move_cursor_end();
    }

    pub fn clear_filter(&mut self) {
        self.filter_form.clear();

        self.select_form.update_filter(self.filter_form.content());
    }

    pub fn match_callback(&self, ev: UserEvent) -> Option<&Callback> {
        self.callbacks
            .iter()
            .find_map(|(cb_ev, cb)| if *cb_ev == ev { Some(cb) } else { None })
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
        self.select_form.widget_item()
    }

    fn chunk(&self) -> Rect {
        self.chunk
    }

    fn select_index(&mut self, _: usize) {
        todo!()
    }

    fn select_next(&mut self, i: usize) {
        self.select_form.select_next(i)
    }

    fn select_prev(&mut self, i: usize) {
        self.select_form.select_prev(i)
    }

    fn select_first(&mut self) {
        self.select_form.select_first()
    }

    fn select_last(&mut self) {
        self.select_form.select_last()
    }

    fn append_widget_item(&mut self, _: Item) {}

    fn update_widget_item(&mut self, items: Item) {
        self.filter_form.clear();
        self.select_form.update_filter("");
        self.select_form.update_widget_item(items);
    }

    fn on_mouse_event(&mut self, ev: MouseEvent) -> EventResult {
        let pos = (ev.column, ev.row);

        let chunks = &self.inner_chunks;

        if chunks[LAYOUT_INDEX_FOR_INPUT_FORM].contains_point(pos) {
            self.filter_form.on_mouse_event(ev)
        } else if chunks[LAYOUT_INDEX_FOR_SELECT_FORM].contains_point(pos) {
            self.select_form.on_mouse_event(ev)
        } else {
            EventResult::Nop
        }
    }

    fn on_key_event(&mut self, ev: KeyEvent) -> EventResult {
        let event_result = match self.filter_form.on_key_event(ev) {
            EventResult::Ignore => self.select_form.on_key_event(ev),
            _ => {
                self.select_form.update_filter(self.filter_form.content());

                EventResult::Nop
            }
        };

        if let EventResult::Ignore = event_result {
            if let Some(cb) = self.match_callback(UserEvent::Key(ev)) {
                return EventResult::Callback(cb.clone());
            }
        }

        event_result
    }

    fn update_chunk(&mut self, chunk: Rect) {
        self.chunk = chunk;

        let inner_chunk = self.widget_base.block().inner(chunk);

        self.inner_chunks = self.layout.split(inner_chunk);

        self.filter_form
            .update_chunk(self.inner_chunks[LAYOUT_INDEX_FOR_INPUT_FORM]);

        self.select_form
            .update_chunk(self.inner_chunks[LAYOUT_INDEX_FOR_SELECT_FORM]);
    }

    fn clear(&mut self) {
        *(self.widget_base.append_title_mut()) = None;
        unimplemented!()
    }

    fn widget_base(&self) -> &WidgetBase {
        &self.widget_base
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.widget_base
    }
}

impl RenderTrait for SingleSelect<'_> {
    fn render(&mut self, f: &mut Frame<'_>, is_active: bool, is_mouse_over: bool) {
        let block = if let Some(block_injection) = &self.block_injection {
            (block_injection)(&*self, is_active)
        } else {
            self.widget_base
                .render_block(self.can_activate() && is_active, is_mouse_over)
        };

        f.render_widget(block, self.chunk);
        self.filter_form.render(f, true, false);
        self.render_status(f);
        self.select_form.render(f);
    }
}
