use std::collections::HashMap;

use regex::Regex;

use crate::{features::node::node_columns::NodeLabelColumn, ui::widget::TableFilterPredicate};

/// Parse a Node-filter input string into a `TableFilterPredicate`.
///
/// `label_registry` supplies the set of valid label-column headers (in
/// addition to the builtin Node column headers). Unknown column names
/// produce a parse error.
pub fn parse_node_filter(
    input: &str,
    label_registry: &[NodeLabelColumn],
) -> Result<TableFilterPredicate, String> {
    let _ = label_registry; // used by later tasks
    let trimmed = input.trim();

    let mut column_includes: HashMap<String, Vec<Regex>> = HashMap::new();
    if !trimmed.is_empty() {
        let regexes: Result<Vec<Regex>, _> =
            trimmed.split_whitespace().map(Regex::new).collect();
        let regexes = regexes.map_err(|e| format!("invalid regex: {}", e))?;
        column_includes.insert("name".to_string(), regexes);
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
}
