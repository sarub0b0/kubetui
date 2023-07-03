use super::input::InputForm;

use crate::ui::{
    event::EventResult,
    util::{key_event_to_code, MousePosition, RectContainsPoint},
    widget::*,
    Window,
};

use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};

use derivative::*;

use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};

use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::*,
    text::Span,
    widgets::{Block, Paragraph},
    Frame,
};

use std::rc::Rc;

use unicode_width::UnicodeWidthStr;

mod inner {
    use std::collections::BTreeMap;

    use crate::ui::widget::LiteralItem;

    #[derive(Debug, Default)]
    pub struct SelectItems {
        items: BTreeMap<LiteralItem, bool>,
    }

    impl SelectItems {
        pub fn update_items<T>(&mut self, items: impl Into<Vec<T>>)
        where
            T: Into<LiteralItem>,
        {
            let old = self.items.clone();

            self.items = items
                .into()
                .into_iter()
                .map(|i| (i.into(), false))
                .collect();

            old.iter().for_each(|(k, v)| {
                if let Some(value) = self.items.get_mut(k) {
                    *value = *v;
                }
            })
        }

        pub fn toggle_select_unselect(&mut self, key: &LiteralItem) {
            if let Some(value) = self.items.get_mut(key) {
                *value = !*value;
            }
        }

        #[allow(dead_code)]
        pub fn items(&self) -> Vec<&LiteralItem> {
            self.items.keys().collect()
        }

        pub fn selected_items(&self) -> Vec<LiteralItem> {
            Self::filter_items(&self.items, true)
        }

        pub fn unselected_items(&self) -> Vec<LiteralItem> {
            Self::filter_items(&self.items, false)
        }

        pub fn select_all(&mut self) {
            self.items.values_mut().for_each(|v| *v = true);
        }

        pub fn unselect_all(&mut self) {
            self.items.values_mut().for_each(|v| *v = false);
        }

        fn filter_items(items: &BTreeMap<LiteralItem, bool>, is_active: bool) -> Vec<LiteralItem> {
            items
                .iter()
                .filter_map(|(k, v)| {
                    if *v == is_active {
                        Some(k.clone())
                    } else {
                        None
                    }
                })
                .collect()
        }

        #[allow(dead_code)]
        pub fn select(&mut self, key: &LiteralItem) {
            if let Some(value) = self.items.get_mut(key) {
                *value = true;
            }
        }

