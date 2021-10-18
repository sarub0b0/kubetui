use std::rc::Rc;

use tui::{
    backend::Backend,
    layout::Rect,
    style::{Modifier, Style},
    Frame,
};

use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};

use tui::widgets::{self, Block, ListState};

use super::{config::WidgetConfig, Item, RenderTrait, WidgetTrait};
use crate::{
    event::{Callback, EventResult},
    key_event_to_code, Window,
};

use derivative::*;

mod inner_item {

    use super::Item;
    use tui::widgets::ListItem;

    #[derive(Debug, Default)]
    pub struct InnerItem<'a> {
        items: Vec<String>,
        list_item: Vec<ListItem<'a>>,
    }

    impl<'a> InnerItem<'a> {
        pub fn items(&self) -> &Vec<String> {
            &self.items
        }

        #[allow(dead_code)]
        pub fn list_item(&self) -> &Vec<ListItem> {
            &self.list_item
        }

        pub fn update_item(&mut self, item: Item) {
            self.items = item.array();
            self.list_item = self.items.iter().cloned().map(ListItem::new).collect();
        }

        pub fn widget_items(&self) -> &[ListItem<'a>] {
            &self.list_item
        }

        pub fn len(&self) -> usize {
            self.items.len()
        }

        pub fn is_empty(&self) -> bool {
            self.items.is_empty()
        }
    }
}

use inner_item::InnerItem;
#[derive(Derivative)]
#[derivative(Debug, Default)]
pub struct List<'a> {
    id: String,
    widget_config: WidgetConfig,
    items: InnerItem<'a>,
    state: ListState,
    chunk: Rect,
    inner_chunk: Rect,
    #[derivative(Debug = "ignore")]
    on_select: Option<Rc<dyn Fn(&mut Window, &String) -> EventResult>>,
}

#[derive(Derivative)]
#[derivative(Debug, Default)]
pub struct ListBuilder {
    id: String,
    widget_config: WidgetConfig,
    items: Vec<String>,
    #[derivative(Debug = "ignore")]
    on_select: Option<Rc<dyn Fn(&mut Window, &String) -> EventResult>>,
}

impl ListBuilder {
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    pub fn widget_config(mut self, widget_config: &WidgetConfig) -> Self {
        self.widget_config = widget_config.clone();
        self
    }

    pub fn items(mut self, items: impl Into<Vec<String>>) -> Self {
        self.items = items.into();
        self
    }

    pub fn on_select<F>(mut self, cb: F) -> Self
    where
        F: Fn(&mut Window, &String) -> EventResult + 'static,
    {
        self.on_select = Some(Rc::new(cb));
        self
    }

    pub fn build(self) -> List<'static> {
        let mut list = List {
            id: self.id,
            widget_config: self.widget_config,
            on_select: self.on_select,
            ..Default::default()
        };

        list.update_widget_item(Item::Array(self.items));
        list
    }
}

impl<'a> List<'a> {
    pub fn builder() -> ListBuilder {
        ListBuilder::default()
    }

    pub fn items(&self) -> &Vec<String> {
        self.items.items()
    }

    pub fn state(&self) -> &ListState {
        &self.state
    }

    pub fn on_select_mut(
        &mut self,
    ) -> &mut Option<Rc<dyn Fn(&mut Window, &String) -> EventResult>> {
        &mut self.on_select
    }

    fn widget(&self, block: Block<'a>) -> widgets::List<'a> {
        widgets::List::new(self.items.widget_items().to_vec())
            .block(block)
            .style(Style::default())
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
    }
}

impl<'a> WidgetTrait for List<'a> {
    fn id(&self) -> &str {
        &self.id
    }

    fn focusable(&self) -> bool {
        true
    }

    fn widget_item(&self) -> Option<Item> {
        self.state
            .selected()
            .map(|i| Item::Single(self.items.items()[i].clone()))
    }

    fn chunk(&self) -> Rect {
        self.chunk
    }

    fn select_index(&mut self, index: usize) {
        let i = if self.items.len() <= index {
            self.items.len().saturating_sub(1)
        } else {
            index
        };
        self.state.select(Some(i));
    }

    fn select_next(&mut self, index: usize) {
        let i = match self.state.selected() {
            Some(i) => {
                if self.items.len().saturating_sub(1) < i + index {
                    self.items.len().saturating_sub(1)
                } else {
                    i + index
                }
            }
            None => 0,
        };

        self.state.select(Some(i));
    }

    fn select_prev(&mut self, index: usize) {
        let i = self.state.selected().unwrap_or(0).saturating_sub(index);

        self.state.select(Some(i));
    }

    fn select_first(&mut self) {
        self.state.select(Some(0));
    }

    fn select_last(&mut self) {
        if self.items.is_empty() {
            self.state.select(Some(0));
        } else {
            self.state.select(Some(self.items.len() - 1))
        }
    }

    fn append_widget_item(&mut self, _: Item) {
        unimplemented!()
    }

    fn update_widget_item(&mut self, items: Item) {
        let old_len = self.items.len();

        self.items.update_item(items);

        match self.items.len() {
            0 => self.state.select(None),
            new_len if new_len < old_len => {
                let i = self.state.selected();
                if i == Some(old_len - 1) {
                    self.state.select(Some(new_len - 1));
                }
            }
            _ => {
                if self.state.selected() == None {
                    self.state.select(Some(0))
                }
            }
        }
    }

