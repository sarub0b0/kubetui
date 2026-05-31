//! Pod filter parser.
//!
//! Delegates tokenization/quoting/predicate-building to the shared
//! `parse_table_filter`. The Pod-specific part is the column validator:
//! `namespace:` returns a guidance message (namespace is a scope, not a
//! column-level filter — use the namespace selector); other unknown columns
//! return `unknown column '<x>'`; builtin `PodColumn`s are accepted. Label
//! columns are not supported for Pod yet (future work).

use std::collections::HashSet;

use strum::IntoEnumIterator;

use crate::{
    features::pod::pod_columns::PodColumn,
    ui::widget::{normalize_column_name, parse_table_filter, TableFilterPredicate},
};

/// Parse a Pod-filter input string into a `TableFilterPredicate`.
///
/// `namespace:` is rejected with a guidance message that points users to the
/// namespace selector (namespace is a scope, not a row attribute). Other
/// columns are validated against the builtin `PodColumn` set; a column not in
/// that set produces `unknown column '<x>'`. Label columns are not supported
/// for Pod yet (future work).
pub fn parse_pod_filter(input: &str) -> Result<TableFilterPredicate, String> {
    let valid: HashSet<String> = PodColumn::iter()
        .map(|c| normalize_column_name(c.display()))
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
        let p = parse_pod_filter("").unwrap();
        assert!(p.column_includes.is_empty());
        assert!(p.column_excludes.is_empty());
        assert_eq!(p.label_selector, None);
    }

    #[test]
    fn bare_value_becomes_name_include() {
        let p = parse_pod_filter("nginx").unwrap();
        let patterns = p.column_includes.get("name").expect("name column");
        assert!(patterns[0].is_match("nginx-abc"));
    }

    #[test]
    fn builtin_columns_are_accepted() {
        let p = parse_pod_filter("status:Running !ready:0/1").unwrap();
        assert!(p.column_includes.contains_key("status"));
        assert!(p.column_excludes.contains_key("ready"));
    }

    #[test]
    fn multiword_builtin_via_normalization() {
        // NOMINATED NODE / READINESS GATES are builtin Pod columns.
        // nominatednode, nominated-node, readinessgates all accepted via normalization.
        assert!(parse_pod_filter("nominatednode:foo").is_ok());
        assert!(parse_pod_filter("nominated-node:foo").is_ok());
        assert!(parse_pod_filter("readinessgates:bar").is_ok());
    }

    #[test]
    fn label_selector_is_captured() {
        let p = parse_pod_filter("label:app=nginx").unwrap();
        assert_eq!(p.label_selector.as_deref(), Some("app=nginx"));
    }

    #[test]
    fn unknown_column_produces_parse_error() {
        let err = parse_pod_filter("staus:Running").unwrap_err();
        assert!(
            err.contains("unknown column") && err.contains("staus"),
            "got: {}",
            err
        );
    }

    #[test]
    fn namespace_returns_guidance_message_not_unknown_column() {
        let err = parse_pod_filter("namespace:default").unwrap_err();
        assert_eq!(
            err,
            "namespace is selected via the namespace selector, not the filter"
        );
        // Case / format-insensitive variants also hit the guidance.
        let err2 = parse_pod_filter("NAMESPACE:default").unwrap_err();
        assert_eq!(
            err2,
            "namespace is selected via the namespace selector, not the filter"
        );
    }

    #[test]
    fn quoted_value_with_whitespace() {
        let p = parse_pod_filter(r#"status:"CreateContainerConfigError""#).unwrap();
        let patterns = p.column_includes.get("status").unwrap();
        assert!(patterns[0].is_match("CreateContainerConfigError"));
    }
}
