use crate::{
    event::{Callback, EventResult},
    key_event_to_code,
    util::{default_focus_block, focus_block},
    Window,
};

use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use derivative::*;

use std::rc::Rc;
use tui::{
    backend::Backend,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Cell, Row, Table as TTable, TableState},
    Frame,
};

use unicode_width::UnicodeWidthStr;

use super::spans::generate_spans_line;
use super::wrap::wrap_line;
use super::{RenderTrait, WidgetItem, WidgetTrait};

const COLUMN_SPACING: u16 = 3;
const HIGHLIGHT_SYMBOL: &str = " ";
const ROW_START_INDEX: usize = 2;

type InnerCallback = Rc<dyn Fn(&mut Window, &[String]) -> EventResult>;

#[derive(Debug, Default)]
struct InnerItemBuilder {
    header: Vec<String>,
    rows: Vec<Vec<String>>,
    max_width: usize,
}

impl InnerItemBuilder {
    fn header(mut self, header: impl Into<Vec<String>>) -> Self {
        self.header = header.into();
        self
    }

    fn rows(mut self, rows: impl Into<Vec<Vec<String>>>) -> Self {
        self.rows = rows.into();
        self
    }

    fn max_width(mut self, max_width: usize) -> Self {
        self.max_width = max_width;
        self
    }

    fn build(self) -> InnerItem<'static> {
        let mut inner_item = InnerItem {
            header: self.header,
            rows: self.rows,
            ..Default::default()
        };

        inner_item.header_row = Row::new(
            inner_item
                .header
                .iter()
                .cloned()
                .map(|h| Cell::from(h).style(Style::default().fg(Color::DarkGray))),
        )
        .bottom_margin(1);

        inner_item.update_rows(self.max_width);

        inner_item
    }
}

#[derive(Debug, Default, Clone)]
struct InnerRow<'a> {
    row: Row<'a>,
    height: usize,
}

#[derive(Debug, Default)]
struct InnerItem<'a> {
    header: Vec<String>,
    header_row: Row<'a>,
    rows: Vec<Vec<String>>,
    widget_rows: Vec<InnerRow<'a>>,
    bottom_margin: u16,
    digits: Vec<usize>,
    max_width: usize,
}

impl<'a> InnerItem<'a> {
    fn len(&self) -> usize {
        self.rows.len()
    }

    fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    fn update_item(&mut self, item: WidgetItem) {
        self.rows = item.double_array();
        self.inner_update_rows();
    }

    fn update_rows(&mut self, max_width: usize) {
        self.max_width = max_width;

        self.inner_update_rows();
    }

    fn inner_update_rows(&mut self) {
        self.update_digits();
        self.inner_update_widget_rows();
    }

    fn inner_update_widget_rows(&mut self) {
        if self.digits.is_empty() {
            return;
        }

        let mut margin = 0;

        self.widget_rows = self
            .rows
            .iter()
            .map(|row| {
                let cells: Vec<(Cell, usize)> = row
                    .iter()
                    .cloned()
                    .enumerate()
                    .map(|(i, cell)| {
                        let wrapped = wrap_line(&cell, self.digits[i]);

                        let mut height = 1;
                        if height < wrapped.len() {
                            height = wrapped.len();
                            margin = 1;
                        }

                        (Cell::from(generate_spans_line(&wrapped)), height)
                    })
                    .collect();

                let height = if let Some((_, h)) = cells.iter().max_by_key(|(_, h)| h) {
                    *h
                } else {
                    1
                };

                let cells = cells.into_iter().map(|(c, _)| c);

                InnerRow {
                    row: Row::new(cells).height(height as u16),
                    height,
                }
            })
            .collect();

        if margin == 1 {
            self.widget_rows = self
                .widget_rows
                .iter()
                .cloned()
                .map(|r| InnerRow {
                    row: r.row.bottom_margin(margin),
                    ..r
                })
                .collect();

            self.bottom_margin = 1;
        } else {
            self.bottom_margin = 0;
        }
    }

