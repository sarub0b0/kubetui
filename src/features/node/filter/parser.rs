use std::collections::HashMap;

use regex::Regex;

use crate::{features::node::node_columns::NodeLabelColumn, ui::widget::TableFilterPredicate};

/// One parsed term from the input.
#[derive(Debug)]
enum Term {
    /// Bare value (no prefix) → defaults to NAME include.
    Bare(String),
    /// `<col>:<value>` include.
    Include { column: String, value: String },
}

fn parse_term(token: &str) -> Term {
    if let Some((col, val)) = token.split_once(':') {
        // Empty column or empty value is treated as Bare so the user sees
        // a regex error later (or no-op). Stricter validation happens in
        // Task 5 (column-name validation).
        if !col.is_empty() && !val.is_empty() {
            return Term::Include {
                column: col.to_lowercase(),
                value: val.to_string(),
            };
        }
    }
    Term::Bare(token.to_string())
}

/// Parse a Node-filter input string into a `TableFilterPredicate`.
///
/// `label_registry` supplies the set of valid label-column headers (in
/// addition to the builtin Node column headers). Unknown column names
/// produce a parse error.
// Consumed by the node_filter_applicator factory in Task 6 (this PR).
#[allow(dead_code)]
pub fn parse_node_filter(
    input: &str,
    label_registry: &[NodeLabelColumn],
) -> Result<TableFilterPredicate, String> {
    let _ = label_registry; // consumed in Task 5

    let trimmed = input.trim();
    let mut column_includes: HashMap<String, Vec<Regex>> = HashMap::new();

    for token in trimmed.split_whitespace() {
        match parse_term(token) {
            Term::Bare(v) => {
                let rx = Regex::new(&v).map_err(|e| format!("invalid regex '{}': {}", v, e))?;
                column_includes
                    .entry("name".to_string())
                    .or_default()
                    .push(rx);
            }
            Term::Include { column, value } => {
                let rx =
                    Regex::new(&value).map_err(|e| format!("invalid regex '{}': {}", value, e))?;
                column_includes.entry(column).or_default().push(rx);
            }
        }
    }

    Ok(TableFilterPredicate {
        column_includes,
        column_excludes: HashMap::new(),
        label_selector: None,
        raw: trimmed.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn no_label_cols() -> Vec<NodeLabelColumn> {
        Vec::new()
    }

    #[test]
    fn empty_input_yields_empty_predicate() {
        let p = parse_node_filter("", &no_label_cols()).unwrap();
        assert!(p.column_includes.is_empty());
        assert!(p.column_excludes.is_empty());
        assert_eq!(p.label_selector, None);
        assert_eq!(p.raw, "");
    }

    #[test]
    fn whitespace_only_input_yields_empty_predicate() {
        let p = parse_node_filter("   \t  ", &no_label_cols()).unwrap();
        assert!(p.column_includes.is_empty());
        assert_eq!(p.raw, "");
    }

    #[test]
    fn single_bare_value_becomes_name_include() {
        let p = parse_node_filter("worker", &no_label_cols()).unwrap();
        assert_eq!(p.column_includes.len(), 1);
        let patterns = p.column_includes.get("name").expect("name column");
        assert_eq!(patterns.len(), 1);
        assert!(patterns[0].is_match("gke-worker-1"));
        assert!(!patterns[0].is_match("gke-control-1"));
        assert_eq!(p.raw, "worker");
    }

    #[test]
    fn multiple_bare_values_become_name_or() {
        let p = parse_node_filter("foo bar", &no_label_cols()).unwrap();
        let patterns = p.column_includes.get("name").expect("name column");
        assert_eq!(patterns.len(), 2);
        assert_eq!(p.raw, "foo bar");
    }

    #[test]
    fn explicit_column_include_creates_column_entry() {
        let p = parse_node_filter("status:Ready", &no_label_cols()).unwrap();
        assert_eq!(p.column_includes.len(), 1);
        let patterns = p.column_includes.get("status").expect("status column");
        assert_eq!(patterns.len(), 1);
        assert!(patterns[0].is_match("Ready"));
        assert_eq!(p.raw, "status:Ready");
    }

    #[test]
    fn column_names_are_case_insensitive_canonicalized_lowercase() {
        let p = parse_node_filter("STATUS:Ready Name:worker", &no_label_cols()).unwrap();
        assert!(p.column_includes.contains_key("status"));
        assert!(p.column_includes.contains_key("name"));
    }

    #[test]
    fn same_column_includes_accumulate_in_order() {
        let p = parse_node_filter("status:Ready status:Pending", &no_label_cols()).unwrap();
        let patterns = p.column_includes.get("status").expect("status column");
        assert_eq!(patterns.len(), 2);
        assert!(patterns[0].is_match("Ready"));
        assert!(patterns[1].is_match("Pending"));
    }

    #[test]
    fn different_columns_coexist_in_predicate() {
        let p = parse_node_filter("status:Ready name:worker", &no_label_cols()).unwrap();
        assert_eq!(p.column_includes.len(), 2);
    }

    #[test]
    fn bare_and_column_includes_mix() {
        // `foo status:Ready` → NAME has `foo`, STATUS has `Ready`
        let p = parse_node_filter("foo status:Ready", &no_label_cols()).unwrap();
        assert_eq!(p.column_includes.len(), 2);
        assert_eq!(p.column_includes.get("name").unwrap().len(), 1);
        assert_eq!(p.column_includes.get("status").unwrap().len(), 1);
    }
}