    fn on_mouse_event(&mut self, ev: MouseEvent) -> EventResult {
        if self.items.is_empty() {
            return EventResult::Nop;
        }

        let (_, row) = (
            ev.column.saturating_sub(self.inner_chunk.left()) as usize,
            ev.row.saturating_sub(self.inner_chunk.top()) as usize,
        );

        if self.items.len() <= row {
            return EventResult::Nop;
        }

        match ev.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                self.state.select(Some(row + self.state.offset()));

                return EventResult::Callback(self.on_select_callback());
            }

            MouseEventKind::ScrollDown => {
                self.select_next(1);
            }
            MouseEventKind::ScrollUp => {
                self.select_prev(1);
            }
            MouseEventKind::Down(_) => {}
            MouseEventKind::Up(_) => {}
            MouseEventKind::Drag(_) => {}
            MouseEventKind::Moved => {}
        }
        EventResult::Nop
    }

    fn on_key_event(&mut self, ev: KeyEvent) -> EventResult {
        match key_event_to_code(ev) {
            KeyCode::Char('j') | KeyCode::Down | KeyCode::PageDown => {
                self.select_next(1);
            }

            KeyCode::Char('k') | KeyCode::Up | KeyCode::PageUp => {
                self.select_prev(1);
            }

            KeyCode::Char('G') | KeyCode::End => {
                self.select_last();
            }
            KeyCode::Char('g') | KeyCode::Home => {
                self.select_first();
            }

            KeyCode::Enter => {
                return EventResult::Callback(self.on_select_callback());
            }
            KeyCode::Char(_) => {
                return EventResult::Ignore;
            }
            _ => {
                return EventResult::Ignore;
            }
        }

        EventResult::Nop
    }

    fn update_chunk(&mut self, chunk: Rect) {
        self.chunk = chunk;
        self.inner_chunk = self.widget_config.block().inner(chunk);
    }

    fn clear(&mut self) {
        self.items = Default::default();
        self.state = Default::default();
        *(self.widget_config.append_title_mut()) = None;
    }

    fn widget_config(&self) -> &WidgetConfig {
        &self.widget_config
    }

    fn widget_config_mut(&mut self) -> &mut WidgetConfig {
        &mut self.widget_config
    }
}

impl<'a> List<'a> {
    fn on_select_callback(&self) -> Option<Callback> {
        self.on_select.clone().and_then(|cb| {
            self.selected_item()
                .map(|v| Callback::from_fn(move |w| cb(w, &v)))
        })
    }

    fn selected_item(&self) -> Option<Rc<String>> {
        self.state
            .selected()
            .map(|i| Rc::new(self.items.items()[i].clone()))
    }
}

impl RenderTrait for List<'_> {
    fn render<B: Backend>(&mut self, f: &mut Frame<B>, selected: bool) {
        f.render_stateful_widget(
            self.widget(
                self.widget_config
                    .render_block(self.focusable() && selected),
            ),
            self.chunk,
            &mut self.state,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn initial_index() {
        let mut list = List::default();
        list.update_widget_item(Item::Array(vec![
            String::from("Item 0"),
            String::from("Item 1"),
            String::from("Item 2"),
        ]));

        assert_eq!(Some(0), list.state.selected())
    }

    #[test]
    fn two_prev_is_selected_last_index() {
        let mut list = List::default();
        list.update_widget_item(Item::Array(vec![
            String::from("Item 0"),
            String::from("Item 1"),
            String::from("Item 2"),
        ]));

        list.select_prev(2);
        assert_eq!(Some(0), list.state.selected())
    }
    #[test]
    fn one_next_is_selected_second_index() {
        let mut list = List::default();
        list.update_widget_item(Item::Array(vec![
            String::from("Item 0"),
            String::from("Item 1"),
            String::from("Item 2"),
        ]));

        list.select_next(1);
        assert_eq!(Some(1), list.state.selected())
    }

    #[test]
    fn last_next_is_selected_first_index() {
        let mut list = List::default();
        list.update_widget_item(Item::Array(vec![
            String::from("Item 0"),
            String::from("Item 1"),
            String::from("Item 2"),
        ]));

        list.select_next(3);
        assert_eq!(Some(2), list.state.selected())
    }

    #[test]
    fn next_offset() {
        let chunk = Rect::new(0, 0, 10, 5);
        let mut list = List::default();
        list.update_widget_item(Item::Array(vec![
            "Item-0".to_string(),
            "Item-1".to_string(),
            "Item-2".to_string(),
            "Item-3".to_string(),
            "Item-4".to_string(),
            "Item-5".to_string(),
            "Item-6".to_string(),
        ]));

        list.update_chunk(chunk);

        list.select_next(5);

        list.select_next(10);

        assert_eq!(list.state.selected().unwrap(), 6);
    }

    #[test]
    fn prev_offset() {
        let chunk = Rect::new(0, 0, 10, 5);
        let mut list = List::default();
        list.update_widget_item(Item::Array(vec![
            "Item-0".to_string(),
            "Item-1".to_string(),
            "Item-2".to_string(),
            "Item-3".to_string(),
            "Item-4".to_string(),
            "Item-5".to_string(),
            "Item-6".to_string(),
        ]));

        list.update_chunk(chunk);

        list.select_next(10);

        list.select_prev(5);

        assert_eq!(list.state.selected().unwrap(), 1);

        list.select_prev(4);
        assert_eq!(list.state.selected().unwrap(), 0);
    }
}