    fn update_digits(&mut self) {
        if self.rows.is_empty() {
            return;
        }

        self.digits = if self.header.is_empty() {
            self.rows[0].iter().map(|i| i.width()).collect()
        } else {
            self.header.iter().map(|h| h.width()).collect()
        };

        for row in &self.rows {
            for (i, col) in row.iter().enumerate() {
                let len = col.len();
                if self.digits.len() < i {
                    break;
                }

                if self.digits[i] < len {
                    self.digits[i] = len
                }
            }
        }

        let sum_width = self.digits.iter().sum::<usize>()
            + (COLUMN_SPACING as usize * self.digits.len().saturating_sub(1));

        if self.max_width < sum_width {
            let index_of_long_digits = self
                .digits
                .iter()
                .enumerate()
                .max_by_key(|(_, l)| *l)
                .unwrap_or((0, &0))
                .0;

            let sum_width: usize = self
                .digits
                .iter()
                .enumerate()
                .filter_map(|(i, w)| {
                    if i == index_of_long_digits {
                        None
                    } else {
                        Some(w)
                    }
                })
                .sum();

            self.digits[index_of_long_digits] = self.max_width.saturating_sub(
                (COLUMN_SPACING as usize * self.digits.len().saturating_sub(1)) + sum_width,
            );
        }
    }
}

#[derive(Debug, Default)]
pub struct TableBuilder {
    id: String,
    title: String,
    header: Vec<String>,
    items: Vec<Vec<String>>,
}

impl TableBuilder {
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn items(mut self, items: impl Into<Vec<Vec<String>>>) -> Self {
        self.items = items.into();
        self
    }

    pub fn header(mut self, header: impl Into<Vec<String>>) -> Self {
        self.header = header.into();
        self
    }

    pub fn build(self) -> Table<'static> {
        let mut table = Table {
            id: self.id,
            title: self.title,
            ..Default::default()
        };

        table.items = InnerItemBuilder::default()
            .header(self.header)
            .rows(self.items)
            .build();

        table.update_row_bounds();

        table
    }
}

#[derive(Derivative)]
#[derivative(Debug, Default)]
pub struct Table<'a> {
    id: String,
    title: String,
    chunk_index: usize,
    items: InnerItem<'a>,
    state: TableState,
    chunk: Rect,
    inner_chunk: Rect,
    row_bounds: Vec<(usize, usize)>,
    #[derivative(Debug = "ignore")]
    on_select: Option<InnerCallback>,
}

impl<'a> Table<'a> {
    pub fn items(&self) -> &[Vec<String>] {
        &self.items.rows
    }

    pub fn state(&self) -> &TableState {
        &self.state
    }

    pub fn equal_header(&self, header: &[String]) -> bool {
        self.items.header == header
    }

    fn max_width(&self) -> usize {
        self.inner_chunk.width.saturating_sub(2) as usize
    }

    pub fn update_header_and_rows(&mut self, header: &[String], rows: &[Vec<String>]) {
        self.items = InnerItemBuilder::default()
            .header(header)
            .rows(rows)
            .max_width(self.max_width())
            .build();

        self.update_widget_item(WidgetItem::DoubleArray(rows.to_vec()));
    }

    fn update_row_bounds(&mut self) {
        let bottom_margin = self.items.bottom_margin as usize;
        self.row_bounds = self
            .items
            .widget_rows
            .iter()
            .scan(0, |sum, row| {
                let b = (*sum, *sum + row.height.saturating_sub(1));
                *sum += row.height + bottom_margin;
                Some(b)
            })
            .collect();
    }
}

