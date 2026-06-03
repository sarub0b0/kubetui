//! Network filter parser.
//!
//! Delegates tokenization/quoting/predicate-building to the shared
//! `parse_table_filter`. The Network-specific part is the column validator:
//! `namespace:` returns a guidance message (namespace is a scope, not a
//! column-level filter — use the namespace selector); other unknown columns
//! return `unknown column '<x>'`; builtin `NetworkColumn`s and registered
//! label columns (whose header appears in `label_registry`) are accepted.

use std::collections::HashSet;

use strum::IntoEnumIterator;

use crate::{
    features::network::{NetworkColumn, NetworkLabelColumn},
    ui::widget::{normalize_column_name, parse_table_filter, TableFilterPredicate},
};

fn valid_columns(label_registry: &[NetworkLabelColumn]) -> HashSet<String> {
    let mut set: HashSet<String> = NetworkColumn::iter()
        .map(|c| normalize_column_name(c.display()))
        .collect();
    for lc in label_registry {
        set.insert(normalize_column_name(&lc.header));
    }
    set
}

pub fn parse_network_filter(
    input: &str,
    label_registry: &[NetworkLabelColumn],
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

    fn no_label_cols() -> Vec<NetworkLabelColumn> {
        Vec::new()
    }

    fn registry_with(name: &str, header: &str) -> Vec<NetworkLabelColumn> {
        vec![NetworkLabelColumn {
            name: name.to_string(),
            key: "irrelevant.example.com/key".to_string(),
            header: header.to_string(),
        }]
    }

    #[test]
    fn empty_input_yields_empty_predicate() {
        let p = parse_network_filter("", &no_label_cols()).unwrap();
        assert!(p.column_includes.is_empty());
        assert!(p.column_excludes.is_empty());
        assert_eq!(p.label_selector, None);
    }

    #[test]
    fn bare_value_becomes_name_include() {
        let p = parse_network_filter("my-svc", &no_label_cols()).unwrap();
        let patterns = p.column_includes.get("name").expect("name column");
        assert!(patterns[0].is_match("my-svc-abc"));
    }

    #[test]
    fn builtin_columns_are_accepted() {
        let p = parse_network_filter("kind:Service !kind:Pod", &no_label_cols()).unwrap();
        assert!(p.column_includes.contains_key("kind"));
        assert!(p.column_excludes.contains_key("kind"));
    }

    #[test]
    fn age_column_is_accepted() {
        let p = parse_network_filter("age:1d", &no_label_cols()).unwrap();
        assert!(p.column_includes.contains_key("age"));
    }

    #[test]
    fn data_column_is_rejected_for_network() {
        let err = parse_network_filter("data:0", &no_label_cols()).unwrap_err();
        assert!(err.contains("unknown column") && err.contains("data"));
    }

    #[test]
    fn label_selector_is_captured() {
        let p = parse_network_filter("label:app=nginx", &no_label_cols()).unwrap();
        assert_eq!(p.label_selector.as_deref(), Some("app=nginx"));
    }

    #[test]
    fn unknown_column_produces_parse_error() {
        let err = parse_network_filter("staus:Active", &no_label_cols()).unwrap_err();
        assert!(err.contains("unknown column") && err.contains("staus"));
    }

    #[test]
    fn namespace_returns_guidance_message() {
        let err = parse_network_filter("namespace:default", &no_label_cols()).unwrap_err();
        assert_eq!(
            err,
            "namespace is selected via the namespace selector, not the filter"
        );
    }

    #[test]
    fn registered_label_column_header_is_accepted() {
        let regs = registry_with("app", "APP");
        let p = parse_network_filter("app:nginx", &regs).unwrap();
        assert!(p.column_includes.contains_key("app"));
    }

    #[test]
    fn namespace_guidance_precedes_registry_even_on_collision() {
        let regs = registry_with("namespace", "NAMESPACE");
        let err = parse_network_filter("namespace:default", &regs).unwrap_err();
        assert_eq!(
            err,
            "namespace is selected via the namespace selector, not the filter"
        );
    }

    #[test]
    fn quoted_value_with_whitespace() {
        let p = parse_network_filter(r#"name:"my service""#, &no_label_cols()).unwrap();
        let patterns = p.column_includes.get("name").unwrap();
        assert!(patterns[0].is_match("my service"));
    }
}
