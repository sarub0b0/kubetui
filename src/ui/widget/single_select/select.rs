use std::{cmp::Reverse, collections::BTreeSet};

use ratatui::{
    crossterm::event::{KeyEvent, MouseEvent},
    layout::Rect,
    Frame,
};

use nucleo_matcher::{
    pattern::{CaseMatching, Normalization, Pattern},
    Config, Matcher, Utf32String,
};

use crate::{
    message::UserEvent,
    ui::{
        event::{Callback, EventResult},
        widget::{
            list::{OnSelectCallback, RenderBlockInjection},
            styled_graphemes::StyledGraphemes,
            Item, List, ListTheme, LiteralItem, RenderTrait, SelectedItem, WidgetBase, WidgetTheme,
            WidgetTrait,
        },
    },
};

#[derive(Debug, Default)]
pub struct SelectFormTheme {
    pub list_theme: ListTheme,
    pub widget_theme: WidgetTheme,
}

#[derive(Debug, Default)]
pub struct SelectFormBuilder {
    theme: SelectFormTheme,
    actions: Vec<(UserEvent, Callback)>,
    on_select: Option<OnSelectCallback>,
    block_injection: Option<RenderBlockInjection>,
}

#[allow(dead_code)]
impl SelectFormBuilder {
    pub fn theme(mut self, theme: SelectFormTheme) -> Self {
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

    pub fn on_select(mut self, on_select: impl Into<OnSelectCallback>) -> Self {
        self.on_select = Some(on_select.into());
        self
    }

    pub fn block_injection(mut self, block_injection: impl Into<RenderBlockInjection>) -> Self {
        self.block_injection = Some(block_injection.into());
        self
    }

    pub fn build(self) -> SelectForm<'static> {
        let mut builder = List::builder();

        if let Some(on_select) = self.on_select {
            builder = builder.on_select(on_select);
        }

        if let Some(block_injection) = self.block_injection {
            builder = builder.block_injection(block_injection);
        }

        for action in self.actions {
            builder = builder.action(action.0, action.1);
        }

        let widget_base = WidgetBase::builder()
            .theme(self.theme.widget_theme)
            .title("Items")
            .build();

        let list_widget = builder
            .widget_base(widget_base)
            .theme(self.theme.list_theme)
            .build();

        SelectForm {
            list_items: BTreeSet::new(),
            list_widget,
            filter: "".to_string(),
            chunk: Rect::default(),
            matcher: Matcher::new(Config::DEFAULT),
        }
    }
}

#[derive(Debug)]
pub struct SelectForm<'a> {
    list_items: BTreeSet<LiteralItem>,
    list_widget: List<'a>,
    filter: String,
    chunk: Rect,
    matcher: Matcher,
}

impl Default for SelectForm<'_> {
    fn default() -> Self {
        SelectFormBuilder::default().build()
    }
}

