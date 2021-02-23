use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, RwLock};

use super::WidgetTrait;

pub struct Logs {
    items: Vec<String>,
    state: Rc<RefCell<LogState>>,
}

pub struct LogState {
    scroll: Option<u16>,
}
impl LogState {
    fn select(&mut self, index: Option<u16>) {
        self.scroll = index;
    }
    fn selected(&self) -> Option<u16> {
        self.scroll
    }
}
impl Default for LogState {
    fn default() -> Self {
        Self { scroll: None }
    }
}
impl Logs {
    pub fn new(items: Vec<String>) -> Self {
        let mut state = LogState::default();
        if 0 < items.len() {
            state.select(Some(0));
        }

        Self {
            items,
            state: Rc::new(RefCell::new(state)),
        }
    }

    pub fn selected(&self) -> Option<u16> {
        self.state.borrow().selected()
    }

    pub fn state(&self) -> Rc<RefCell<LogState>> {
        Rc::clone(&self.state)
    }

    pub fn next(&self) {
        let i = match self.state.borrow().selected() {
            Some(i) => {
                if self.items.len() - 1 <= i as usize {
                    (self.items.len() - 1) as u16
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

    fn set_items(&mut self, items: Vec<String>) {
        match items.len() {
            0 => self.state.borrow_mut().select(None),
            len if len < self.items.len() => self.state.borrow_mut().select(Some(len as u16 - 1)),
            _ => {}
        }
        self.items = items;
    }

    pub fn items(&self) -> &Vec<String> {
        &self.items
    }

    pub fn add_item(&mut self, item: &String) {
        self.items.push(item.clone());
        // self.state.borrow_mut().select(Some(self.items.len() - 1));
    }

    fn unselect(&self) {
        self.state.borrow_mut().select(None);
    }
}

impl WidgetTrait for Logs {
    fn selectable(&self) -> bool {
        true
    }
}
