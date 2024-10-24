use std::collections::BTreeSet;

use ratatui::{
    crossterm::event::{KeyEvent, MouseEvent},
    layout::Rect,
    Frame,
};

use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};

use crate::ui::{
    event::EventResult,
    widget::{
        list::{OnSelectCallback, RenderBlockInjection},
        styled_graphemes::StyledGraphemes,
        Item, List, LiteralItem, RenderTrait, SelectedItem, WidgetBase, WidgetTrait,
    },
};

#[derive(Debug)]
pub struct SelectFormBuilder {
    widget_base: WidgetBase,
    on_select: Option<OnSelectCallback>,
    block_injection: Option<RenderBlockInjection>,
}

#[allow(dead_code)]
impl SelectFormBuilder {
    pub fn widget_base(mut self, widget_base: WidgetBase) -> Self {
        self.widget_base = widget_base;
        self
    }

    pub fn on_select(mut self, on_select: impl Into<OnSelectCallback>) -> Self {
        self.on_select = Some(on_select.into());
        self
    }

    pub fn block_injection(mut self, block_injection: impl Into<RenderBlockInjection>) -> Self {
        self.block_injection = Some(block_injection.into());
        self
    }

    pub fn build(self) -> SelectForm<'static> {
        let mut list_widget = List::builder();

        if let Some(on_select) = self.on_select {
            list_widget = list_widget.on_select(on_select);
        }

        if let Some(block_injection) = self.block_injection {
            list_widget = list_widget.block_injection(block_injection);
        }

        list_widget = list_widget.widget_base(self.widget_base);

        SelectForm {
            list_items: BTreeSet::new(),
            list_widget: list_widget.build(),
            filter: "".to_string(),
            chunk: Rect::default(),
            matcher: SkimMatcherV2::default(),
        }
    }
}

impl Default for SelectFormBuilder {
    fn default() -> Self {
        Self {
            widget_base: WidgetBase::builder().title("Items").build(),
            on_select: None,
            block_injection: None,
        }
    }
}

pub struct SelectForm<'a> {
    list_items: BTreeSet<LiteralItem>,
    list_widget: List<'a>,
    filter: String,
    chunk: Rect,
    matcher: SkimMatcherV2,
}

impl std::fmt::Debug for SelectForm<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SelectForm")
            .field("list_items", &self.list_items)
            .field("list_widget", &self.list_widget)
            .field("filter", &self.filter)
            .field("chunk", &self.chunk)
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

    pub fn render(&mut self, f: &mut Frame) {
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

    pub fn update_chunk(&mut self, chunk: Rect) {
        self.chunk = chunk;
        self.list_widget.update_chunk(chunk);
    }

    pub fn update_widget_item(&mut self, items: Item) {
        self.list_items = items.clone().array().into_iter().collect();

        self.list_widget.update_widget_item(items);

        let filter = self.filter.clone();
        self.update_filter(&filter);
    }

    pub fn widget_item(&self) -> Option<SelectedItem> {
        self.list_widget.widget_item()
    }

    pub fn update_filter(&mut self, filter: impl Into<String>) {
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

    pub fn status(&self) -> (usize, usize) {
        let mut pos = self.list_widget.state().selected().unwrap_or(0);

        let size = self.list_widget.items().len();

        if 0 < size {
            pos += 1;
        }

        (pos, size)
    }

    pub fn on_mouse_event(&mut self, ev: MouseEvent) -> EventResult {
        self.list_widget.on_mouse_event(ev)
    }

    pub fn on_key_event(&mut self, ev: KeyEvent) -> EventResult {
        self.list_widget.on_key_event(ev)
    }

    pub fn select_next(&mut self, n: usize) {
        self.list_widget.select_next(n);
    }

    pub fn select_prev(&mut self, n: usize) {
        self.list_widget.select_prev(n);
    }

    pub fn select_first(&mut self) {
        self.list_widget.select_first();
    }

    pub fn select_last(&mut self) {
        self.list_widget.select_last();
    }
}

#[cfg(test)]
mod tests {
    use crate::ui::widget::LiteralItem;

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
