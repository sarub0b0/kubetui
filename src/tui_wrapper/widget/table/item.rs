use std::ops::Deref;

use tui::{
    style::{Color, Style},
    widgets::{Cell, Row},
};

use crate::tui_wrapper::widget::{
    spans::generate_spans_line, styled_graphemes::StyledGraphemes, wrap::wrap_line, TableItem,
};

use super::COLUMN_SPACING;

const HEADER_BOTTOM_MARGIN: u16 = 1;
const ITEM_BOTTOM_MARGIN: u16 = 1;

#[derive(Debug, Default)]
pub struct InnerItemBuilder {
    header: Vec<String>,
    rows: Vec<TableItem>,
    max_width: usize,
}

impl InnerItemBuilder {
    pub fn header(mut self, header: impl Into<Vec<String>>) -> Self {
        self.header = header.into();
        self
    }

    pub fn rows(mut self, rows: impl Into<Vec<TableItem>>) -> Self {
        self.rows = rows.into();
        self
    }

    pub fn max_width(mut self, max_width: usize) -> Self {
        self.max_width = max_width;
        self
    }

    pub fn build(self) -> InnerItem<'static> {
        let mut inner_item = InnerItem {
            header: Header::new(self.header),
            original_items: self.rows,
            ..Default::default()
        };

        inner_item.update_max_width(self.max_width);

        inner_item
    }
}

#[derive(Debug, Default, Clone)]
pub struct InnerRow<'a> {
    pub row: Row<'a>,
    pub height: usize,
}

#[derive(Debug, Default)]
pub struct InnerItem<'a> {
    header: Header<'a>,
    original_items: Vec<TableItem>,
    rendered_items: Vec<InnerRow<'a>>,
    digits: Digits,
    max_width: usize,
    item_margin: u16,
}

impl<'a> InnerItem<'a> {
    pub fn builder() -> InnerItemBuilder {
        InnerItemBuilder::default()
    }

    pub fn len(&self) -> usize {
        self.original_items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.original_items.is_empty()
    }

    pub fn header(&self) -> &Header {
        &self.header
    }

    pub fn items(&self) -> &[TableItem] {
        &self.original_items
    }

    pub fn rendered_items(&self) -> &[InnerRow] {
        &self.rendered_items
    }

    pub fn rendered_rows(&self) -> Vec<Row> {
        self.rendered_items
            .iter()
            .cloned()
            .map(|item| item.row)
            .collect()
    }

    pub fn digits(&self) -> &[usize] {
        &self.digits
    }

    pub fn item_margin(&self) -> u16 {
        self.item_margin
    }

    pub fn update_items(&mut self, item: Vec<TableItem>) {
        self.original_items = item;
        self.inner_update_rendered_items();
    }

    pub fn update_max_width(&mut self, max_width: usize) {
        self.max_width = max_width;
        self.inner_update_rendered_items();
    }
}

impl<'a> InnerItem<'a> {
    fn inner_update_rendered_items(&mut self) {
        self.digits = Digits::new(&self.original_items, &self.header.original, self.max_width);

        if self.digits.is_empty() {
            return;
        }

        let mut need_margin = false;

        self.rendered_items = self
            .original_items
            .iter()
            .map(|row| {
                let mut row_height = 1;

                let cells: Vec<Cell> = row
                    .item
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
            self.rendered_items = self
                .rendered_items
                .iter()
                .cloned()
                .map(|r| InnerRow {
                    row: r.row.bottom_margin(ITEM_BOTTOM_MARGIN),
                    ..r
                })
                .collect();

            self.item_margin = ITEM_BOTTOM_MARGIN;
        } else {
            self.item_margin = 0;
        }
    }
}

#[derive(Debug, Default)]
pub struct Header<'a> {
    original: Vec<String>,
    rendered: Row<'a>,
}

impl Header<'_> {
    fn new(header: Vec<String>) -> Self {
        let rendered = Row::new(header.iter().cloned().map(|h| {
            Cell::from(h.styled_graphemes_symbols().concat())
                .style(Style::default().fg(Color::DarkGray))
        }))
        .bottom_margin(HEADER_BOTTOM_MARGIN);

        Self {
            original: header,
            rendered,
        }
    }

    pub fn original(&self) -> &[String] {
        &self.original
    }

    pub fn rendered(&self) -> Row {
        self.rendered.clone()
    }

    pub fn is_empty(&self) -> bool {
        self.original.is_empty()
    }
}

#[derive(Debug, Default)]
struct Digits(Vec<usize>);

impl Digits {
    fn new(items: &[TableItem], header: &[String], max_width: usize) -> Self {
        if items.is_empty() {
            return Self::default();
        }

        let mut digits: Vec<usize> = if header.is_empty() {
            items[0]
                .item
                .iter()
                .map(|i| i.styled_graphemes_width())
                .collect()
        } else {
            header.iter().map(|h| h.styled_graphemes_width()).collect()
        };

        for row in items {
            for (i, col) in row.item.iter().enumerate() {
                let len = col.styled_graphemes_width();
                if digits.len() < i {
                    break;
                }

                if digits[i] < len {
                    digits[i] = len
                }
            }
        }

        let sum_width = digits.iter().sum::<usize>()
            + (COLUMN_SPACING as usize * digits.len().saturating_sub(1));

        if max_width < sum_width {
            let index_of_longest_digits = digits
                .iter()
                .enumerate()
                .max_by_key(|(_, l)| *l)
                .unwrap_or((0, &0))
                .0;

            let sum_width: usize = digits
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

            digits[index_of_longest_digits] = max_width.saturating_sub(
                (COLUMN_SPACING as usize * digits.len().saturating_sub(1)) + sum_width,
            );
        }

        Self(digits)
    }
}

impl Deref for Digits {
    type Target = Vec<usize>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