impl SelectForm<'_> {
    pub fn builder() -> SelectFormBuilder {
        SelectFormBuilder::default()
    }

    pub fn render(&mut self, f: &mut Frame) {
        self.list_widget.render(f, true, false);
    }

    fn filter_items(&self, items: &BTreeSet<LiteralItem>) -> Vec<LiteralItem> {
        struct MatchedItem {
            score: u32,
            item: LiteralItem,
        }

        // Empty filter means show all items
        if self.filter.is_empty() {
            return items.iter().cloned().collect();
        }

        let pattern = Pattern::parse(&self.filter, CaseMatching::Ignore, Normalization::Smart);

        let mut matcher = self.matcher.clone();

        let mut ret: Vec<MatchedItem> = items
            .iter()
            .filter_map(|item| {
                let text = item.item.styled_graphemes_symbols().concat();
                let haystack = Utf32String::from(text.as_str());

                pattern
                    .score(haystack.slice(..), &mut matcher)
                    .map(|score| MatchedItem {
                        score,
                        item: item.clone(),
                    })
            })
            .collect();

        ret.sort_by_key(|item| Reverse(item.score));

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
    fn filter_basic_partial_match() {
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

    #[test]
    fn filter_empty_returns_all_items() {
        let mut select_form = SelectForm::default();

        let items = vec![
            "pod-1".to_string().into(),
            "pod-2".to_string().into(),
            "deployment-1".to_string().into(),
        ];

        select_form.update_widget_item(Item::Array(items.clone()));
        select_form.update_filter("");

        let res = select_form.list_widget.items().clone();

        // All items should be present (order may differ due to BTreeSet)
        assert_eq!(res.len(), items.len());
        for item in &items {
            assert!(res.contains(item));
        }
    }

    #[test]
    fn filter_no_match_returns_empty() {
        let mut select_form = SelectForm::default();

        select_form.update_widget_item(Item::Array(vec![
            "pod-1".to_string().into(),
            "pod-2".to_string().into(),
            "deployment-1".to_string().into(),
        ]));

        select_form.update_filter("xyz");

        let res = select_form.list_widget.items().clone();
        let expected: Vec<LiteralItem> = vec![];

        assert_eq!(res, expected);
    }

    #[test]
    fn filter_exact_match() {
        let mut select_form = SelectForm::default();

        select_form.update_widget_item(Item::Array(vec![
            "pod".to_string().into(),
            "pod-1".to_string().into(),
            "my-pod".to_string().into(),
        ]));

        select_form.update_filter("pod");

        let res = select_form.list_widget.items().clone();

        // All items should match, but exact match should score higher
        assert_eq!(res.len(), 3);
        // "pod" should be first due to exact match
        assert_eq!(res[0], "pod".to_string().into());
    }

    #[test]
    fn filter_fuzzy_match_sequential() {
        let mut select_form = SelectForm::default();

        select_form.update_widget_item(Item::Array(vec![
            "kubernetes".to_string().into(),
            "kube-system".to_string().into(),
            "test".to_string().into(),
        ]));

        select_form.update_filter("kube");

        let res = select_form.list_widget.items().clone();

        // Both "kubernetes" and "kube-system" should match
        assert_eq!(res.len(), 2);
        assert!(res.contains(&"kubernetes".to_string().into()));
        assert!(res.contains(&"kube-system".to_string().into()));
    }

    #[test]
    fn filter_fuzzy_match_non_sequential() {
        let mut select_form = SelectForm::default();

        select_form.update_widget_item(Item::Array(vec![
            "kubernetes".to_string().into(),
            "kube-system".to_string().into(),
            "test".to_string().into(),
        ]));

        // Characters in order but not consecutive
        select_form.update_filter("kbs");

        let res = select_form.list_widget.items().clone();

        // "kube-system" should match (k, b, s)
        assert!(res.contains(&"kube-system".to_string().into()));
    }

    #[test]
    fn filter_case_insensitive() {
        let mut select_form = SelectForm::default();

        select_form.update_widget_item(Item::Array(vec![
            "Pod".to_string().into(),
            "pod".to_string().into(),
            "POD".to_string().into(),
            "deployment".to_string().into(),
        ]));

        select_form.update_filter("pod");

        let res = select_form.list_widget.items().clone();

        // All three "pod" variations should match
        assert_eq!(res.len(), 3);
        assert!(res.contains(&"Pod".to_string().into()));
        assert!(res.contains(&"pod".to_string().into()));
        assert!(res.contains(&"POD".to_string().into()));
    }

    #[test]
    fn filter_kubernetes_resource_names_with_hyphens() {
        let mut select_form = SelectForm::default();

        select_form.update_widget_item(Item::Array(vec![
            "my-app-deployment".to_string().into(),
            "my-app-service".to_string().into(),
            "other-deployment".to_string().into(),
            "application".to_string().into(),
        ]));

        select_form.update_filter("my-app");

        let res = select_form.list_widget.items().clone();

        // Both "my-app-deployment" and "my-app-service" should match
        assert_eq!(res.len(), 2);
        assert!(res.contains(&"my-app-deployment".to_string().into()));
        assert!(res.contains(&"my-app-service".to_string().into()));
    }

    #[test]
    fn filter_kubernetes_resource_names_with_numbers() {
        let mut select_form = SelectForm::default();

        select_form.update_widget_item(Item::Array(vec![
            "app1".to_string().into(),
            "app123".to_string().into(),
            "app2".to_string().into(),
            "application".to_string().into(),
        ]));

        select_form.update_filter("app1");

        let res = select_form.list_widget.items().clone();

        // "app1" and "app123" should match
        assert_eq!(res.len(), 2);
        assert!(res.contains(&"app1".to_string().into()));
        assert!(res.contains(&"app123".to_string().into()));
    }

    #[test]
    fn filter_namespace_patterns() {
        let mut select_form = SelectForm::default();

        select_form.update_widget_item(Item::Array(vec![
            "default".to_string().into(),
            "kube-system".to_string().into(),
            "kube-public".to_string().into(),
            "my-namespace".to_string().into(),
        ]));

        select_form.update_filter("kube");

        let res = select_form.list_widget.items().clone();

        // "kube-system" and "kube-public" should match
        assert_eq!(res.len(), 2);
        assert!(res.contains(&"kube-system".to_string().into()));
        assert!(res.contains(&"kube-public".to_string().into()));
    }

    #[test]
    fn filter_empty_item_list() {
        let mut select_form = SelectForm::default();

        select_form.update_widget_item(Item::Array(vec![]));
        select_form.update_filter("test");

        let res = select_form.list_widget.items().clone();
        let expected: Vec<LiteralItem> = vec![];

        assert_eq!(res, expected);
    }

    #[test]
    fn filter_single_item_match() {
        let mut select_form = SelectForm::default();

        select_form.update_widget_item(Item::Array(vec!["kubernetes".to_string().into()]));
        select_form.update_filter("kube");

        let res = select_form.list_widget.items().clone();

        assert_eq!(res.len(), 1);
        assert_eq!(res[0], "kubernetes".to_string().into());
    }

    #[test]
    fn filter_single_item_no_match() {
        let mut select_form = SelectForm::default();

        select_form.update_widget_item(Item::Array(vec!["kubernetes".to_string().into()]));
        select_form.update_filter("xyz");

        let res = select_form.list_widget.items().clone();
        let expected: Vec<LiteralItem> = vec![];

        assert_eq!(res, expected);
    }

    #[test]
    fn filter_score_ordering_prefix_match_wins() {
        let mut select_form = SelectForm::default();

        select_form.update_widget_item(Item::Array(vec![
            "test-deployment".to_string().into(),
            "my-test".to_string().into(),
            "testing".to_string().into(),
        ]));

        select_form.update_filter("test");

        let res = select_form.list_widget.items().clone();

        // All should match, but items starting with "test" should rank higher
        assert_eq!(res.len(), 3);
        // "test-deployment" or "testing" should be first (both start with "test")
        let first_item = &res[0];
        assert!(
            first_item == &"test-deployment".to_string().into()
                || first_item == &"testing".to_string().into()
        );
    }

    #[test]
    fn filter_with_ansi_escape_sequences() {
        let mut select_form = SelectForm::default();

        // Test that ANSI escape sequences are handled correctly
        select_form.update_widget_item(Item::Array(vec![
            "\x1b[90mtest-pod\x1b[39m".to_string().into(),
            "test-deployment".to_string().into(),
            "other".to_string().into(),
        ]));

        select_form.update_filter("test");

        let res = select_form.list_widget.items().clone();

        assert_eq!(res.len(), 2);
        assert!(res.contains(&"\x1b[90mtest-pod\x1b[39m".to_string().into()));
        assert!(res.contains(&"test-deployment".to_string().into()));
    }
}
