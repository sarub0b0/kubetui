use std::collections::HashSet;

use strum::IntoEnumIterator;

use crate::{
    features::node::node_columns::{NodeColumn, NodeLabelColumn},
    ui::widget::{normalize_column_name, parse_table_filter, TableFilterPredicate},
};

// ---------------------------------------------------------------------------
// Column validation helpers
// ---------------------------------------------------------------------------

/// Build the set of valid (known) column names: builtin Node columns plus any
/// defined label columns, normalized so matching is case/format-insensitive.
fn valid_columns(label_registry: &[NodeLabelColumn]) -> HashSet<String> {
    let mut set: HashSet<String> = NodeColumn::iter()
        .map(|c| normalize_column_name(c.display()))
        .collect();
    for lc in label_registry {
        set.insert(normalize_column_name(&lc.header));
    }
    set
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Parse a Node-filter input string into a `TableFilterPredicate`.
///
/// Column references are validated against the set of *known* columns (builtin
/// Node columns plus defined label columns in `label_registry`); an unknown
/// column produces a parse error. A known column that is not currently
/// displayed is accepted here and becomes inactive at match time. Tokenization
/// and predicate building are delegated to the shared `parse_table_filter`.
pub fn parse_node_filter(
    input: &str,
    label_registry: &[NodeLabelColumn],
) -> Result<TableFilterPredicate, String> {
    let valid = valid_columns(label_registry);
    parse_table_filter(input, |column| {
        if valid.contains(&normalize_column_name(column)) {
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

    fn no_label_cols() -> Vec<NodeLabelColumn> {
        Vec::new()
    }

    fn registry_with(name: &str, header: &str) -> Vec<NodeLabelColumn> {
        vec![NodeLabelColumn {
            name: name.to_string(),
            key: "irrelevant.example.com/key".to_string(),
            header: header.to_string(),
        }]
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

    #[test]
    fn excludes_prefixed_with_bang_populate_column_excludes() {
        let p = parse_node_filter("!name:kube-system", &no_label_cols()).unwrap();
        assert!(p.column_includes.is_empty());
        let patterns = p.column_excludes.get("name").expect("name column");
        assert_eq!(patterns.len(), 1);
        assert!(patterns[0].is_match("kube-system"));
    }

    #[test]
    fn includes_and_excludes_coexist() {
        let p = parse_node_filter("status:Ready !name:kube-system", &no_label_cols()).unwrap();
        assert_eq!(p.column_includes.len(), 1);
        assert_eq!(p.column_excludes.len(), 1);
    }

    #[test]
    fn bang_without_colon_is_treated_as_bare_value() {
        // `!worker` は `!name:worker` の省略形ではない。bang は明示的な column と組でのみ意味を持つ。
        let p = parse_node_filter("!worker", &no_label_cols()).unwrap();
        // 文字列 `!worker` がそのまま NAME 列の regex になる。regex crate は `!worker` をリテラル `!worker` のマッチとして受け入れる。
        let patterns = p.column_includes.get("name").expect("name column");
        assert_eq!(patterns.len(), 1);
        assert!(patterns[0].is_match("!worker"));
        assert!(p.column_excludes.is_empty());
    }

    #[test]
    fn label_selector_is_captured_verbatim() {
        let p = parse_node_filter("label:role=worker", &no_label_cols()).unwrap();
        assert_eq!(p.label_selector.as_deref(), Some("role=worker"));
        assert!(p.column_includes.is_empty());
        assert!(p.column_excludes.is_empty());
    }

    #[test]
    fn label_selector_supports_kubectl_comma_and() {
        let p = parse_node_filter("label:role=worker,zone=us-west", &no_label_cols()).unwrap();
        assert_eq!(
            p.label_selector.as_deref(),
            Some("role=worker,zone=us-west")
        );
    }

    #[test]
    fn multiple_label_terms_keep_the_last() {
        // The k8s API accepts only one labelSelector value; spec requires
        // last-wins to match the Pod log query convention.
        let p = parse_node_filter("label:a=1 label:b=2", &no_label_cols()).unwrap();
        assert_eq!(p.label_selector.as_deref(), Some("b=2"));
    }

    #[test]
    fn label_and_column_terms_coexist() {
        let p = parse_node_filter(
            "status:Ready label:role=worker !name:kube-system",
            &no_label_cols(),
        )
        .unwrap();
        assert_eq!(p.column_includes.len(), 1);
        assert_eq!(p.column_excludes.len(), 1);
        assert_eq!(p.label_selector.as_deref(), Some("role=worker"));
    }

    #[test]
    fn unknown_column_produces_parse_error() {
        let err = parse_node_filter("statusu:Ready", &no_label_cols()).unwrap_err();
        assert!(
            err.contains("unknown column") && err.contains("statusu"),
            "error should mention the bad column: {}",
            err
        );
    }

    #[test]
    fn unknown_column_in_exclude_also_errors() {
        let err = parse_node_filter("!agee:1h", &no_label_cols()).unwrap_err();
        assert!(
            err.contains("unknown column") && err.contains("agee"),
            "error should mention the bad column: {}",
            err
        );
    }

    #[test]
    fn label_keyword_is_not_treated_as_a_column_lookup() {
        // 'label:role=worker' must NOT trigger unknown-column validation
        // (it's the special-cased k8s labelSelector path).
        assert!(parse_node_filter("label:role=worker", &no_label_cols()).is_ok());
    }

    #[test]
    fn registered_label_column_header_is_accepted() {
        let regs = registry_with("zone", "ZONE");
        let p = parse_node_filter("zone:us-west", &regs).unwrap();
        assert!(p.column_includes.contains_key("zone"));
    }

    #[test]
    fn builtin_columns_are_accepted() {
        // name と status は builtin → エラーにならない
        assert!(parse_node_filter("name:n status:s", &no_label_cols()).is_ok());
    }

    #[test]
    fn hyphenated_builtin_column_is_accepted_via_normalization() {
        // INTERNAL-IP は builtin。internalip / internal-ip いずれでも受理。
        let p = parse_node_filter("internalip:10.", &no_label_cols()).unwrap();
        assert!(p.column_includes.contains_key("internalip"));
        let p2 = parse_node_filter("internal-ip:10.", &no_label_cols()).unwrap();
        assert!(p2.column_includes.contains_key("internalip"));
    }

    // -----------------------------------------------------------------------
    // New quoting / escape tests (Task 12)
    // -----------------------------------------------------------------------

    #[test]
    fn double_quoted_value_with_spaces_is_kept_intact() {
        let p = parse_node_filter(r#"os-image:"Ubuntu 22.04.3 LTS""#, &no_label_cols()).unwrap();
        let patterns = p.column_includes.get("osimage").expect("osimage col");
        assert_eq!(patterns.len(), 1);
        assert!(patterns[0].is_match("Ubuntu 22.04.3 LTS"));
    }

    #[test]
    fn single_quoted_value_with_spaces_is_kept_intact() {
        let p = parse_node_filter(r#"os-image:'Ubuntu 22.04 LTS'"#, &no_label_cols()).unwrap();
        let patterns = p.column_includes.get("osimage").unwrap();
        assert!(patterns[0].is_match("Ubuntu 22.04 LTS"));
    }

    #[test]
    fn quoted_value_with_escaped_quote() {
        let p = parse_node_filter(r#"name:"foo\"bar""#, &no_label_cols()).unwrap();
        let patterns = p.column_includes.get("name").unwrap();
        // 値は `foo"bar` という regex
        assert!(patterns[0].is_match(r#"foo"bar"#));
    }

    #[test]
    fn quoted_value_preserves_regex_backslash_classes() {
        // `\s` をリテラルに残して regex `\s`（空白）になる
        let p = parse_node_filter(r#"name:"foo\sbar""#, &no_label_cols()).unwrap();
        let patterns = p.column_includes.get("name").unwrap();
        assert!(patterns[0].is_match("foo bar")); // regex \s が空白マッチ
        assert!(!patterns[0].is_match("foobar"));
    }

    #[test]
    fn bare_value_with_quoted_spaces() {
        // bare の場合も quoted value をサポート: "node a" → NAME に regex "node a"
        let p = parse_node_filter(r#""node a""#, &no_label_cols()).unwrap();
        let patterns = p.column_includes.get("name").unwrap();
        assert!(patterns[0].is_match("node a"));
    }

    #[test]
    fn mixed_quoted_and_unquoted_tokens() {
        let p =
            parse_node_filter(r#"status:Ready os-image:"Ubuntu 22.04""#, &no_label_cols()).unwrap();
        assert_eq!(p.column_includes.len(), 2);
        assert!(p.column_includes.get("osimage").unwrap()[0].is_match("Ubuntu 22.04"));
    }

    #[test]
    fn unclosed_quote_is_a_parse_error() {
        assert!(parse_node_filter(r#"name:"unterminated"#, &no_label_cols()).is_err());
    }
}
