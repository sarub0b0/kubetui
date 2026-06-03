//! Config filter parser.
//!
//! Delegates tokenization/quoting/predicate-building to the shared
//! `parse_table_filter`. The Config-specific part is the column validator:
//! `namespace:` returns a guidance message (namespace is a scope, not a
//! column-level filter — use the namespace selector); other unknown columns
//! return `unknown column '<x>'`; builtin `ConfigColumn`s and registered
//! label columns (whose header appears in `label_registry`) are accepted.

use std::collections::HashSet;

use strum::IntoEnumIterator;

use crate::{
    features::config::{ConfigColumn, ConfigLabelColumn},
    ui::widget::{normalize_column_name, parse_table_filter, TableFilterPredicate},
};

fn valid_columns(label_registry: &[ConfigLabelColumn]) -> HashSet<String> {
    let mut set: HashSet<String> = ConfigColumn::iter()
        .map(|c| normalize_column_name(c.display()))
        .collect();
    for lc in label_registry {
        set.insert(normalize_column_name(&lc.header));
    }
    set
}

/// Parse a Config-filter input string into a `TableFilterPredicate`.
///
/// `namespace:` is rejected with a guidance message that points users to the
/// namespace selector. This check fires *before* the builtin / registry
/// lookup so the guidance is preserved even when a label column with header
/// "NAMESPACE" is registered. Other columns are validated against the
/// builtin `ConfigColumn` set plus any defined label columns in
/// `label_registry`.
pub fn parse_config_filter(
    input: &str,
    label_registry: &[ConfigLabelColumn],
) -> Result<TableFilterPredicate, String> {
    let valid = valid_columns(label_registry);
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

    fn no_label_cols() -> Vec<ConfigLabelColumn> {
        Vec::new()
    }

    fn registry_with(name: &str, header: &str) -> Vec<ConfigLabelColumn> {
        vec![ConfigLabelColumn {
            name: name.to_string(),
            key: "irrelevant.example.com/key".to_string(),
            header: header.to_string(),
        }]
    }

    #[test]
    fn empty_input_yields_empty_predicate() {
        let p = parse_config_filter("", &no_label_cols()).unwrap();
        assert!(p.column_includes.is_empty());
        assert!(p.column_excludes.is_empty());
        assert_eq!(p.label_selector, None);
    }

    #[test]
    fn bare_value_becomes_name_include() {
        let p = parse_config_filter("my-cm", &no_label_cols()).unwrap();
        let patterns = p.column_includes.get("name").expect("name column");
        assert!(patterns[0].is_match("my-cm-abc"));
    }

    #[test]
    fn builtin_columns_are_accepted() {
        let p = parse_config_filter("kind:ConfigMap !kind:Secret", &no_label_cols()).unwrap();
        assert!(p.column_includes.contains_key("kind"));
        assert!(p.column_excludes.contains_key("kind"));
    }

    #[test]
    fn data_and_age_columns_are_accepted() {
        let p = parse_config_filter("data:0 age:1d", &no_label_cols()).unwrap();
        assert!(p.column_includes.contains_key("data"));
        assert!(p.column_includes.contains_key("age"));
    }

    #[test]
    fn label_selector_is_captured() {
        let p = parse_config_filter("label:app=nginx", &no_label_cols()).unwrap();
        assert_eq!(p.label_selector.as_deref(), Some("app=nginx"));
    }

    #[test]
    fn unknown_column_produces_parse_error() {
        let err = parse_config_filter("staus:Active", &no_label_cols()).unwrap_err();
        assert!(err.contains("unknown column") && err.contains("staus"));
    }

    #[test]
    fn namespace_returns_guidance_message() {
        let err = parse_config_filter("namespace:default", &no_label_cols()).unwrap_err();
        assert_eq!(
            err,
            "namespace is selected via the namespace selector, not the filter"
        );
    }

    #[test]
    fn registered_label_column_header_is_accepted() {
        let regs = registry_with("app", "APP");
        let p = parse_config_filter("app:nginx", &regs).unwrap();
        assert!(p.column_includes.contains_key("app"));
    }

    #[test]
    fn namespace_guidance_precedes_registry_even_on_collision() {
        let regs = registry_with("namespace", "NAMESPACE");
        let err = parse_config_filter("namespace:default", &regs).unwrap_err();
        assert_eq!(
            err,
            "namespace is selected via the namespace selector, not the filter"
        );
    }

    #[test]
    fn quoted_value_with_whitespace() {
        let p = parse_config_filter(r#"name:"my config""#, &no_label_cols()).unwrap();
        let patterns = p.column_includes.get("name").unwrap();
        assert!(patterns[0].is_match("my config"));
    }
}
