use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, RwLock};

use tui::style::Style;
use tui::text::Spans;
use tui::widgets::{Block, Paragraph, Wrap};

use super::WidgetTrait;

pub struct Logs<'a> {
    items: Vec<String>,
    state: Rc<RefCell<LogState>>,
    spans: Vec<Spans<'a>>,
    paragraph: Paragraph<'a>,
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
impl<'a> Logs<'a> {
    pub fn new(items: Vec<String>) -> Self {
        let mut state = LogState::default();
        if 0 < items.len() {
            state.select(Some(0));
        }

        let paragraph = Paragraph::new(vec![Spans::default()]);

        Self {
            items,
            state: Rc::new(RefCell::new(state)),
            spans: vec![Spans::default()],
            paragraph,
        }
    }

    pub fn selected(&self) -> Option<u16> {
        self.state.borrow().selected()
    }

    pub fn select(&self, scroll: Option<u16>) {
        self.state.borrow_mut().select(scroll)
    }

    pub fn state(&self) -> Rc<RefCell<LogState>> {
        Rc::clone(&self.state)
    }

    pub fn scroll_top(&self) {
        self.state.borrow_mut().select(Some(0));
    }

    pub fn scroll_bottom(&self) {
        let last_index: u16 = self.items.len() as u16 - 1;
        self.state.borrow_mut().select(Some(last_index));
    }

    pub fn next(&mut self) {
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
        self.paragraph = self.paragraph.clone().scroll((i, 0));
    }

    pub fn prev(&mut self) {
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
        self.paragraph = self.paragraph.clone().scroll((i, 0));
    }

    pub fn set_items(&mut self, items: Vec<String>) {
        match items.len() {
            0 => self.state.borrow_mut().select(None),
            len if len < self.items.len() => self.state.borrow_mut().select(Some(len as u16 - 1)),
            _ => {}
        }
        self.items = items.clone();

        self.spans = items.iter().cloned().map(Spans::from).collect();
        self.paragraph = Paragraph::new(self.spans.clone())
            .style(Style::default())
            .wrap(Wrap { trim: false });
    }

    pub fn items(&self) -> &Vec<String> {
        &self.items
    }

    pub fn add_item(&mut self, item: &String) {
        self.items.push(item.clone());
        // self.state.borrow_mut().select(Some(self.items.len() - 1));
    }

    pub fn spans(&self) -> &Vec<Spans> {
        &self.spans
    }

    pub fn paragraph(&self, block: Block<'a>) -> Paragraph<'a> {
        let scroll = match self.state().borrow().selected() {
            Some(scroll) => scroll,
            None => 0,
        };

        self.paragraph.clone().block(block)
    }

    fn unselect(&self) {
        self.state.borrow_mut().select(None);
    }
}

impl WidgetTrait for Logs<'_> {
    fn selectable(&self) -> bool {
        true
    }
}