        #[allow(dead_code)]
        pub fn unselect(&mut self, key: &LiteralItem) {
            if let Some(value) = self.items.get_mut(key) {
                *value = false;
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn select_unselect_and_selected_items() {
            let mut items = SelectItems::default();

            items.update_items([
                "Item 0".to_string(),
                "Item 1".to_string(),
                "Item 2".to_string(),
                "Item 3".to_string(),
                "Item 4".to_string(),
                "Item 5".to_string(),
            ]);

            items.select(&"Item 2".to_string().into());
            items.select(&"Item 5".to_string().into());
            items.select(&"Item 4".to_string().into());

            let expected: Vec<LiteralItem> = vec![
                "Item 2".to_string().into(),
                "Item 4".to_string().into(),
                "Item 5".to_string().into(),
            ];

            assert_eq!(items.selected_items(), expected);

            items.unselect(&"Item 2".to_string().into());

            let expected: Vec<LiteralItem> =
                vec!["Item 4".to_string().into(), "Item 5".to_string().into()];

            assert_eq!(items.selected_items(), expected);
        }

        #[test]
        fn update_items() {
            let mut items = SelectItems::default();

            items.update_items([
                "Item 0".to_string(),
                "Item 1".to_string(),
                "Item 2".to_string(),
                "Item 3".to_string(),
                "Item 4".to_string(),
                "Item 5".to_string(),
            ]);

            items.select(&"Item 2".to_string().into());
            items.select(&"Item 5".to_string().into());
            items.select(&"Item 4".to_string().into());

            let expected: Vec<LiteralItem> = vec![
                "Item 2".to_string().into(),
                "Item 4".to_string().into(),
                "Item 5".to_string().into(),
            ];

            assert_eq!(items.selected_items(), expected);

            items.update_items([
                "Item 0".to_string(),
                "Item 1".to_string(),
                "Item 2".to_string(),
            ]);

            let expected: Vec<LiteralItem> = vec!["Item 2".to_string().into()];

            assert_eq!(items.selected_items(), expected);
        }
    }
}

use inner::SelectItems;

const LIST_FORM_ID: usize = 0;
const SELECTED_FORM_ID: usize = 1;

#[derive(Derivative)]
#[derivative(Debug)]
struct SelectForm<'a> {
    items: SelectItems,
    filter: String,
    list_widget: List<'a>,
    selected_widget: List<'a>,
    chunk: Rect,
    active_form_index: usize,
    mouse_over_widget_index: Option<usize>,
    direction: Direction,
    #[derivative(Debug = "ignore")]
    matcher: SkimMatcherV2,
}

impl Default for SelectForm<'_> {
    fn default() -> Self {
        Self {
            items: SelectItems::default(),
            filter: String::default(),
            list_widget: List::default(),
            selected_widget: List::default(),
            chunk: Rect::default(),
            active_form_index: 0,
            mouse_over_widget_index: None,
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
                    (self.chunk.width / 2).saturating_sub(1),
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
                let right_chunk = Rect::new(cx, center_chunk.y + 1, cw, ch.saturating_sub(margin));

                ([left_chunk, center_chunk, right_chunk], "↓".to_string())
            }
        }
    }

    fn render<B: Backend>(&mut self, f: &mut Frame<B>) {
        let (chunks, arrow) = self.chunks_and_arrow();

        let arrow = Paragraph::new(Span::styled(
            arrow,
            Style::default().add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Center)
        .block(Block::default());

        self.list_widget.render(
            f,
            self.active_form_index == LIST_FORM_ID,
            self.mouse_over_widget_index == Some(LIST_FORM_ID),
        );

        f.render_widget(arrow, chunks[1]);

        self.selected_widget.render(
            f,
            self.active_form_index == 1,
            self.mouse_over_widget_index == Some(SELECTED_FORM_ID),
        );
    }

    fn update_layout(&mut self, chunk: Rect) {
        // 等幅フォントのとき 幅:高さ = 1:2
        if chunk.width < chunk.height * 4 {
            self.direction = Direction::Vertical;
        } else {
            self.direction = Direction::Horizontal;
        };
    }

    fn update_chunk(&mut self, chunk: Rect) {
        self.update_layout(chunk);

        self.chunk = chunk;

        let (chunks, _) = self.chunks_and_arrow();

        self.list_widget.update_chunk(chunks[0]);
        self.selected_widget.update_chunk(chunks[2]);
    }

    fn select_next(&mut self, i: usize) {
        self.active_form_mut().select_next(i);
    }

    fn select_prev(&mut self, i: usize) {
        self.active_form_mut().select_prev(i);
    }

    fn select_first(&mut self) {
        self.active_form_mut().select_first();
    }

    fn select_last(&mut self) {
        self.active_form_mut().select_last();
    }

    fn filter_items(&self, items: &[LiteralItem]) -> Vec<LiteralItem> {
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

    fn active_form(&mut self) -> &List<'a> {
        if self.active_form_index == LIST_FORM_ID {
            &self.list_widget
        } else {
            &self.selected_widget
        }
    }

    fn active_form_mut(&mut self) -> &mut List<'a> {
        if self.active_form_index == LIST_FORM_ID {
            &mut self.list_widget
        } else {
            &mut self.selected_widget
        }
    }

    #[allow(dead_code)]
    fn inactive_form_mut(&mut self) -> &mut List<'a> {
        if self.active_form_index == LIST_FORM_ID {
            &mut self.selected_widget
        } else {
            &mut self.list_widget
        }
    }

    fn toggle_active_form(&mut self) {
        self.clear_mouse_over();

        if self.active_form_index == LIST_FORM_ID {
            self.active_form_index = SELECTED_FORM_ID
        } else {
            self.active_form_index = LIST_FORM_ID
        }
    }

    fn activate_form_by_index(&mut self, index: usize) {
        self.clear_mouse_over();

        self.active_form_index = index;
    }

    fn update_widget_item(&mut self, items: Item) {
        self.items.update_items(items.as_array());

        self.update_widgets();
    }

    fn update_widgets(&mut self) {
        self.list_widget.update_widget_item(Item::Array(
            self.filter_items(&self.items.unselected_items()),
        ));
        self.selected_widget
            .update_widget_item(Item::Array(self.items.selected_items()));
    }

    fn toggle_select_unselect(&mut self) {
        let list = self.active_form();
        let selected_key = list.state().selected().map(|i| list.items()[i].clone());

        if let Some(key) = selected_key {
            self.items.toggle_select_unselect(&key);
            self.update_widgets();
        }
    }

    fn update_filter(&mut self, filter: impl Into<String>) {
        self.filter = filter.into();

        self.list_widget.update_widget_item(Item::Array(
            self.filter_items(&self.items.unselected_items()),
        ));

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

    fn selected_items(&self) -> Vec<LiteralItem> {
        self.items.selected_items()
    }

    fn select_item(&mut self, item: &LiteralItem) {
        if let Some((i, _)) = self
            .list_widget
            .items()
            .iter()
            .enumerate()
            .find(|(_, i)| item == *i)
        {
            self.list_widget.select_index(i);
            self.toggle_select_unselect();
            self.list_widget.select_first();
        }
    }

    fn select_all(&mut self) {
        self.items.select_all();
        self.update_widgets();
    }

    fn unselect_all(&mut self) {
        self.items.unselect_all();
        self.update_widgets();
    }

    fn on_mouse_event(&mut self, ev: MouseEvent) -> EventResult {
        let pos = ev.position();

        let (chunks, _) = self.chunks_and_arrow();

        if chunks[0].contains_point(pos) {
            match ev.kind {
                MouseEventKind::Down(MouseButton::Left) => {
                    if self.active_form_index != LIST_FORM_ID {
                        self.activate_form_by_index(LIST_FORM_ID);
                    }
                }
                MouseEventKind::Moved => {
                    self.mouse_over_widget_index = Some(LIST_FORM_ID);
                }
                _ => {}
            }

            self.active_form_mut().on_mouse_event(ev)
        } else if chunks[2].contains_point(pos) {
            match ev.kind {
                MouseEventKind::Down(MouseButton::Left) => {
                    if self.active_form_index != SELECTED_FORM_ID {
                        self.activate_form_by_index(SELECTED_FORM_ID);
                    }
                }
                MouseEventKind::Moved => {
                    self.mouse_over_widget_index = Some(SELECTED_FORM_ID);
                }
                _ => {}
            }

            self.active_form_mut().on_mouse_event(ev)
        } else {
            EventResult::Nop
        }
    }

    fn on_key_event(&mut self, ev: KeyEvent) -> EventResult {
        self.clear_mouse_over();

        self.active_form_mut().on_key_event(ev)
    }

    fn clear_mouse_over(&mut self) {
        self.mouse_over_widget_index = None;
    }
}

