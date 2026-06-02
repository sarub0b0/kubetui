//! Network filter parser.
//!
//! Delegates tokenization/quoting/predicate-building to the shared
//! `parse_table_filter`. The Network-specific part is the column validator:
//! `namespace:` returns a guidance message (namespace is a scope, not a
//! column-level filter — use the namespace selector); other unknown columns
//! return `unknown column '<x>'`; the builtin Network columns (`name`, `kind`,
//! `age`) are accepted.

use std::collections::HashSet;

use crate::ui::widget::{normalize_column_name, parse_table_filter, TableFilterPredicate};

/// Builtin Network columns (header form). The Network tab aggregates many
/// resource types (Service, Ingress, NetworkPolicy, Pod, Gateway, HTTPRoute)
/// into a single table; all columns are fixed in code.
const BUILTIN_COLUMNS: &[&str] = &["NAME", "KIND", "AGE"];

/// Parse a Network-filter input string into a `TableFilterPredicate`.
///
/// `namespace:` is rejected with a guidance message that points users to the
/// namespace selector (namespace is a scope, not a row attribute). This check
/// fires *before* the builtin lookup. Other columns are validated against
/// `BUILTIN_COLUMNS`; a column not in that set produces `unknown column '<x>'`.
pub fn parse_network_filter(input: &str) -> Result<TableFilterPredicate, String> {
    let valid: HashSet<String> = BUILTIN_COLUMNS
        .iter()
        .map(|c| normalize_column_name(c))
        .collect();
    parse_table_filter(input, |column| {
        let normalized = normalize_column_name(column);
        if normalized == "namespace" {
            return Err(
                "namespace is selected via the namespace selector, not the filter".to_string(),
            );
        }
        if valid.contains(&normalized) {
            Ok(())
        } else {
            Err(format!("unknown column '{}'", column))
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn empty_input_yields_empty_predicate() {
        let p = parse_network_filter("").unwrap();
        assert!(p.column_includes.is_empty());
        assert!(p.column_excludes.is_empty());
        assert_eq!(p.label_selector, None);
    }

    #[test]
    fn bare_value_becomes_name_include() {
        let p = parse_network_filter("my-svc").unwrap();
        let patterns = p.column_includes.get("name").expect("name column");
        assert!(patterns[0].is_match("my-svc-abc"));
    }

    #[test]
    fn builtin_columns_are_accepted() {
        let p = parse_network_filter("kind:Service !kind:Pod").unwrap();
        assert!(p.column_includes.contains_key("kind"));
        assert!(p.column_excludes.contains_key("kind"));
    }

    #[test]
    fn age_column_is_accepted() {
        let p = parse_network_filter("age:1d").unwrap();
        assert!(p.column_includes.contains_key("age"));
    }

    #[test]
    fn label_selector_is_captured() {
        let p = parse_network_filter("label:app=nginx").unwrap();
        assert_eq!(p.label_selector.as_deref(), Some("app=nginx"));
    }

    #[test]
    fn unknown_column_produces_parse_error() {
        let err = parse_network_filter("staus:Active").unwrap_err();
        assert!(
            err.contains("unknown column") && err.contains("staus"),
            "got: {}",
            err
        );
    }

    #[test]
    fn namespace_returns_guidance_message_not_unknown_column() {
        let err = parse_network_filter("namespace:default").unwrap_err();
        assert_eq!(
            err,
            "namespace is selected via the namespace selector, not the filter"
        );
        let err2 = parse_network_filter("NAMESPACE:default").unwrap_err();
        assert_eq!(
            err2,
            "namespace is selected via the namespace selector, not the filter"
        );
    }

    #[test]
    fn label_keyword_is_not_treated_as_a_column_lookup() {
        assert!(parse_network_filter("label:app=nginx").is_ok());
    }

    #[test]
    fn quoted_value_with_whitespace() {
        let p = parse_network_filter(r#"name:"my svc""#).unwrap();
        let patterns = p.column_includes.get("name").unwrap();
        assert!(patterns[0].is_match("my svc"));
    }

    #[test]
    fn data_column_is_not_accepted_unlike_config() {
        // Config has DATA column; Network does not. Confirm validator rejects it.
        let err = parse_network_filter("data:0").unwrap_err();
        assert!(err.contains("unknown column") && err.contains("data"));
    }
}
