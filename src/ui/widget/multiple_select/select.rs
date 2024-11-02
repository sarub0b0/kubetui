use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};

use ratatui::{
    crossterm::event::{KeyEvent, MouseButton, MouseEvent, MouseEventKind},
    layout::{Alignment, Direction, Rect},
    style::*,
    text::Span,
    widgets::{Block, Paragraph},
    Frame,
};

use unicode_width::UnicodeWidthStr;

use crate::ui::{
    event::EventResult,
    util::{MousePosition, RectContainsPoint},
    widget::{
        list::{OnSelectCallback, RenderBlockInjection},
        styled_graphemes::StyledGraphemes,
        Item, List, ListTheme, LiteralItem, RenderTrait as _, WidgetBase, WidgetTheme,
        WidgetTrait as _,
    },
};

use super::SelectItems;

const LIST_FORM_ID: usize = 0;
const SELECTED_FORM_ID: usize = 1;

#[derive(Debug, Default)]
pub struct SelectFormTheme {
    pub list_theme: ListTheme,
    pub widget_theme: WidgetTheme,
}

#[derive(Debug, Default)]
pub struct SelectFormBuilder {
    theme: SelectFormTheme,

    on_select_selected: Option<OnSelectCallback>,
    block_injection_selected: Option<RenderBlockInjection>,

    on_select_unselected: Option<OnSelectCallback>,
    block_injection_unselected: Option<RenderBlockInjection>,
}

#[allow(dead_code)]
impl SelectFormBuilder {
    pub fn theme(mut self, theme: SelectFormTheme) -> Self {
        self.theme = theme;
        self
    }

    pub fn on_select_selected(mut self, on_select: impl Into<OnSelectCallback>) -> Self {
        self.on_select_selected = Some(on_select.into());
        self
    }

    pub fn block_injection_selected(
        mut self,
        block_injection: impl Into<RenderBlockInjection>,
    ) -> Self {
        self.block_injection_selected = Some(block_injection.into());
        self
    }

    pub fn on_select_unselected(mut self, on_select: impl Into<OnSelectCallback>) -> Self {
        self.on_select_unselected = Some(on_select.into());
        self
    }

    pub fn block_injection_unselected(
        mut self,
        block_injection: impl Into<RenderBlockInjection>,
    ) -> Self {
        self.block_injection_unselected = Some(block_injection.into());
        self
    }

    pub fn build(self) -> SelectForm<'static> {
        let selected_widget = {
            let mut builder = List::builder().theme(self.theme.list_theme.clone());

            let widget_base = WidgetBase::builder()
                .theme(self.theme.widget_theme.clone())
                .title("Selected")
                .build();

            builder = builder.widget_base(widget_base);

            if let Some(on_select) = self.on_select_unselected {
                builder = builder.on_select(on_select);
            }

            if let Some(block_injection) = self.block_injection_unselected {
                builder = builder.block_injection(block_injection);
            }

            builder
        };

        let unselected_widget = {
            let mut builder = List::builder().theme(self.theme.list_theme);

            let widget_base = WidgetBase::builder()
                .theme(self.theme.widget_theme)
                .title("Items")
                .build();

            builder = builder.widget_base(widget_base);

            if let Some(on_select) = self.on_select_selected {
                builder = builder.on_select(on_select);
            }

            if let Some(block_injection) = self.block_injection_selected {
                builder = builder.block_injection(block_injection);
            }

            builder
        };

        SelectForm {
            items: SelectItems::default(),
            filter: String::default(),
            selected_widget: selected_widget.build(),
            unselected_widget: unselected_widget.build(),
            chunk: Rect::default(),
            active_form_index: 0,
            mouse_over_widget_index: None,
            matcher: SkimMatcherV2::default(),
            direction: Direction::Vertical,
        }
    }
}

pub struct SelectForm<'a> {
    items: SelectItems,
    filter: String,
    selected_widget: List<'a>,
    unselected_widget: List<'a>,
    chunk: Rect,
    active_form_index: usize,
    mouse_over_widget_index: Option<usize>,
    direction: Direction,
    matcher: SkimMatcherV2,
}

impl std::fmt::Debug for SelectForm<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SelectForm")
            .field("items", &self.items)
            .field("filter", &self.filter)
            .field("selected_widget", &self.selected_widget)
            .field("unselected_widget", &self.unselected_widget)
            .field("chunk", &self.chunk)
            .field("active_form_index", &self.active_form_index)
            .field("mouse_over_widget_index", &self.mouse_over_widget_index)
            .field("direction", &self.direction)
            .field("matcher", &"SkimMatcherV2")
            .finish()
    }
}

impl Default for SelectForm<'_> {
    fn default() -> Self {
        SelectFormBuilder::default().build()
    }
}