impl WidgetTrait for Table<'_> {
    fn focusable(&self) -> bool {
        true
    }

    fn select_next(&mut self, index: usize) {
        let i = match self.state.selected() {
            Some(i) => {
                if self.items.len().saturating_sub(1) <= i + index {
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
        let i = self.state.selected().unwrap_or(0);

        self.state.select(Some(i.saturating_sub(index)));
    }

    fn select_first(&mut self) {
        self.state.select(Some(0))
    }

    fn select_last(&mut self) {
        if self.items.is_empty() {
            self.state.select(Some(0));
        } else {
            self.state.select(Some(self.items.len() - 1))
        }
    }

    fn update_widget_item(&mut self, items: WidgetItem) {
        self.items.update_item(items);

        match self.items.len() {
            0 => self.state.select(None),
            len if len < self.items.len() => self.state.select(Some(len - 1)),
            _ => {
                if self.state.selected() == None {
                    self.state.select(Some(0))
                }
            }
        }

        self.update_row_bounds();
    }

    fn update_chunk(&mut self, chunk: Rect) {
        self.chunk = chunk;
        self.inner_chunk = default_focus_block().inner(chunk);

        self.items.update_rows(self.max_width());

        self.update_row_bounds();
    }

    fn clear(&mut self) {
        self.state = TableState::default();
        self.items = InnerItemBuilder::default()
            .max_width(self.max_width())
            .build();
        self.row_bounds = Vec::default();
    }

    fn widget_item(&self) -> Option<WidgetItem> {
        self.state
            .selected()
            .map(|i| WidgetItem::Array(self.items.rows[i].clone()))
    }

    fn append_widget_item(&mut self, _: WidgetItem) {}

    fn on_mouse_event(&mut self, ev: MouseEvent) -> EventResult {
        if self.items.is_empty() {
            return EventResult::Nop;
        }

        if ev.row == self.inner_chunk.bottom() {
            return EventResult::Ignore;
        }

        let (_, row) = (
            ev.column.saturating_sub(self.inner_chunk.left()) as usize,
            ev.row.saturating_sub(self.inner_chunk.top()) as usize,
        );

        match ev.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                let offset = self.state.offset();
                let offset_bound = self.row_bounds[offset];

                let header_margin = if self.items.header.is_empty() {
                    0
                } else {
                    ROW_START_INDEX
                };

                if let Some((index, _)) =
                    self.row_bounds[offset..].iter().enumerate().find(|(_, b)| {
                        let b = (
                            b.0 - offset_bound.0 + header_margin,
                            b.1 - offset_bound.1 + header_margin,
                        );

                        b.0 <= row && row <= b.1
                    })
                {
                    self.state.select(Some(index + offset));
                    return EventResult::Callback(self.on_select_callback());
                }
            }

            MouseEventKind::ScrollDown => {
                self.select_next(1);
                return EventResult::Nop;
            }
            MouseEventKind::ScrollUp => {
                self.select_prev(1);
                return EventResult::Nop;
            }
            _ => {}
        }

        EventResult::Ignore
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

    fn title(&self) -> &str {
        &self.title
    }

    fn chunk(&self) -> Rect {
        self.chunk
    }

    fn id(&self) -> &str {
        &self.id
    }
}

impl<'a> Table<'a> {
    pub fn on_select<F>(mut self, cb: F) -> Self
    where
        F: Fn(&mut Window, &[String]) -> EventResult + 'static,
    {
        self.on_select = Some(Rc::new(cb));
        self
    }

    fn on_select_callback(&self) -> Option<Callback> {
        self.on_select.clone().and_then(|cb| {
            self.selected_item()
                .map(|v| Callback::from_fn(move |w| cb(w, &v)))
        })
    }

    fn selected_item(&self) -> Option<Rc<Vec<String>>> {
        self.state
            .selected()
            .map(|i| Rc::new(self.items.rows[i].clone()))
    }
}

impl RenderTrait for Table<'_> {
    fn render<B>(&mut self, f: &mut Frame<'_, B>, selected: bool)
    where
        B: Backend,
    {
        let title = self.title().to_string();

        let constraints = constraints(&self.items.digits);

        let mut widget = TTable::new(self.items.widget_rows.iter().cloned().map(|row| row.row))
            .block(focus_block(&title, selected))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol(HIGHLIGHT_SYMBOL)
            .column_spacing(COLUMN_SPACING)
            .widths(&constraints);

        if !self.items.header.is_empty() {
            widget = widget.header(self.items.header_row.clone());
        }

        f.render_stateful_widget(widget, self.chunk, &mut self.state);
    }
}

fn constraints(digits: &[usize]) -> Vec<Constraint> {
    digits
        .iter()
        .map(|d| Constraint::Length(*d as u16))
        .collect()
}
