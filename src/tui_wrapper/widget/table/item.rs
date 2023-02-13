use tui::{
    style::{Color, Style},
    widgets::{Cell, Row},
};

use crate::tui_wrapper::widget::{
    spans::generate_spans_line, styled_graphemes::StyledGraphemes, wrap::wrap_line, Item, TableItem,
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
pub struct InnerRow<'a> {
    pub row: Row<'a>,
    pub height: usize,
}

#[derive(Debug, Default)]
pub struct InnerItem<'a> {
    pub header: Vec<String>,
    pub header_row: Row<'a>,
    pub rows: Vec<TableItem>,
    pub widget_rows: Vec<InnerRow<'a>>,
    pub bottom_margin: u16,
    pub digits: Vec<usize>,
    pub max_width: usize,
}

impl<'a> InnerItem<'a> {
    pub fn builder() -> InnerItemBuilder {
        InnerItemBuilder::default()
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn update_item(&mut self, item: Item) {
        self.rows = item.table();
        self.inner_update_rows();
    }

    pub fn update_rows(&mut self, max_width: usize) {
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
            self.rows[0]
                .item
                .iter()
                .map(|i| i.styled_graphemes_width())
                .collect()
        } else {
            self.header
                .iter()
                .map(|h| h.styled_graphemes_width())
                .collect()
        };

        for row in &self.rows {
            for (i, col) in row.item.iter().enumerate() {
                let len = col.styled_graphemes_width();
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