impl<'a> SelectForm<'a> {
    pub fn builder() -> SelectFormBuilder {
        SelectFormBuilder::default()
    }

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

    pub fn render(&mut self, f: &mut Frame) {
        let (chunks, arrow) = self.chunks_and_arrow();

        let arrow = Paragraph::new(Span::styled(
            arrow,
            Style::default().add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Center)
        .block(Block::default());

        self.unselected_widget.render(
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

    pub fn update_chunk(&mut self, chunk: Rect) {
        self.update_layout(chunk);

        self.chunk = chunk;

        let (chunks, _) = self.chunks_and_arrow();

        self.unselected_widget.update_chunk(chunks[0]);
        self.selected_widget.update_chunk(chunks[2]);
    }

    pub fn select_next(&mut self, i: usize) {
        self.active_form_mut().select_next(i);
    }

    pub fn select_prev(&mut self, i: usize) {
        self.active_form_mut().select_prev(i);
    }

    pub fn select_first(&mut self) {
        self.active_form_mut().select_first();
    }

    pub fn select_last(&mut self) {
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

    pub fn active_form(&mut self) -> &List<'a> {
        if self.active_form_index == LIST_FORM_ID {
            &self.unselected_widget
        } else {
            &self.selected_widget
        }
    }

    pub fn active_form_mut(&mut self) -> &mut List<'a> {
        if self.active_form_index == LIST_FORM_ID {
            &mut self.unselected_widget
        } else {
            &mut self.selected_widget
        }
    }

    #[allow(dead_code)]
    pub fn inactive_form_mut(&mut self) -> &mut List<'a> {
        if self.active_form_index == LIST_FORM_ID {
            &mut self.selected_widget
        } else {
            &mut self.unselected_widget
        }
    }

    pub fn toggle_active_form(&mut self) {
        self.clear_mouse_over();

        if self.active_form_index == LIST_FORM_ID {
            self.active_form_index = SELECTED_FORM_ID
        } else {
            self.active_form_index = LIST_FORM_ID
        }
    }

    pub fn activate_form_by_index(&mut self, index: usize) {
        self.clear_mouse_over();

        self.active_form_index = index;
    }

    pub fn update_widget_item(&mut self, items: Item) {
        self.items.update_items(items.as_array());

        self.update_widgets();
    }

    fn update_widgets(&mut self) {
        self.unselected_widget.update_widget_item(Item::Array(
            self.filter_items(&self.items.unselected_items()),
        ));
        self.selected_widget
            .update_widget_item(Item::Array(self.items.selected_items()));
    }

    pub fn toggle_select_unselect(&mut self) {
        let list = self.active_form();
        let selected_key = list.state().selected().map(|i| list.items()[i].clone());

        if let Some(key) = selected_key {
            self.items.toggle_select_unselect(&key);
            self.update_widgets();
        }
    }

    pub fn update_filter(&mut self, filter: impl Into<String>) {
        self.filter = filter.into();

        self.unselected_widget.update_widget_item(Item::Array(
            self.filter_items(&self.items.unselected_items()),
        ));

        let current_pos = self.unselected_widget.state().selected();

        if let Some(pos) = current_pos {
            if self.unselected_widget.items().len() <= pos {
                self.unselected_widget.select_last()
            }
        }
    }

    pub fn status(&self) -> (usize, usize) {
        let mut pos = self.unselected_widget.state().selected().unwrap_or(0);

        let size = self.unselected_widget.items().len();

        if 0 < size {
            pos += 1;
        }

        (pos, size)
    }

    pub fn selected_items(&self) -> Vec<LiteralItem> {
        self.items.selected_items()
    }

    pub fn select_item(&mut self, item: &LiteralItem) {
        if let Some((i, _)) = self
            .unselected_widget
            .items()
            .iter()
            .enumerate()
            .find(|(_, i)| item == *i)
        {
            self.unselected_widget.select_index(i);
            self.toggle_select_unselect();
            self.unselected_widget.select_first();
        }
    }

    pub fn select_all(&mut self) {
        self.items.select_all();
        self.update_widgets();
    }

    pub fn unselect_all(&mut self) {
        self.items.unselect_all();
        self.update_widgets();
    }

    pub fn on_mouse_event(&mut self, ev: MouseEvent) -> EventResult {
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

    pub fn on_key_event(&mut self, ev: KeyEvent) -> EventResult {
        self.clear_mouse_over();

        self.active_form_mut().on_key_event(ev)
    }

    pub fn clear_mouse_over(&mut self) {
        self.mouse_over_widget_index = None;
    }
}

#[inline]
fn is_odd(num: u16) -> bool {
    num & 1 != 0
}
