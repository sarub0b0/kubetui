use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, RwLock};
use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans, Text};
use tui::widgets::{Block, List, ListItem, ListState};

use super::WidgetTrait;

pub struct Pods<'a> {
    items: Vec<String>,
    state: Rc<RefCell<PodState>>,
    list_item: Vec<ListItem<'a>>,
}
pub struct PodState {
    inner: ListState,
}

impl PodState {
    pub fn select(&mut self, index: Option<usize>) {
        self.inner.select(index);
    }

    pub fn selected(&self) -> Option<usize> {
        self.inner.selected()
    }

    pub fn state(&mut self) -> &mut ListState {
        &mut self.inner
    }
}
impl Default for PodState {
    fn default() -> Self {
        Self {
            inner: ListState::default(),
        }
    }
}

impl<'a> Pods<'a> {
    pub fn new(items: Vec<String>) -> Self {
        let mut state = PodState::default();
        if 0 < items.len() {
            state.select(Some(0));
        }

        Self {
            items,
            state: Rc::new(RefCell::new(state)),
            list_item: Vec::new(),
        }
    }
    pub fn unselect(&self) {
        self.state.borrow_mut().select(None);
    }

    pub fn selected(&self) -> Option<usize> {
        self.state.borrow().selected()
    }

    pub fn select_first(&self) {
        self.state.borrow_mut().select(Some(0));
    }

    pub fn select_last(&self) {
        let last_index = self.items.len() - 1;
        self.state.borrow_mut().select(Some(last_index));
    }

    pub fn state(&self) -> Rc<RefCell<PodState>> {
        Rc::clone(&self.state)
    }

    pub fn next(&self) {
        let i = match self.state.borrow().selected() {
            Some(i) => {
                if self.items.len() - 1 <= i {
                    self.items.len() - 1
                } else {
                    i + 1
                }
            }
            None => 0,
        };

        self.state.borrow_mut().select(Some(i));
    }

    pub fn prev(&self) {
        let i = match self.state.borrow().selected() {
            Some(i) => {
                if i == 0 {
                    0
                } else {
                    i - 1
                }
            }
            None => 0,
        };

        self.state.borrow_mut().select(Some(i));
    }

    pub fn set_items(&mut self, items: Vec<String>) {
        match items.len() {
            0 => self.state.borrow_mut().select(None),
            len if len < self.items.len() => self.state.borrow_mut().select(Some(len - 1)),
            _ => {}
        }
        self.items = items;

        self.set_listitem();
    }

    fn add_item(&mut self, item: impl Into<String>) {
        let item: String = item.into();
        self.items.push(item.clone());
        self.add_listitem(item);
    }

    pub fn items(&self) -> &Vec<String> {
        &self.items
    }

    pub fn list(&self, block: Block<'a>) -> List<'a> {
        List::new(self.list_item.clone())
            .block(block)
            .style(Style::default())
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
    }

    fn set_listitem(&mut self) {
        self.list_item = self
            .items
            .iter()
            .cloned()
            .map(|i| ListItem::new(i))
            .collect();
    }
    fn add_listitem(&mut self, item: String) {
        self.list_item.push(ListItem::new(item));
    }
}

impl WidgetTrait for Pods<'_> {
    fn selectable(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_index() {
        let list = Pods::new(vec![
            String::from("Item 0"),
            String::from("Item 1"),
            String::from("Item 2"),
        ]);

        assert_eq!(Some(0), list.selected())
    }

    #[test]
    fn two_prev_is_selected_last_index() {
        let list = Pods::new(vec![
            String::from("Item 0"),
            String::from("Item 1"),
            String::from("Item 2"),
        ]);

        list.prev();
        list.prev();
        assert_eq!(Some(1), list.selected())
    }
    #[test]
    fn one_next_is_selected_second_index() {
        let list = Pods::new(vec![
            String::from("Item 0"),
            String::from("Item 1"),
            String::from("Item 2"),
        ]);

        list.next();
        assert_eq!(Some(1), list.selected())
    }

    #[test]
    fn last_next_is_selected_first_index() {
        let list = Pods::new(vec![
            String::from("Item 0"),
            String::from("Item 1"),
            String::from("Item 2"),
        ]);

        list.next();
        list.next();
        list.next();
        assert_eq!(Some(0), list.selected())
    }
}
