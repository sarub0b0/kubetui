use std::cell::RefCell;
use std::rc::Rc;

use tui::style::{Modifier, Style};

use tui::widgets::{self, Block, ListItem, ListState};

use super::WidgetTrait;

pub struct List<'a> {
    items: Vec<String>,
    state: Rc<RefCell<ListState>>,
    list_item: Vec<ListItem<'a>>,
}
impl<'a> List<'a> {
    pub fn new(items: Vec<String>) -> Self {
        let mut state = ListState::default();
        if 0 < items.len() {
            state.select(Some(0));
        }

        Self {
            items,
            state: Rc::new(RefCell::new(state)),
            list_item: Vec::new(),
        }
    }
    pub fn select_first(&self) {
        self.state.borrow_mut().select(Some(0));
    }

    pub fn select_last(&self) {
        let last_index = self.items.len() - 1;
        self.state.borrow_mut().select(Some(last_index));
    }

    pub fn state(&self) -> Rc<RefCell<ListState>> {
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

    pub fn items(&self) -> &Vec<String> {
        &self.items
    }

    pub fn selected(&self) -> Option<usize> {
        self.state.borrow().selected()
    }

    pub fn widget(&self, block: Block<'a>) -> widgets::List<'a> {
        widgets::List::new(self.list_item.clone())
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
}

impl WidgetTrait for List<'_> {
    fn selectable(&self) -> bool {
        true
    }
    fn select_next(&mut self, index: usize) {
        let i = match self.state.borrow().selected() {
            Some(i) => {
                if self.items.len() - 1 <= i {
                    self.items.len() - 1
                } else {
                    i + index
                }
            }
            None => 0,
        };
        self.state.borrow_mut().select(Some(i));
    }

    fn select_prev(&mut self, index: usize) {
        let i = match self.state.borrow().selected() {
            Some(i) => {
                if i == 0 {
                    0
                } else {
                    i - index
                }
            }
            None => 0,
        };

        self.state.borrow_mut().select(Some(i));
    }

    fn select_first(&mut self) {
        self.state.borrow_mut().select(Some(0));
    }

    fn select_last(&mut self) {
        let last_index = self.items.len() - 1;
        self.state.borrow_mut().select(Some(last_index));
    }
    fn set_items(&mut self, items: Vec<String>) {
        match items.len() {
            0 => self.state.borrow_mut().select(None),
            len if len < self.items.len() => self.state.borrow_mut().select(Some(len - 1)),
            _ => {
                if self.state.borrow().selected() == None {
                    self.state().borrow_mut().select(Some(0))
                }
            }
        }
        self.items = items;

        self.set_listitem();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_index() {
        let list = List::new(vec![
            String::from("Item 0"),
            String::from("Item 1"),
            String::from("Item 2"),
        ]);

        assert_eq!(Some(0), list.selected())
    }

    #[test]
    fn two_prev_is_selected_last_index() {
        let list = List::new(vec![
            String::from("Item 0"),
            String::from("Item 1"),
            String::from("Item 2"),
        ]);

        list.prev();
        list.prev();
        assert_eq!(Some(0), list.selected())
    }
    #[test]
    fn one_next_is_selected_second_index() {
        let list = List::new(vec![
            String::from("Item 0"),
            String::from("Item 1"),
            String::from("Item 2"),
        ]);

        list.next();
        assert_eq!(Some(1), list.selected())
    }

    #[test]
    fn last_next_is_selected_first_index() {
        let list = List::new(vec![
            String::from("Item 0"),
            String::from("Item 1"),
            String::from("Item 2"),
        ]);

        list.next();
        list.next();
        list.next();
        assert_eq!(Some(2), list.selected())
    }
}
