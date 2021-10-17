use crate::{
    event::{Callback, EventResult},
    key_event_to_code,
    util::default_focus_block,
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
use super::{config::WidgetConfig, wrap::wrap_line};
use super::{Item, RenderTrait, WidgetTrait};

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
    fn builder() -> InnerItemBuilder {
        InnerItemBuilder::default()
    }

    fn len(&self) -> usize {
        self.rows.len()
    }

    fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    fn update_item(&mut self, item: Item) {
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

        let mut need_margin = false;

        self.widget_rows = self
            .rows
            .iter()
            .map(|row| {
                let mut row_height = 1;

                let cells: Vec<Cell> = row
                    .iter()
                    .cloned()
                    .enumerate()
                    .map(|(i, cell)| {
                        let wrapped = wrap_line(&cell, self.digits[i]);

                        let wrapped_len = wrapped.len();
                        if row_height < wrapped_len {
                            need_margin = true;

                            row_height = wrapped_len;
                        }

                        Cell::from(generate_spans_line(&wrapped))
                    })
                    .collect();

                InnerRow {
                    row: Row::new(cells).height(row_height as u16),
                    height: row_height,
                }
            })
            .collect();

        if need_margin {
            self.widget_rows = self
                .widget_rows
                .iter()
                .cloned()
                .map(|r| InnerRow {
                    row: r.row.bottom_margin(1),
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
            let index_of_longest_digits = self
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
                    if i == index_of_longest_digits {
                        None
                    } else {
                        Some(w)
                    }
                })
                .sum();

            self.digits[index_of_longest_digits] = self.max_width.saturating_sub(
                (COLUMN_SPACING as usize * self.digits.len().saturating_sub(1)) + sum_width,
            );
        }
    }
}

#[derive(Derivative)]
#[derivative(Debug, Default)]
pub struct TableBuilder {
    id: String,
    widget_config: WidgetConfig,
    header: Vec<String>,
    items: Vec<Vec<String>>,
    #[derivative(Debug = "ignore")]
    on_select: Option<InnerCallback>,
}

impl TableBuilder {
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    pub fn widget_config(mut self, widget_config: &WidgetConfig) -> Self {
        self.widget_config = widget_config.clone();
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

    pub fn on_select<F>(mut self, cb: F) -> Self
    where
        F: Fn(&mut Window, &[String]) -> EventResult + 'static,
    {
        self.on_select = Some(Rc::new(cb));
        self
    }

    pub fn build(self) -> Table<'static> {
        let mut table = Table {
            id: self.id,
            widget_config: self.widget_config,
            on_select: self.on_select,
            ..Default::default()
        };

        table.items = InnerItem::builder()
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
    widget_config: WidgetConfig,
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
    pub fn builder() -> TableBuilder {
        TableBuilder::default()
    }

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
        self.items = InnerItem::builder()
            .header(header)
            .rows(rows)
            .max_width(self.max_width())
            .build();

        self.update_widget_item(Item::DoubleArray(rows.to_vec()));
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
    fn id(&self) -> &str {
        &self.id
    }

    fn focusable(&self) -> bool {
        true
    }

    fn widget_item(&self) -> Option<Item> {
        self.state
            .selected()
            .map(|i| Item::Array(self.items.rows[i].clone()))
    }

    fn chunk(&self) -> Rect {
        self.chunk
    }

    fn select_index(&mut self, _: usize) {
        todo!()
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

    fn append_widget_item(&mut self, _: Item) {
        unimplemented!()
    }

    fn update_widget_item(&mut self, items: Item) {
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
                let offset_index = self.state.offset();
                let offset_bound = self.row_bounds[offset_index];
                let offset_row = offset_bound.0;

                let header_margin = if self.items.header.is_empty() {
                    0
                } else {
                    ROW_START_INDEX
                };

                if let Some((index, _)) =
                    self.row_bounds[offset_index..]
                        .iter()
                        .enumerate()
                        .find(|(_, b)| {
                            let b = (
                                b.0.saturating_sub(offset_row) + header_margin,
                                b.1.saturating_sub(offset_row) + header_margin,
                            );

                            #[cfg(feature = "logging")]
                            log::debug!(
                                "table::on_mouse_event Mouse {:?}, row_bounds {:?} ",
                                ev,
                                b
                            );

                            b.0 <= row && row <= b.1
                        })
                {
                    self.state.select(Some(index + offset_index));
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

    fn update_chunk(&mut self, chunk: Rect) {
        self.chunk = chunk;
        self.inner_chunk = default_focus_block().inner(chunk);

        self.items.update_rows(self.max_width());

        self.update_row_bounds();
    }

    fn clear(&mut self) {
        self.state = TableState::default();
        self.items = InnerItem::builder().max_width(self.max_width()).build();
        self.row_bounds = Vec::default();

        *(self.widget_config.append_title_mut()) = None;
    }

    fn widget_config(&self) -> &WidgetConfig {
        &self.widget_config
    }

    fn widget_config_mut(&mut self) -> &mut WidgetConfig {
        &mut self.widget_config
    }
}

impl<'a> Table<'a> {
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
        let block = self
            .widget_config
            .render_block(self.focusable() && selected);

        let constraints = constraints(&self.items.digits);

        let mut widget = TTable::new(self.items.widget_rows.iter().cloned().map(|row| row.row))
            .block(block)
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
