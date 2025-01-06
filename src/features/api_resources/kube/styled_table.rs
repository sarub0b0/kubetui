use std::fmt::Display;

use ratatui::style::Style;

use crate::{kube::apis::v1_table::Table, ui::widget::ansi_color::style_to_ansi};

pub struct StyledTable<'a> {
    table: &'a Table,
    header_style: Style,
    rows_style: Style,
}

impl<'a> StyledTable<'a> {
    pub fn new(table: &'a Table, header_style: Style, rows_style: Style) -> Self {
        Self {
            table,
            header_style,
            rows_style,
        }
    }
}

const WIDTH_PADDING: &str = "   ";

impl Display for StyledTable<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.table.rows.is_empty() || self.table.column_definitions.is_empty() {
            return Ok(());
        }

        // priorityが0のヘッダーを抜き出す
        let header: Vec<(usize, &str)> = self
            .table
            .column_definitions
            .iter()
            .enumerate()
            .filter(|(_, c)| c.priority == 0)
            .map(|(i, c)| (i, c.name.as_ref()))
            .collect::<Vec<_>>();

        // ヘッダーに応じた列を持つ行を抜き出す
        let rows: Vec<Vec<String>> = self
            .table
            .rows
            .iter()
            .map(|row| {
                header
                    .iter()
                    .map(|(i, _)| row.cells[*i].to_string())
                    .collect()
            })
            .collect();

        // 各行ごとの文字幅を決定するdigitsを計算
        // 1. ヘッダーの文字列長を取得
        let mut digits: Vec<usize> = header.iter().map(|(_, hdr)| hdr.len()).collect();

        // 2. 各ヘッダーの文字列長と列の文字列長を比較して大きいほうで文字列長を更新
        rows.iter().for_each(|cells| {
            cells.iter().enumerate().for_each(|(i, cell)| {
                if digits[i] < cell.len() {
                    digits[i] = cell.len()
                }
            });
        });

        // ヘッダーを表示
        write!(f, "{}", style_to_ansi(self.header_style))?;

        let header_str = header
            .iter()
            .map(|(i, hdr)| format!("{:<digit$}", hdr.to_uppercase(), digit = digits[*i]))
            .collect::<Vec<String>>()
            .join(WIDTH_PADDING);

        write!(f, "{}\x1b[39m", header_str)?;

        // 行を表示
        for row in rows.iter() {
            write!(f, "\n{}", style_to_ansi(self.rows_style))?;

            let rows_str = row
                .iter()
                .enumerate()
                .map(|(i, cell)| format!("{:<digit$}", cell, digit = digits[i]))
                .collect::<Vec<String>>()
                .join(WIDTH_PADDING);

            write!(f, "{}\x1b[39m", rows_str)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::kube::apis::v1_table::{TableColumnDefinition, TableRow};
    use indoc::indoc;
    use pretty_assertions::assert_eq;
    use ratatui::style::Color;

    use super::*;

    // テーブルデータが空の場合は何も表示しない
    #[test]
    fn when_table_is_empty_then_display_nothing() {
        let table = Table {
            column_definitions: vec![],
            rows: vec![],
            ..Default::default()
        };

        let styled_table = StyledTable::new(&table, Style::default(), Style::default());

        assert_eq!(styled_table.to_string(), "");
    }

    // テーブルデータが存在する場合はヘッダーと行を表示する
    #[test]
    fn when_table_has_data_then_display_header_and_rows() {
        let table = Table {
            column_definitions: vec![
                TableColumnDefinition {
                    name: "Name".into(),
                    priority: 0,
                    ..Default::default()
                },
                TableColumnDefinition {
                    name: "Age".into(),
                    priority: 0,
                    ..Default::default()
                },
            ],
            rows: vec![
                TableRow {
                    cells: ["Alice", "20"].into_iter().map(|s| s.into()).collect(),
                    ..Default::default()
                },
                TableRow {
                    cells: ["Bob", "30"].into_iter().map(|s| s.into()).collect(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        let styled_table = StyledTable::new(
            &table,
            Style::default().fg(Color::DarkGray),
            Style::default().fg(Color::White),
        );

        let expected = indoc! {"
            \x1b[90mNAME    AGE\x1b[39m
            \x1b[37mAlice   20 \x1b[39m
            \x1b[37mBob     30 \x1b[39m
        "};

        assert_eq!(styled_table.to_string(), expected.trim_end());
    }
}
