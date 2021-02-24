use std::cell::RefCell;
use std::rc::Rc;

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
    scroll: u16,
}
impl LogState {
    fn select(&mut self, index: u16) {
        self.scroll = index;
    }
    fn selected(&self) -> u16 {
        self.scroll
    }
}
impl Default for LogState {
    fn default() -> Self {
        Self { scroll: 0 }
    }
}
impl<'a> Logs<'a> {
    pub fn new(items: Vec<String>) -> Self {
        let paragraph = Paragraph::new(vec![Spans::default()]);

        Self {
            items,
            state: Rc::new(RefCell::new(LogState::default())),
            spans: vec![Spans::default()],
            paragraph,
        }
    }

    pub fn selected(&self) -> u16 {
        self.state.borrow().selected()
    }

    pub fn select(&self, scroll: u16) {
        self.state.borrow_mut().select(scroll);
    }

    pub fn state(&self) -> Rc<RefCell<LogState>> {
        Rc::clone(&self.state)
    }

    pub fn scroll_top(&self) {
        self.state.borrow_mut().select(0);
    }

    pub fn scroll_bottom(&self) {
        let last_index: u16 = self.items.len() as u16 - 1;
        self.state.borrow_mut().select(last_index);
    }

    pub fn next(&mut self) {
        let mut i = self.state.borrow().selected();

        if self.items.len() - 1 <= i as usize {
            i = (self.items.len() - 1) as u16;
        } else {
            i = i + 1;
        }

        self.state.borrow_mut().select(i);
        self.paragraph = self.paragraph.clone().scroll((i, 0));
    }

    pub fn prev(&mut self) {
        let mut i = self.state.borrow().selected();
        if i == 0 {
            i = 0;
        } else {
            i = i - 1;
        }
        self.state.borrow_mut().select(i);
        self.paragraph = self.paragraph.clone().scroll((i, 0));
    }

    pub fn set_items(&mut self, items: Vec<String>) {
        self.state.borrow_mut().select(0);
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
        self.state.borrow_mut().select(self.items.len() as u16 - 1);
    }

    pub fn spans(&self) -> &Vec<Spans> {
        &self.spans
    }

    pub fn paragraph(&self, block: Block<'a>) -> Paragraph<'a> {
        let scroll = self.state().borrow().selected();

        self.paragraph.clone().block(block).scroll((scroll, 0))
    }

    fn unselect(&self) {
        self.state().borrow_mut().select(0);
    }
}

impl WidgetTrait for Logs<'_> {
    fn selectable(&self) -> bool {
        true
    }
}
