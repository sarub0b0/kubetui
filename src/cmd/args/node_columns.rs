use std::str::FromStr as _;

use anyhow::Result;
use strum::IntoEnumIterator;

use crate::features::node::{NodeColumn, NodeColumns};

fn valid_columns() -> String {
    NodeColumn::iter()
        .map(|column| NodeColumn::normalize_column(column.as_str()))
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn parse_node_columns(input: &str) -> Result<NodeColumns> {
    let entries: Vec<&str> = input
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();

    if entries.is_empty() {
        return Err(anyhow::anyhow!("Columns list must not be empty",));
    }

    let has_full = entries
        .iter()
        .any(|e| NodeColumn::normalize_column(e) == "full");

    if has_full && entries.len() > 1 {
        return Err(anyhow::anyhow!(
            "Cannot specify 'full' with other columns. Use 'full' alone to get all columns."
        ));
    }

    if entries.len() == 1 && has_full {
        return Ok(NodeColumns::full());
    }

    let mut columns = Vec::new();

    for column in entries {
        let normalized = NodeColumn::normalize_column(column);

        if let Ok(node_column) = NodeColumn::from_str(normalized.as_str()) {
            columns.push(node_column);
        } else {
            return Err(anyhow::anyhow!(
                "Invalid column name: {}. Valid options are: {}",
                column,
                valid_columns()
            ));
        }
    }

    Ok(NodeColumns::new(columns)
        .ensure_name_column()
        .dedup_columns())
}

#[cfg(test)]
mod tests {
    use super::*;

    use pretty_assertions::assert_eq;

    #[test]
    fn parses_a_comma_separated_list() {
        let actual = parse_node_columns("name,status,roles").unwrap();
        assert_eq!(
            actual.columns(),
            &[NodeColumn::Name, NodeColumn::Status, NodeColumn::Roles]
        );
    }

    #[test]
    fn normalizes_and_accepts_wide_columns() {
        let actual = parse_node_columns("name, Internal-IP, OS_Image").unwrap();
        assert_eq!(
            actual.columns(),
            &[
                NodeColumn::Name,
                NodeColumn::InternalIP,
                NodeColumn::OSImage
            ]
        );
    }

    #[test]
    fn ensures_name_column_is_present() {
        let actual = parse_node_columns("status").unwrap();
        assert_eq!(actual.columns(), &[NodeColumn::Name, NodeColumn::Status]);
    }

    #[test]
    fn full_returns_all_columns() {
        let actual = parse_node_columns("full").unwrap();
        assert_eq!(actual.columns(), NodeColumns::full().columns());
    }

    #[test]
    fn full_with_other_columns_is_error() {
        assert!(parse_node_columns("full,status").is_err());
    }

    #[test]
    fn invalid_column_is_error() {
        assert!(parse_node_columns("bogus").is_err());
    }

    #[test]
    fn empty_is_error() {
        assert!(parse_node_columns("  ").is_err());
    }
}
