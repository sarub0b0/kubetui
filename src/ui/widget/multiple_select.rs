mod filter;
mod item;
mod select;

use std::rc::Rc;

use ratatui::{
    crossterm::event::{KeyCode, KeyEvent, MouseEvent},
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Paragraph},
    Frame,
};

use crate::{
    define_callback,
    ui::{
        event::EventResult,
        util::{key_event_to_code, RectContainsPoint},
        widget::{Item, LiteralItem, RenderTrait, SelectedItem, WidgetBase, WidgetTrait},
    },
};

pub use filter::{FilterForm, FilterFormTheme};
use item::SelectItems;
pub use select::{SelectForm, SelectFormTheme};

define_callback!(pub RenderBlockInjection, Fn(&MultipleSelect, bool) -> Block<'static>);

#[derive(Debug, Default)]
pub struct MultipleSelectTheme {
    status_style: Style,
}

impl MultipleSelectTheme {
    pub fn status_style(mut self, status_style: impl Into<Style>) -> Self {
        self.status_style = status_style.into();
        self
    }
}

#[derive(Debug, Default)]
pub struct MultipleSelectBuilder {
    id: String,
    widget_base: WidgetBase,
    select_form: SelectForm<'static>,
    filter_form: FilterForm,
    theme: MultipleSelectTheme,
    block_injection: Option<RenderBlockInjection>,
}

#[allow(dead_code)]
impl MultipleSelectBuilder {
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

    pub fn theme(mut self, theme: impl Into<MultipleSelectTheme>) -> Self {
        self.theme = theme.into();
        self
    }

    pub fn block_injection<F>(mut self, block_injection: F) -> Self
    where
        F: Into<RenderBlockInjection>,
    {
        self.block_injection = Some(block_injection.into());
        self
    }

    pub fn build(self) -> MultipleSelect<'static> {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(1),
                Constraint::Min(3),
            ]);

        MultipleSelect {
            id: self.id,
            widget_base: self.widget_base,
            filter_form: self.filter_form,
            select_form: self.select_form,
            theme: self.theme,
            layout,
            block_injection: self.block_injection,
            inner_chunks: Rc::new([]),
            ..Default::default()
        }
    }
}

const LAYOUT_INDEX_FOR_INPUT_FORM: usize = 0;
const LAYOUT_INDEX_FOR_STATUS: usize = 1;
const LAYOUT_INDEX_FOR_SELECT_FORM: usize = 2;

#[derive(Debug)]
pub struct MultipleSelect<'a> {
    id: String,
    widget_base: WidgetBase,
    filter_form: FilterForm,
    select_form: SelectForm<'a>,
    theme: MultipleSelectTheme,
    layout: Layout,
    chunk: Rect,
    inner_chunks: Rc<[Rect]>,
    block_injection: Option<RenderBlockInjection>,
}

impl Default for MultipleSelect<'_> {
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
            block_injection: Default::default(),
        }
    }
}

impl RenderTrait for MultipleSelect<'_> {
    fn render(&mut self, f: &mut Frame, is_active: bool, is_mouse_over: bool) {
        let block = if let Some(block_injection) = &self.block_injection {
            (block_injection)(&*self, is_active)
        } else {
            self.widget_base
                .render_block(self.can_activate() && is_active, is_mouse_over)
        };

        let inner_chunk = block.inner(self.chunk);

        f.render_widget(block, self.chunk);

        self.filter_form.render(f, true, false);

        let status = self.select_form.status();
        f.render_widget(
            Paragraph::new(format!("[{}/{}]", status.0, status.1)).style(self.theme.status_style),
            self.layout.split(inner_chunk)[LAYOUT_INDEX_FOR_STATUS],
        );
        self.select_form.render(f);
    }
}

// split [InputForm, SelectForms]
// ---------------------
// |     InputForm     |
// |-------------------|
// |         |         |
// | Select  | Select  |
// |         |         |
// |         |         |
// ---------------------
impl MultipleSelect<'_> {
    pub fn builder() -> MultipleSelectBuilder {
        MultipleSelectBuilder::default()
    }

    fn clear_filter(&mut self) {
        self.filter_form.clear();
        self.select_form.update_filter("");
    }

    pub fn selected_items(&self) -> Vec<LiteralItem> {
        self.select_form.selected_items()
    }

    pub fn select_item(&mut self, item: &LiteralItem) {
        self.select_form.select_item(item);
    }

    pub fn toggle_select_unselect(&mut self) {
        self.select_form.toggle_select_unselect();
    }

    pub fn unselect_all(&mut self) {
        self.select_form.unselect_all();
    }

    pub fn select_all(&mut self) {
        self.select_form.select_all();
    }

    pub fn clear_mouse_over(&mut self) {
        self.select_form.clear_mouse_over();
    }
}

impl WidgetTrait for MultipleSelect<'_> {
    fn id(&self) -> &str {
        &self.id
    }

    fn can_activate(&self) -> bool {
        true
    }

    fn widget_item(&self) -> Option<SelectedItem> {
        Some(self.select_form.selected_items().into())
    }

    fn chunk(&self) -> Rect {
        self.chunk
    }

    fn select_index(&mut self, _: usize) {
        unimplemented!()
    }

    fn select_next(&mut self, i: usize) {
        self.select_form.select_next(i);
    }

    fn select_prev(&mut self, i: usize) {
        self.select_form.select_prev(i);
    }

    fn select_first(&mut self) {
        self.select_form.select_first()
    }

    fn select_last(&mut self) {
        self.select_form.select_last()
    }

    fn append_widget_item(&mut self, _: Item) {
        unimplemented!()
    }

    fn update_widget_item(&mut self, items: Item) {
        self.clear_filter();
        self.select_form.update_widget_item(items);
    }

    fn on_mouse_event(&mut self, ev: MouseEvent) -> EventResult {
        let pos = (ev.column, ev.row);

        let chunks = &self.inner_chunks;

        let ret = if chunks[LAYOUT_INDEX_FOR_INPUT_FORM].contains_point(pos) {
            self.filter_form.on_mouse_event(ev)
        } else if chunks[LAYOUT_INDEX_FOR_SELECT_FORM].contains_point(pos) {
            self.select_form.on_mouse_event(ev)
        } else {
            EventResult::Nop
        };

        if let EventResult::Callback(_) = &ret {
            self.toggle_select_unselect();
        }

        ret
    }

    fn on_key_event(&mut self, ev: KeyEvent) -> EventResult {
        match self.filter_form.on_key_event(ev) {
            EventResult::Ignore => match key_event_to_code(ev) {
                KeyCode::Tab | KeyCode::BackTab => {
                    self.select_form.toggle_active_form();
                    EventResult::Nop
                }
                KeyCode::Enter => {
                    let ret = self.select_form.on_key_event(KeyCode::Enter.into());
                    self.toggle_select_unselect();
                    ret
                }
                _ => self.select_form.on_key_event(ev),
            },
            _ => {
                self.select_form.activate_form_by_index(0);
                self.select_form.update_filter(self.filter_form.content());
                EventResult::Nop
            }
        }
    }

    fn update_chunk(&mut self, chunk: Rect) {
        self.chunk = chunk;

        self.inner_chunks = self.layout.split(self.widget_base.block().inner(chunk));

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
