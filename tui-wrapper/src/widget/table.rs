use super::{RenderTrait, WidgetItem, WidgetTrait};

use std::cell::RefCell;
use std::rc::Rc;

use tui::{
    backend::Backend,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Cell, Row, Table as TTable, TableState},
    Frame,
};

#[derive(Debug, Clone)]
pub struct Table<'a> {
    items: Vec<Vec<String>>,
    header: Vec<String>,
    state: Rc<RefCell<TableState>>,
    rows: Vec<Row<'a>>,
    widths: Vec<Constraint>,
}

impl Default for Table<'_> {
    fn default() -> Self {
        Self {
            items: Default::default(),
            header: Default::default(),
            state: Default::default(),
            rows: Default::default(),
            widths: Default::default(),
        }
    }
}

impl<'a> Table<'a> {
    pub fn new(items: Vec<Vec<String>>, header: Vec<String>) -> Self {
        let mut state = TableState::default();
        if !items.is_empty() {
            state.select(Some(0))
        }

        let mut table = Self {
            items,
            header,
            state: Rc::new(RefCell::new(state)),
            ..Default::default()
        };

        table.set_rows();
        table.set_widths();

        table
    }

    pub fn items(&self) -> &Vec<Vec<String>> {
        &self.items
    }

    pub fn next(&mut self) {
        self.select_next(1)
    }
    pub fn prev(&mut self) {
        self.select_prev(1)
    }

    pub fn widget(&'a self, block: Block<'a>) -> TTable<'a> {
        let header_cells = self
            .header
            .iter()
            .cloned()
            .map(|h| Cell::from(h).style(Style::default().fg(Color::DarkGray)));

        let header = Row::new(header_cells).height(1);

        TTable::new(self.rows.clone())
            .block(block)
            .header(header)
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .column_spacing(3)
            .widths(&self.widths)
    }

    pub fn state(&self) -> Rc<RefCell<TableState>> {
        self.state.clone()
    }

    fn set_widths(&mut self) {
        let mut digit_vec: Vec<usize> = self.header.iter().map(|h| h.len()).collect();

        for row in &self.items {
            for (i, col) in row.iter().enumerate() {
                let len = col.len();
                if digit_vec.len() < i {
                    break;
                }

                if digit_vec[i] < len {
                    digit_vec[i] = len
                }
            }
        }

        self.widths = digit_vec
            .iter()
            .map(|d| Constraint::Length(*d as u16))
            .collect()
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
        let items = items.double_array();

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

    fn get_item(&self) -> Option<WidgetItem> {
        let index = self.state.borrow().selected();
        match index {
            Some(i) => Some(WidgetItem::Array(self.items[i].clone())),
            None => None,
        }
    }
}

impl RenderTrait for Table<'_> {
    fn render<B>(&mut self, f: &mut Frame<'_, B>, block: Block, chunk: Rect)
    where
        B: Backend,
    {
        let header_cells = self
            .header
            .iter()
            .cloned()
            .map(|h| Cell::from(h).style(Style::default().fg(Color::DarkGray)));

        let header = Row::new(header_cells).height(1);

        let widget = TTable::new(self.rows.clone())
            .block(block)
            .header(header)
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .column_spacing(3)
            .widths(&self.widths);

        f.render_stateful_widget(widget, chunk, &mut self.state.borrow_mut());
    }
}
