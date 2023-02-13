use std::ops::Deref;

use tui::{
    style::{Color, Style},
    widgets::{Cell, Row},
};

use crate::tui_wrapper::widget::{
    spans::generate_spans_line, styled_graphemes::StyledGraphemes, wrap::wrap_line, TableItem,
};

use super::COLUMN_SPACING;

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
            original_header: self.header,
            original_items: self.rows,
            ..Default::default()
        };

        inner_item.rendered_header = Row::new(
            inner_item
                .original_header
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
pub struct InnerRow<'a> {
    pub row: Row<'a>,
    pub height: usize,
}

#[derive(Debug, Default)]
pub struct InnerItem<'a> {
    original_header: Vec<String>,
    rendered_header: Row<'a>,
    pub original_items: Vec<TableItem>,
    pub rendered_items: Vec<InnerRow<'a>>,
    pub bottom_margin: u16,
    digits: Digits,
    pub max_width: usize,
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

    pub fn header(&self) -> &[String] {
        &self.original_header
    }

    pub fn rendered_header(&self) -> Row {
        self.rendered_header.clone()
    }

    pub fn digits(&self) -> &[usize] {
        &self.digits
    }

    pub fn update_items(&mut self, item: Vec<TableItem>) {
        self.original_items = item;
        self.inner_update_rows();
    }

    pub fn update_rows(&mut self, max_width: usize) {
        self.max_width = max_width;

        self.inner_update_rows();
    }
}

impl<'a> InnerItem<'a> {
    fn inner_update_rows(&mut self) {
        self.digits = Digits::new(&self.original_items, &self.original_header, self.max_width);

        self.inner_update_widget_rows();
    }

    fn inner_update_widget_rows(&mut self) {
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
                    row: r.row.bottom_margin(1),
                    ..r
                })
                .collect();

            self.bottom_margin = 1;
        } else {
            self.bottom_margin = 0;
        }
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
