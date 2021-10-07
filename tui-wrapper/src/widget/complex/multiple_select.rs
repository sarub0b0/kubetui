use crate::{
    crossterm::event::{KeyCode, KeyEvent, MouseEvent},
    event::EventResult,
    tui::{
        backend::Backend,
        layout::{Alignment, Constraint, Direction, Layout, Rect},
        style::*,
        text::Span,
        widgets::{Block, Paragraph},
        Frame,
    },
    util::{
        contains, default_focus_block, focus_block, focus_title_style, key_event_to_code, mouse_pos,
    },
    widget::*,
    Window,
};

use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

use unicode_width::UnicodeWidthStr;

use derivative::*;

use super::input::InputForm;

mod inner {
    use std::collections::HashMap;

    #[derive(Debug, Default)]
    pub struct SelectItems {
        items: HashMap<String, bool>,
    }

    impl SelectItems {
        pub fn update_items<T>(&mut self, items: impl Into<Vec<T>>)
        where
            T: Into<String>,
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

        pub fn toggle_select_unselect(&mut self, key: &str) {
            if let Some(value) = self.items.get_mut(key) {
                *value = !*value;
            }
        }

        #[allow(dead_code)]
        pub fn items(&self) -> Vec<&str> {
            self.items.iter().map(|(k, _)| k.as_str()).collect()
        }

        pub fn selected_items(&self) -> Vec<String> {
            Self::filter_items(&self.items, true)
        }

        pub fn unselected_items(&self) -> Vec<String> {
            Self::filter_items(&self.items, false)
        }

        pub fn select_all(&mut self) {
            self.items.values_mut().for_each(|v| *v = true);
        }

        pub fn unselect_all(&mut self) {
            self.items.values_mut().for_each(|v| *v = false);
        }

        fn filter_items(items: &HashMap<String, bool>, selected: bool) -> Vec<String> {
            let mut ret: Vec<String> = items
                .iter()
                .filter_map(|(k, v)| {
                    if *v == selected {
                        Some(k.to_string())
                    } else {
                        None
                    }
                })
                .collect();

            ret.sort();
            ret
        }

        #[allow(dead_code)]
        pub fn select(&mut self, key: &str) {
            if let Some(value) = self.items.get_mut(key) {
                *value = true;
            }
        }

        #[allow(dead_code)]
        pub fn unselect(&mut self, key: &str) {
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

            items.update_items(["Item 0", "Item 1", "Item 2", "Item 3", "Item 4", "Item 5"]);

            items.select("Item 2");
            items.select("Item 5");
            items.select("Item 4");

            assert_eq!(items.selected_items(), vec!["Item 2", "Item 4", "Item 5"]);

            items.unselect("Item 2");

            assert_eq!(items.selected_items(), vec!["Item 4", "Item 5"]);
        }

        #[test]
        fn update_items() {
            let mut items = SelectItems::default();

            items.update_items(["Item 0", "Item 1", "Item 2", "Item 3", "Item 4", "Item 5"]);

            items.select("Item 2");
            items.select("Item 5");
            items.select("Item 4");

            assert_eq!(items.selected_items(), vec!["Item 2", "Item 4", "Item 5"]);

            items.update_items(["Item 0", "Item 1", "Item 2"]);

            assert_eq!(items.selected_items(), vec!["Item 2"]);
        }
    }
}

use inner::SelectItems;

