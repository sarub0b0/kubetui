use super::{WidgetItem, WidgetTrait};

use std::cell::RefCell;
use std::rc::Rc;

use tui::layout::Constraint;
use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans};
use tui::widgets::{Block, Cell, Row, Table as TTable, TableState};

#[derive(Debug)]
pub struct Table<'a> {
    items: Vec<Vec<String>>,
    state: Rc<RefCell<TableState>>,
    rows: Vec<Row<'a>>,
    widths: Vec<Constraint>,
}

impl Default for Table<'_> {
    fn default() -> Self {
        Self {
            items: Default::default(),
            state: Default::default(),
            rows: Default::default(),
            widths: Default::default(),
        }
    }
}

impl<'a> Table<'a> {
    pub fn new(items: Vec<Vec<String>>) -> Self {
        let mut state = TableState::default();
        if !items.is_empty() {
            state.select(Some(0))
        }

        let mut table = Self {
            items,
            state: Rc::new(RefCell::new(state)),
            ..Default::default()
        };

        table.set_rows();
        table.set_widths();

        table
    }

    pub fn next(&mut self) {
        self.select_next(1)
    }
    pub fn prev(&mut self) {
        self.select_prev(1)
    }

    pub fn widget(&'a self, block: Block<'a>) -> TTable<'a> {
        let header_cells = ["NAME", "READY", "STATUS", "AGE"]
            .iter()
            .map(|h| Cell::from(*h).style(Style::default().fg(Color::DarkGray)));

        let header = Row::new(header_cells).height(1);

        TTable::new(self.rows.clone())
            .block(block)
            .header(header)
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .column_spacing(2)
            .widths(&self.widths)
    }

    pub fn state(&self) -> Rc<RefCell<TableState>> {
        self.state.clone()
    }

    fn set_widths(&mut self) {
        let mut d0 = 4; // NAME
        let mut d1 = 5; // READY
        let mut d2 = 6; // STATUS
        let mut d3 = 3; // AGE

        for row in &self.items {
            for (i, col) in row.iter().enumerate() {
                let len = col.len();
                match i {
                    0 => {
                        if d0 < len {
                            d0 = len;
                        }
                    }
                    1 => {
                        if d1 < len {
                            d1 = len;
                        }
                    }
                    2 => {
                        if d2 < len {
                            d2 = len;
                        }
                    }
                    3 => {
                        if d3 < len {
                            d3 = len;
                        }
                    }
                    _ => {
                        unreachable!()
                    }
                }
            }
        }

        self.widths = vec![
            Constraint::Length(d0 as u16),
            Constraint::Length(d1 as u16),
            Constraint::Length(d2 as u16),
            Constraint::Length(d3 as u16),
        ]
    }

    fn set_rows(&mut self) {
        self.rows = self
            .items
            .iter()
            .map(|i| Row::new(i.iter().cloned().map(|s| Cell::from(Span::from(s)))))
            .collect();
    }
}

impl WidgetTrait for Table<'_> {
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

        self.state.borrow_mut().select(Some(i))
    }

    fn select_prev(&mut self, index: usize) {
        let i = match self.state.borrow().selected() {
            Some(i) => {
                if i < index {
                    0
                } else {
                    i - index
                }
            }
            None => 0,
        };

        self.state.borrow_mut().select(Some(i))
    }

    fn select_first(&mut self) {
        self.state.borrow_mut().select(Some(0))
    }

    fn select_last(&mut self) {
        if self.items.is_empty() {
            self.state.borrow_mut().select(Some(0));
        } else {
            self.state.borrow_mut().select(Some(self.items.len() - 1))
        }
    }

    fn set_items(&mut self, items: WidgetItem) {
        let items = items.get_double_array();

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
        self.set_rows();
        self.set_widths()
    }

    fn update_area(&mut self, _area: tui::layout::Rect) {}

    fn clear(&mut self) {
        *self = Self::default();
    }
}
