use ratatui::widgets::{Cell, Row};
use std::ops::Deref;

use crate::{
    logger,
    ui::widget::{
        line::convert_lines_to_styled_lines,
        styled_graphemes::StyledGraphemes,
        wrap::wrap_line,
        TableItem,
    },
};

use super::COLUMN_SPACING;

const HEADER_BOTTOM_MARGIN: u16 = 1;
const ITEM_BOTTOM_MARGIN: u16 = 1;

#[derive(Debug, Default)]
pub struct InnerItemBuilder {
    header: Vec<String>,
    items: Vec<TableItem>,
    max_width: usize,
    filtered_key: String,
}

impl InnerItemBuilder {
    pub fn header(mut self, header: impl Into<Vec<String>>) -> Self {
        self.header = header.into();
        self
    }

    pub fn items(mut self, items: impl Into<Vec<TableItem>>) -> Self {
        self.items = items.into();
        self
    }

    pub fn max_width(mut self, max_width: usize) -> Self {
        self.max_width = max_width;
        self
    }

    pub fn filtered_key(mut self, key: impl Into<String>) -> Self {
        self.filtered_key = key.into();
        self
    }

    pub fn build(self) -> InnerItem<'static> {
        let mut inner_item = InnerItem {
            header: Header::new(self.header),
            original_items: self.items.clone(),
            filtered_items: self.items,
            filtered_key: self.filtered_key,
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
    filtered_items: Vec<TableItem>,
    rendered_items: Vec<InnerRow<'a>>,
    item_margin: u16,
    digits: Digits,
    max_width: usize,
    filtered_key: String,
    filtered_word: String,
}

impl InnerItem<'_> {
    pub fn builder() -> InnerItemBuilder {
        InnerItemBuilder::default()
    }

    pub fn len(&self) -> usize {
        self.filtered_items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.filtered_items.is_empty()
    }

    pub fn header(&self) -> &Header<'_> {
        &self.header
    }

    pub fn items(&self) -> &[TableItem] {
        &self.filtered_items
    }

    pub fn rendered_items(&self) -> &[InnerRow<'_>] {
        &self.rendered_items
    }

    pub fn to_rendered_rows(&self) -> Vec<Row<'_>> {
        self.rendered_items.iter().cloned().map(|i| i.row).collect()
    }

    pub fn digits(&self) -> &[usize] {
        &self.digits
    }

    pub fn item_margin(&self) -> u16 {
        self.item_margin
    }

    pub fn update_items(&mut self, item: Vec<TableItem>) {
        self.original_items = item;
        self.inner_filter_items();
        self.inner_update_rendered_items();
    }

    pub fn update_max_width(&mut self, max_width: usize) {
        self.max_width = max_width;
        self.inner_update_rendered_items();
    }

    pub fn update_filter(&mut self, word: impl Into<String>) {
        self.filtered_word = word.into();
        self.inner_filter_items();
        self.inner_update_rendered_items();
    }
}

impl InnerItem<'_> {
    fn inner_filter_items(&mut self) {
        self.filtered_items = if self.filtered_word.is_empty() {
            self.original_items.clone()
        } else {
            self.original_items
                .iter()
                .filter_map(|item| {
                    let choice = item.item[self.filtered_index()]
                        .styled_graphemes_symbols()
                        .concat();

                    if self
                        .filtered_word
                        .split(' ')
                        .any(|pattern| choice.contains(pattern))
                    {
                        Some(item.clone())
                    } else {
                        None
                    }
                })
                .collect()
        }
    }

    fn inner_update_rendered_items(&mut self) {
        self.digits = Digits::new(&self.filtered_items, &self.header.original, self.max_width);

        if self.digits.is_empty() {
            return;
        }

        let mut need_margin = false;

        self.rendered_items = self
            .filtered_items
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

                        Cell::from(convert_lines_to_styled_lines(&wrapped))
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
                .map(|r| {
                    InnerRow {
                        row: r.row.bottom_margin(ITEM_BOTTOM_MARGIN),
                        ..r
                    }
                })
                .collect();

            self.item_margin = ITEM_BOTTOM_MARGIN;
        } else {
            self.item_margin = 0;
        }
    }

    fn filtered_index(&self) -> usize {
        let index = self
            .header
            .original
            .iter()
            .position(|header| header == &self.filtered_key)
            .unwrap_or(0);

        logger!(
            debug,
            "[table] header={:?} filtered_key={}",
            self.header.original,
            index
        );

        index
    }
}

#[derive(Debug, Default)]
pub struct Header<'a> {
    original: Vec<String>,
    rendered: Row<'a>,
}