#[derive(Derivative)]
#[derivative(Debug)]
struct SelectForm<'a> {
    items: SelectItems,
    filter: String,
    list_widget: List<'a>,
    selected_widget: List<'a>,
    chunk: Rect,
    focus_id: usize,
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
            focus_id: 0,
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
                    self.chunk.width / 2 - 1,
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
                let right_chunk = Rect::new(cx, center_chunk.y + 1, cw, ch - margin);

                ([left_chunk, center_chunk, right_chunk], "↓".to_string())
            }
        }
    }

    fn render<B: Backend>(&mut self, f: &mut Frame<B>, _: bool) {
        let (chunks, arrow) = self.chunks_and_arrow();

        let arrow = Paragraph::new(Span::styled(
            arrow,
            Style::default().add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Center)
        .block(Block::default());

        self.list_widget.render(f, self.focus_id == 0);

        f.render_widget(arrow, chunks[1]);

        self.selected_widget.render(f, self.focus_id == 1);
    }

    fn update_layout(&mut self, chunk: Rect) {
        if 65 < chunk.width {
            self.direction = Direction::Horizontal;
        } else {
            self.direction = Direction::Vertical;
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
        self.focused_form_mut().select_next(i);
    }

    fn select_prev(&mut self, i: usize) {
        self.focused_form_mut().select_prev(i);
    }

    fn select_first(&mut self) {
        self.focused_form_mut().select_first();
    }

    fn select_last(&mut self) {
        self.focused_form_mut().select_last();
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

    fn focused_form(&mut self) -> &List<'a> {
        if self.focus_id == 0 {
            &self.list_widget
        } else {
            &self.selected_widget
        }
    }

    fn focused_form_mut(&mut self) -> &mut List<'a> {
        if self.focus_id == 0 {
            &mut self.list_widget
        } else {
            &mut self.selected_widget
        }
    }

    #[allow(dead_code)]
    fn unfocused_form_mut(&mut self) -> &mut List<'a> {
        if self.focus_id == 1 {
            &mut self.list_widget
        } else {
            &mut self.selected_widget
        }
    }

    fn toggle_focus(&mut self) {
        if self.focus_id == 0 {
            self.focus_id = 1
        } else {
            self.focus_id = 0
        }
    }

    fn focus(&mut self, id: usize) {
        self.focus_id = id;
    }

    fn update_widget_item(&mut self, items: WidgetItem) {
        self.items.update_items(items.as_array());

        self.update_widgets();
    }

    fn update_widgets(&mut self) {
        self.list_widget.update_widget_item(WidgetItem::Array(
            self.filter_items(&self.items.unselected_items()),
        ));
        self.selected_widget
            .update_widget_item(WidgetItem::Array(self.items.selected_items()));
    }

    fn toggle_select_unselect(&mut self) {
        let list = self.focused_form();
        let selected_key = list.state().selected().map(|i| list.items()[i].to_string());

        if let Some(key) = selected_key {
            self.items.toggle_select_unselect(&key);
            self.update_widgets();
        }
    }

    fn update_filter(&mut self, filter: impl Into<String>) {
        self.filter = filter.into();

        self.list_widget.update_widget_item(WidgetItem::Array(
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

    fn selected_items(&self) -> Vec<String> {
        self.items.selected_items()
    }

    fn select_item(&mut self, item: &str) {
        if let Some((i, _)) = self
            .list_widget
            .items()
            .iter()
            .enumerate()
            .find(|(_, i)| item == i.as_str())
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
        let pos = mouse_pos(ev);

        let (chunks, _) = self.chunks_and_arrow();

        if contains(chunks[0], pos) {
            self.focus(0);
            self.list_widget.on_mouse_event(ev)
        } else if contains(chunks[2], pos) {
            self.focus(1);
            self.selected_widget.on_mouse_event(ev)
        } else {
            EventResult::Nop
        }
    }

    fn on_key_event(&mut self, ev: KeyEvent) -> EventResult {
        self.focused_form_mut().on_key_event(ev)
    }
}

#[derive(Debug, Default)]
pub struct MultipleSelectBuilder {
    id: String,
    title: String,
}

impl MultipleSelectBuilder {
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
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

        let selected_widget = SelectForm {
            list_widget: ListBuilder::default().title("Items").build(),
            selected_widget: ListBuilder::default().title("Selected").build(),
            ..Default::default()
        };

        MultipleSelect {
            id: self.id,
            title: self.title,
            layout,
            selected_widget,
            ..Default::default()
        }
    }
}

const LAYOUT_INDEX_FOR_INPUT_FORM: usize = 0;
const LAYOUT_INDEX_FOR_STATUS: usize = 1;
const LAYOUT_INDEX_FOR_SELECT_FORM: usize = 2;

#[derive(Derivative)]
#[derivative(Debug, Default)]
pub struct MultipleSelect<'a> {
    id: String,
    title: String,
    chunk_index: usize,
    input_widget: InputForm<'a>,
    selected_widget: SelectForm<'a>,
    layout: Layout,
    chunk: Rect,
}

impl RenderTrait for MultipleSelect<'_> {
    fn render<B: Backend>(&mut self, f: &mut Frame<B>, selected: bool) {
        let title = self.title.to_string();
        let block = focus_block(&title, selected);
        let inner_chunk = block.inner(self.chunk);

        f.render_widget(
            block.title(Span::styled(
                format!(" {} ", self.title()),
                focus_title_style(selected),
            )),
            self.chunk,
        );

        self.input_widget.render(f);

        let status = self.selected_widget.status();
        f.render_widget(
            Paragraph::new(format!("[{}/{}]", status.0, status.1)),
            self.layout.split(inner_chunk)[LAYOUT_INDEX_FOR_STATUS],
        );
        self.selected_widget.render(f, selected);
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
    pub fn on_select<F>(mut self, f: F) -> Self
    where
        F: Fn(&mut Window, &String) -> EventResult + 'static + Clone,
    {
        self.selected_widget.list_widget = self.selected_widget.list_widget.on_select(f.clone());
        self.selected_widget.selected_widget = self.selected_widget.selected_widget.on_select(f);
        self
    }

    fn clear_filter(&mut self) {
        self.input_widget.clear();
        self.selected_widget.update_filter("");
    }

    pub fn selected_items(&self) -> Vec<String> {
        self.selected_widget.selected_items()
    }

    pub fn select_item(&mut self, item: &str) {
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
}

impl WidgetTrait for MultipleSelect<'_> {
    fn focusable(&self) -> bool {
        true
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

    fn update_widget_item(&mut self, items: WidgetItem) {
        self.clear_filter();
        self.selected_widget.update_widget_item(items);
    }

    fn append_widget_item(&mut self, _: WidgetItem) {}

    fn widget_item(&self) -> Option<WidgetItem> {
        Some(WidgetItem::Array(self.selected_widget.selected_items()))
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
                if let KeyCode::Tab | KeyCode::BackTab = key_event_to_code(ev) {
                    self.selected_widget.toggle_focus();
                } else {
                    return self.selected_widget.on_key_event(ev);
                }
            }
            _ => {
                self.selected_widget.focus(0);
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

    fn select_index(&mut self, _: usize) {
        todo!()
    }

    fn update_title(&mut self, title: impl Into<String>) {
        self.title = title.into();
    }
}

#[inline]
fn is_odd(num: u16) -> bool {
    num & 1 != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    mod widget_trait {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn update_title() {
            let mut w = MultipleSelectBuilder::default()
                .title("multiple-select")
                .build();
            assert_eq!("multiple-select", w.title());

            w.update_title("multiple-select update");
            assert_eq!("multiple-select update", w.title());
        }
    }
}