type RenderBlockInjection = Rc<dyn Fn(&MultipleSelect, bool) -> Block<'static>>;
type RenderBlockInjectionForList = Box<dyn Fn(&List, bool) -> Block<'static>>;
type OnSelectCallback = Box<dyn Fn(&mut Window, &LiteralItem) -> EventResult>;

#[derive(Derivative)]
#[derivative(Debug, Default)]
pub struct MultipleSelectBuilder {
    id: String,
    widget_config: WidgetConfig,
    #[derivative(Debug = "ignore")]
    on_select_list: Option<OnSelectCallback>,
    #[derivative(Debug = "ignore")]
    on_select_selected: Option<OnSelectCallback>,
    #[derivative(Debug = "ignore")]
    block_injection: Option<RenderBlockInjection>,
    #[derivative(Debug = "ignore")]
    block_injection_for_list: Option<RenderBlockInjectionForList>,
    #[derivative(Debug = "ignore")]
    block_injection_for_selected: Option<RenderBlockInjectionForList>,
}

impl MultipleSelectBuilder {
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    pub fn widget_config(mut self, widget_config: &WidgetConfig) -> Self {
        self.widget_config = widget_config.clone();
        self
    }

    pub fn on_select<F>(mut self, cb: F) -> Self
    where
        F: Fn(&mut Window, &LiteralItem) -> EventResult + 'static,
        F: Clone,
    {
        self.on_select_list = Some(Box::new(cb.clone()));
        self.on_select_selected = Some(Box::new(cb));
        self
    }

    pub fn block_injection<F>(mut self, block_injection: F) -> Self
    where
        F: Fn(&MultipleSelect, bool) -> Block<'static> + 'static,
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

    pub fn block_injection_for_selected<F>(mut self, block_injection: F) -> Self
    where
        F: Fn(&List, bool) -> Block<'static> + 'static,
    {
        self.block_injection_for_selected = Some(Box::new(block_injection));
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

        let mut list_widget_builder = if let Some(on_select) = self.on_select_list {
            List::builder().on_select(on_select)
        } else {
            List::builder()
        }
        .widget_config(&WidgetConfig::builder().title("Items").build());

        if let Some(block_injection) = self.block_injection_for_list {
            list_widget_builder = list_widget_builder.block_injection(block_injection);
        }

        let mut selected_widget_builder = if let Some(on_select) = self.on_select_selected {
            List::builder().on_select(on_select)
        } else {
            List::builder()
        }
        .widget_config(&WidgetConfig::builder().title("Selected").build());

        if let Some(block_injection) = self.block_injection_for_selected {
            selected_widget_builder = selected_widget_builder.block_injection(block_injection);
        }

        let selected_widget = SelectForm {
            list_widget: list_widget_builder.build(),
            selected_widget: selected_widget_builder.build(),
            ..Default::default()
        };

        MultipleSelect {
            id: self.id,
            widget_config: self.widget_config,
            layout,
            selected_widget,
            block_injection: self.block_injection,
            input_widget: InputForm::new(WidgetConfig::builder().title("Filter").build()),
            inner_chunks: Rc::new([]),
            ..Default::default()
        }
    }
}