impl Header<'_> {
    fn new(header: Vec<String>) -> Self {
        let rendered = Row::new(
            header
                .iter()
                .cloned()
                .map(|h| Cell::from(h.styled_graphemes_symbols().concat())),
        )
        .bottom_margin(HEADER_BOTTOM_MARGIN);

        Self {
            original: header,
            rendered,
        }
    }

    pub fn original(&self) -> &[String] {
        &self.original
    }

    pub fn rendered(&self) -> Row<'_> {
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

        // Width available for cell content, i.e. excluding the spacing drawn
        // between columns.
        let spacing = COLUMN_SPACING as usize * digits.len().saturating_sub(1);
        let content_budget = max_width.saturating_sub(spacing);

        // Shrink the widest column first, down to MIN_COLUMN_WIDTH if needed,
        // before touching the next-widest. This truncates long values
        // (typically the NAME column) first while keeping shorter columns at
        // their natural width as long as possible. A column is never reduced to
        // 0 (which would make it vanish), and the total always fits the budget
        // (so ratatui does not clip a whole column off the right edge).
        const MIN_COLUMN_WIDTH: usize = 1;

        let mut overflow = digits.iter().sum::<usize>().saturating_sub(content_budget);
        while overflow > 0 {
            let Some(idx) = digits
                .iter()
                .enumerate()
                .filter(|(_, &w)| w > MIN_COLUMN_WIDTH)
                .max_by_key(|(_, &w)| w)
                .map(|(i, _)| i)
            else {
                // Every column is already at the minimum width; the pane is too
                // narrow to fit them all and nothing more can be shrunk.
                break;
            };

            let reducible = digits[idx] - MIN_COLUMN_WIDTH;
            let reduce = reducible.min(overflow);
            digits[idx] -= reduce;
            overflow -= reduce;
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

#[cfg(test)]
mod tests {
    use super::*;

    mod filtered_index {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn headerにfiltered_keyに一致する要素があるとき要素のインデックスを返す() {
            let item = InnerItem::builder()
                .header(
                    ["FOO", "BAR", "BAZ"]
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>(),
                )
                .filtered_key("BAR")
                .build();

            let actual = item.filtered_index();

            assert_eq!(actual, 1);
        }

        #[test]
        fn headerにfiltered_keyに一致する要素がないとき0を返す() {
            let item = InnerItem::builder()
                .header(
                    ["FOO", "BAR", "BAZ"]
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>(),
                )
                .filtered_key("HOGE")
                .build();

            let actual = item.filtered_index();

            assert_eq!(actual, 0);
        }
    }

    mod digits {
        use super::*;
        use pretty_assertions::assert_eq;

        fn items(cells: &[&str]) -> Vec<TableItem> {
            vec![TableItem::new(
                cells.iter().map(ToString::to_string).collect::<Vec<_>>(),
                None,
            )]
        }

        fn header(cols: &[&str]) -> Vec<String> {
            cols.iter().map(ToString::to_string).collect()
        }

        fn total(digits: &Digits) -> usize {
            digits.iter().sum::<usize>() + COLUMN_SPACING as usize * digits.len().saturating_sub(1)
        }

        #[test]
        fn 十分な幅では自然幅をそのまま使う() {
            let h = header(&["NAME", "ZONE", "STATUS"]);
            let it = items(&["node-abcdefgh", "", "Ready"]);

            let digits = Digits::new(&it, &h, usize::MAX);

            // NAME=13, ZONE=ヘッダ4, STATUS=ヘッダ6
            assert_eq!(*digits, vec![13, 4, 6]);
        }

        #[test]
        fn 狭い幅でもどの列も0に潰れずクリップされない() {
            let h = header(&["NAME", "ZONE", "STATUS"]);
            let it = items(&["gke-very-long-node-name-0123456789", "", "Ready"]);

            let digits = Digits::new(&it, &h, 16);

            assert!(
                digits.iter().all(|&w| w >= 1),
                "どの列も0幅にならないこと: {:?}",
                *digits
            );
            assert!(
                total(&digits) <= 16,
                "合計が max_width に収まること: total={} digits={:?}",
                total(&digits),
                *digits
            );
        }

        #[test]
        fn 縮小は最長列から行い短い列は可能な限り残す() {
            let h = header(&["NAME", "ZONE", "STATUS"]);
            let it = items(&["gke-very-long-node-name-0123456789", "", "Ready"]);

            let digits = Digits::new(&it, &h, 20);

            // 最長の NAME 列が縮められ、自然幅(34)より小さくなる。
            assert!(digits[0] < 34, "最長列が縮小されること: {:?}", *digits);
            // 短い列(ZONE=4, STATUS=6)は最長列より先に削られない。
            assert_eq!(digits[1], 4, "ZONE は保持: {:?}", *digits);
            assert_eq!(digits[2], 6, "STATUS は保持: {:?}", *digits);
            assert!(total(&digits) <= 20);
        }
    }
}
