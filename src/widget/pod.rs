use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, RwLock};
use tui::widgets::ListState;

use super::WidgetTrait;

pub struct Pods {
    items: Vec<String>,
    state: Rc<RefCell<PodState>>,
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

impl Pods {
    pub fn new(items: Vec<String>) -> Self {
        let mut state = PodState::default();
        if 0 < items.len() {
            state.select(Some(0));
        }

        Self {
            items,
            state: Rc::new(RefCell::new(state)),
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
    }

    fn add_item(&mut self, item: impl Into<String>) {
        self.items.push(item.into());
    }

    pub fn items(&self) -> &Vec<String> {
        &self.items
    }
}

impl WidgetTrait for Pods {
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
