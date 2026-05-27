use std::collections::{HashMap, HashSet};

use regex::Regex;
use strum::IntoEnumIterator;

use crate::{
    features::node::node_columns::{NodeColumn, NodeLabelColumn},
    ui::widget::TableFilterPredicate,
};

/// One parsed term from the input.
#[derive(Debug)]
enum Term {
    /// Bare value (no prefix) → defaults to NAME include.
    Bare(String),
    /// `<col>:<value>` include.
    Include { column: String, value: String },
    /// `!<col>:<value>` exclude.
    Exclude { column: String, value: String },
    /// `label:<selector>` → passed verbatim to the k8s API as labelSelector.
    Label(String),
}

fn parse_term(token: &str) -> Term {
    // `label:` is checked first so it is never mistaken for a generic column include.
    if let Some(sel) = token.strip_prefix("label:") {
        if !sel.is_empty() {
            return Term::Label(sel.to_string());
        }
    }

    if let Some(stripped) = token.strip_prefix('!') {
        if let Some((col, val)) = stripped.split_once(':') {
            if !col.is_empty() && !val.is_empty() {
                return Term::Exclude {
                    column: col.to_lowercase(),
                    value: val.to_string(),
                };
            }
        }
        // Fall through: `!worker` without colon is a bare value.
    }

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

/// Build the set of valid column names from the builtin `NodeColumn` variants
/// plus any headers registered in `label_registry`. All names are lowercased
/// so matching is case-insensitive.
fn valid_columns(label_registry: &[NodeLabelColumn]) -> HashSet<String> {
    let mut set: HashSet<String> = NodeColumn::iter()
        .map(|c| c.display().to_lowercase())
        .collect();
    for lc in label_registry {
        set.insert(lc.header.to_lowercase());
    }
    set
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
    let valid = valid_columns(label_registry);

    let trimmed = input.trim();
    let mut column_includes: HashMap<String, Vec<Regex>> = HashMap::new();
    let mut column_excludes: HashMap<String, Vec<Regex>> = HashMap::new();
    let mut label_selector: Option<String> = None;

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
                if !valid.contains(&column) {
                    return Err(format!("unknown column '{}'", column));
                }
                let rx =
                    Regex::new(&value).map_err(|e| format!("invalid regex '{}': {}", value, e))?;
                column_includes.entry(column).or_default().push(rx);
            }
            Term::Exclude { column, value } => {
                if !valid.contains(&column) {
                    return Err(format!("unknown column '{}'", column));
                }
                let rx =
                    Regex::new(&value).map_err(|e| format!("invalid regex '{}': {}", value, e))?;
                column_excludes.entry(column).or_default().push(rx);
            }
            Term::Label(sel) => {
                // Last label: term wins (k8s API accepts only one labelSelector value).
                label_selector = Some(sel);
            }
        }
    }

    Ok(TableFilterPredicate {
        column_includes,
        column_excludes,
        label_selector,
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

    fn registry_with(name: &str, header: &str) -> Vec<NodeLabelColumn> {
        vec![NodeLabelColumn {
            name: name.to_string(),
            key: "irrelevant.example.com/key".to_string(),
            header: header.to_string(),
        }]
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
    fn builtin_columns_are_accepted() {
        // `name` and `status` are builtin headers — must not error.
        assert!(parse_node_filter("name:n status:s", &no_label_cols()).is_ok());
    }

    #[test]
    fn registered_label_column_header_is_accepted() {
        let regs = registry_with("zone", "ZONE");
        let p = parse_node_filter("zone:us-west", &regs).unwrap();
        assert!(p.column_includes.contains_key("zone"));
    }

    #[test]
    fn label_keyword_is_not_treated_as_a_column_lookup() {
        // 'label:role=worker' must NOT trigger unknown-column validation
        // (it's the special-cased k8s labelSelector path).
        assert!(parse_node_filter("label:role=worker", &no_label_cols()).is_ok());
    }
}