const LAYOUT_INDEX_FOR_INPUT_FORM: usize = 0;
const LAYOUT_INDEX_FOR_STATUS: usize = 1;
const LAYOUT_INDEX_FOR_SELECT_FORM: usize = 2;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct MultipleSelect<'a> {
    id: String,
    widget_config: WidgetConfig,
    chunk_index: usize,
    input_widget: InputForm,
    selected_widget: SelectForm<'a>,
    layout: Layout,
    chunk: Rect,
    inner_chunks: Rc<[Rect]>,
    #[derivative(Debug = "ignore")]
    block_injection: Option<RenderBlockInjection>,
}

impl Default for MultipleSelect<'_> {
    fn default() -> Self {
        Self {
            id: Default::default(),
            widget_config: Default::default(),
            chunk_index: Default::default(),
            input_widget: Default::default(),
            selected_widget: Default::default(),
            layout: Default::default(),
            chunk: Default::default(),
            inner_chunks: Rc::new([Rect::default()]),
            block_injection: Default::default(),
        }
    }
}

impl RenderTrait for MultipleSelect<'_> {
    fn render<B: Backend>(&mut self, f: &mut Frame<B>, is_active: bool, is_mouse_over: bool) {
        let block = if let Some(block_injection) = &self.block_injection {
            (block_injection)(&*self, is_active)
        } else {
            self.widget_config
                .render_block(self.can_activate() && is_active, is_mouse_over)
        };

        let inner_chunk = block.inner(self.chunk);

        f.render_widget(block, self.chunk);

        self.input_widget.render(f, true);

        let status = self.selected_widget.status();
        f.render_widget(
            Paragraph::new(format!("[{}/{}]", status.0, status.1)),
            self.layout.split(inner_chunk)[LAYOUT_INDEX_FOR_STATUS],
        );
        self.selected_widget.render(f);
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
impl<'a> MultipleSelect<'a> {
    pub fn builder() -> MultipleSelectBuilder {
        MultipleSelectBuilder::default()
    }

    fn clear_filter(&mut self) {
        self.input_widget.clear();
        self.selected_widget.update_filter("");
    }

    pub fn selected_items(&self) -> Vec<LiteralItem> {
        self.selected_widget.selected_items()
    }

    pub fn select_item(&mut self, item: &LiteralItem) {
        self.selected_widget.select_item(item);
    }

    pub fn toggle_select_unselect(&mut self) {
        self.selected_widget.toggle_select_unselect();
    }

    pub fn unselect_all(&mut self) {
        self.selected_widget.unselect_all();
    }

    pub fn select_all(&mut self) {
        self.selected_widget.select_all();
    }

    pub fn clear_mouse_over(&mut self) {
        self.selected_widget.clear_mouse_over();
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
        Some(self.selected_widget.selected_items().into())
    }

    fn chunk(&self) -> Rect {
        self.chunk
    }

    fn select_index(&mut self, _: usize) {
        unimplemented!()
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

    fn append_widget_item(&mut self, _: Item) {
        unimplemented!()
    }

    fn update_widget_item(&mut self, items: Item) {
        self.clear_filter();
        self.selected_widget.update_widget_item(items);
    }

    fn on_mouse_event(&mut self, ev: MouseEvent) -> EventResult {
        let pos = (ev.column, ev.row);

        let chunks = &self.inner_chunks;

        let ret = if chunks[LAYOUT_INDEX_FOR_INPUT_FORM].contains_point(pos) {
            self.input_widget.on_mouse_event(ev)
        } else if chunks[LAYOUT_INDEX_FOR_SELECT_FORM].contains_point(pos) {
            self.selected_widget.on_mouse_event(ev)
        } else {
            EventResult::Nop
        };

        if let EventResult::Callback(_) = &ret {
            self.toggle_select_unselect();
        }

        ret
    }

    fn on_key_event(&mut self, ev: KeyEvent) -> EventResult {
        match self.input_widget.on_key_event(ev) {
            EventResult::Ignore => match key_event_to_code(ev) {
                KeyCode::Tab | KeyCode::BackTab => {
                    self.selected_widget.toggle_active_form();
                    EventResult::Nop
                }
                KeyCode::Enter => {
                    let ret = self.selected_widget.on_key_event(KeyCode::Enter.into());
                    self.toggle_select_unselect();
                    ret
                }
                _ => self.selected_widget.on_key_event(ev),
            },
            _ => {
                self.selected_widget.activate_form_by_index(0);
                self.selected_widget
                    .update_filter(self.input_widget.content());
                EventResult::Nop
            }
        }
    }

    fn update_chunk(&mut self, chunk: Rect) {
        self.chunk = chunk;

        self.inner_chunks = self.layout.split(self.widget_config.block().inner(chunk));

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

#[inline]
fn is_odd(num: u16) -> bool {
    num & 1 != 0
}
