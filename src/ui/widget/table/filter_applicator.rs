use std::collections::HashMap;

use regex::Regex;

use crate::ui::widget::{styled_graphemes::StyledGraphemes, TableItem};

/// A set of filter predicates that determine whether a [`TableItem`] should be
/// shown in the table.
///
/// All non-empty fields are AND-combined; within `column_includes` and
/// `column_excludes` the per-column patterns are AND-combined too, but the
/// patterns within a single column list are OR-combined.
///
/// ```text
/// result = (col_A matches include_A?) AND (col_B matches include_B?) AND …
///        AND NOT (col_A matches exclude_A?) AND NOT (col_B matches exclude_B?) AND …
/// ```
#[derive(Debug, Clone, Default)]
pub struct TableFilterPredicate {
    /// Column-name → list of regexes, any one of which must match that column
    /// (OR within a column, AND across columns).
    pub column_includes: HashMap<String, Vec<Regex>>,

    /// Column-name → list of regexes; if any pattern matches the column,
    /// the row is excluded.
    pub column_excludes: HashMap<String, Vec<Regex>>,

    /// Opaque label selector string (e.g. `"app=foo,env=prod"`).
    /// Stored for display / forwarding; NOT evaluated inside `matches()`.
    pub label_selector: Option<String>,

    /// Raw filter string stored for display / forwarding.
    /// NOT evaluated inside `matches()`.
    pub raw: String,
}

impl TableFilterPredicate {
    /// Returns `true` when this predicate is entirely empty (no filtering).
    pub fn is_empty(&self) -> bool {
        self.column_includes.is_empty()
            && self.column_excludes.is_empty()
            && self.label_selector.is_none()
            && self.raw.is_empty()
    }

    /// Returns `true` when `item` passes all active filters.
    pub fn matches(&self, item: &TableItem, header: &[String]) -> bool {
        // --- column_includes (AND across columns, OR within) ---
        for (col, patterns) in &self.column_includes {
            let cell = cell_of(item, header, col).unwrap_or_default();
            if !patterns.iter().any(|r| r.is_match(&cell)) {
                return false;
            }
        }

        // --- column_excludes (AND across columns, OR within → exclude) ---
        for (col, patterns) in &self.column_excludes {
            let cell = cell_of(item, header, col).unwrap_or_default();
            if patterns.iter().any(|r| r.is_match(&cell)) {
                return false;
            }
        }

        true
    }
}

/// Returns the ANSI-stripped text of the column named `col_name` in `item`,
/// or `None` if the column name is not found in `header`.
// TODO(perf): cell_of() is called per column × per row × per render. Each
// invocation re-lowercases the entire header. If profiling shows this in the
// hot path once Table widget wiring lands (Tasks 5/7), pre-compute a
// column-name → index map at filter_state set time.
fn cell_of(item: &TableItem, header: &[String], col_name: &str) -> Option<String> {
    let col_name_lower = col_name.to_lowercase();
    let idx = header
        .iter()
        .position(|h| h.to_lowercase() == col_name_lower)?;

    item.item
        .get(idx)
        .map(|c| c.styled_graphemes_symbols().concat())
}

// ---------------------------------------------------------------------------
// Callback types and factory struct
// ---------------------------------------------------------------------------

use crate::{
    define_callback,
    ui::{event::EventResult, Window},
};

// TableFilterParser: parses a raw filter string into a TableFilterPredicate.
// Returns Ok(predicate) on success, or Err(message) for display to the user.
define_callback!(pub TableFilterParser, Fn(&str) -> Result<TableFilterPredicate, String>);

// OnFilterApply: called after a filter has been applied (or cleared). Receives
// the new predicate and a mutable Window reference for side effects (e.g.,
// forwarding labelSelector to a poller via shared state).
define_callback!(pub OnFilterApply, Fn(&TableFilterPredicate, &mut Window));

/// Controls *when* the filter is actually applied to the table rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyStrategy {
    /// Filter is applied on every keystroke (incremental / live search).
    Live,
    /// Filter is applied only when the user presses Enter.
    EnterToConfirm,
}

/// Bundles everything a [`Table`] widget needs to support column filtering.
///
/// Construct via [`TableFilterApplicator::new`] and use the builder methods to
/// attach optional components before passing to `Table::builder().filter_applicator()`.
pub struct TableFilterApplicator {
    /// Converts a raw filter string into a [`TableFilterPredicate`].
    pub(crate) parser: TableFilterParser,
    /// Controls when the predicate is applied to the visible rows.
    pub(crate) strategy: ApplyStrategy,
    /// Optional dialog-id of the help overlay (opened when user presses `?`
    /// while the filter input is focused).
    pub(crate) help_dialog_id: Option<String>,
    /// Optional callback invoked after the filter changes.
    pub(crate) on_apply: Option<OnFilterApply>,
}

impl std::fmt::Debug for TableFilterApplicator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TableFilterApplicator")
            .field("strategy", &self.strategy)
            .field("help_dialog_id", &self.help_dialog_id)
            .field("parser", &"<TableFilterParser>")
            .field("on_apply", &self.on_apply.as_ref().map(|_| "<OnFilterApply>"))
            .finish()
    }
}

impl TableFilterApplicator {
    /// Create a new applicator with the given parser and apply strategy.
    pub fn new(parser: TableFilterParser, strategy: ApplyStrategy) -> Self {
        Self {
            parser,
            strategy,
            help_dialog_id: None,
            on_apply: None,
        }
    }

    /// Attach a help-dialog id. When the filter input is focused and the user
    /// presses `?`, the dialog with this id will be opened.
    pub fn with_help_dialog(mut self, id: impl Into<String>) -> Self {
        self.help_dialog_id = Some(id.into());
        self
    }

    /// Attach a callback that is invoked after every filter change.
    pub fn with_on_apply(mut self, cb: OnFilterApply) -> Self {
        self.on_apply = Some(cb);
        self
    }
}

// Suppress unused-import warning: EventResult is referenced by the
// define_callback! expansion indirectly; keep the use above for correctness.
#[allow(unused_imports)]
use EventResult as _;

/// 既存タブが使う「単一列の部分一致」フィルタを構成する。
/// `column` は対象列名（大小区別なし、内部で小文字化）。
///
/// 挙動は既存の `filtered_key + split(' ').any(contains)` と等価:
/// - 空入力 → フィルタなし（全行 pass）
/// - スペース区切りの複数 pattern → OR
/// - 各 pattern は `regex::escape` でリテラル化（ユーザーは regex を
///   意識しなくていい、`.` `*` 等が混入しても安全）
pub fn substring_applicator(column: &str) -> TableFilterApplicator {
    let col = column.to_string().to_lowercase();
    let parser: TableFilterParser = (move |input: &str| {
        let raw = input.to_string();
        let patterns: Result<Vec<Regex>, _> = input
            .split_whitespace()
            .map(regex::escape)
            .map(|p| Regex::new(&p))
            .collect();
        let patterns = patterns.map_err(|e| e.to_string())?;

        let mut column_includes = HashMap::new();
        if !patterns.is_empty() {
            column_includes.insert(col.clone(), patterns);
        }

        Ok(TableFilterPredicate {
            column_includes,
            column_excludes: HashMap::new(),
            label_selector: None,
            raw,
        })
    })
    .into();

    TableFilterApplicator::new(parser, ApplyStrategy::Live)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_item(cells: &[&str]) -> TableItem {
        TableItem::new(cells.iter().map(|s| s.to_string()).collect::<Vec<_>>(), None)
    }

    fn header(cols: &[&str]) -> Vec<String> {
        cols.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn empty_predicate_matches_anything() {
        let pred = TableFilterPredicate::default();
        let item = make_item(&["foo", "bar"]);
        let hdr = header(&["NAME", "STATUS"]);
        assert!(pred.matches(&item, &hdr));
    }

    #[test]
    fn includes_within_column_use_or() {
        let mut pred = TableFilterPredicate::default();
        pred.column_includes.insert(
            "STATUS".to_string(),
            vec![
                Regex::new("Running").unwrap(),
                Regex::new("Pending").unwrap(),
            ],
        );
        let hdr = header(&["NAME", "STATUS"]);

        // matches "Running"
        assert!(pred.matches(&make_item(&["pod-a", "Running"]), &hdr));
        // matches "Pending"
        assert!(pred.matches(&make_item(&["pod-b", "Pending"]), &hdr));
        // neither → rejected
        assert!(!pred.matches(&make_item(&["pod-c", "Failed"]), &hdr));
    }

    #[test]
    fn includes_across_columns_use_and() {
        let mut pred = TableFilterPredicate::default();
        pred.column_includes
            .insert("NAME".to_string(), vec![Regex::new("web").unwrap()]);
        pred.column_includes
            .insert("STATUS".to_string(), vec![Regex::new("Running").unwrap()]);
        let hdr = header(&["NAME", "STATUS"]);

        // both match
        assert!(pred.matches(&make_item(&["web-server", "Running"]), &hdr));
        // name matches but status doesn't
        assert!(!pred.matches(&make_item(&["web-server", "Pending"]), &hdr));
        // status matches but name doesn't
        assert!(!pred.matches(&make_item(&["api-server", "Running"]), &hdr));
    }

    #[test]
    fn excludes_any_match_excludes() {
        let mut pred = TableFilterPredicate::default();
        pred.column_excludes.insert(
            "STATUS".to_string(),
            vec![
                Regex::new("Failed").unwrap(),
                Regex::new("Error").unwrap(),
            ],
        );
        let hdr = header(&["NAME", "STATUS"]);

        assert!(pred.matches(&make_item(&["pod-a", "Running"]), &hdr));
        assert!(!pred.matches(&make_item(&["pod-b", "Failed"]), &hdr));
        assert!(!pred.matches(&make_item(&["pod-c", "Error"]), &hdr));
    }

    #[test]
    fn excludes_across_columns_block_on_any_match() {
        let mut pred = TableFilterPredicate::default();
        pred.column_excludes
            .insert("NAME".to_string(), vec![Regex::new("bad").unwrap()]);
        pred.column_excludes
            .insert("STATUS".to_string(), vec![Regex::new("Failed").unwrap()]);
        let hdr = header(&["NAME", "STATUS"]);

        // neither column excluded → passes
        assert!(pred.matches(&make_item(&["good-pod", "Running"]), &hdr));
        // NAME matches exclusion → rejected
        assert!(!pred.matches(&make_item(&["bad-pod", "Running"]), &hdr));
        // STATUS matches exclusion → rejected
        assert!(!pred.matches(&make_item(&["good-pod", "Failed"]), &hdr));
    }

    #[test]
    fn includes_and_excludes_combine() {
        let mut pred = TableFilterPredicate::default();
        pred.column_includes
            .insert("NAME".to_string(), vec![Regex::new("web").unwrap()]);
        pred.column_excludes
            .insert("STATUS".to_string(), vec![Regex::new("Failed").unwrap()]);
        let hdr = header(&["NAME", "STATUS"]);

        // include satisfied, exclude not triggered
        assert!(pred.matches(&make_item(&["web-server", "Running"]), &hdr));
        // include satisfied, but exclude triggered
        assert!(!pred.matches(&make_item(&["web-server", "Failed"]), &hdr));
        // include NOT satisfied
        assert!(!pred.matches(&make_item(&["api-server", "Running"]), &hdr));
    }

    #[test]
    fn column_name_matching_is_case_insensitive() {
        let mut pred = TableFilterPredicate::default();
        pred.column_includes
            .insert("status".to_string(), vec![Regex::new("Running").unwrap()]);
        // header uses uppercase "STATUS"
        let hdr = header(&["NAME", "STATUS"]);

        assert!(pred.matches(&make_item(&["pod-a", "Running"]), &hdr));
        assert!(!pred.matches(&make_item(&["pod-a", "Pending"]), &hdr));
    }

    #[test]
    fn unknown_column_yields_empty_cell_so_fails_match() {
        let mut pred = TableFilterPredicate::default();
        pred.column_includes
            .insert("NONEXISTENT".to_string(), vec![Regex::new("anything").unwrap()]);
        let hdr = header(&["NAME", "STATUS"]);
        // The cell for an unknown column is "", which can never match "anything"
        assert!(!pred.matches(&make_item(&["pod-a", "Running"]), &hdr));
    }

    #[test]
    fn ansi_escape_in_cell_is_stripped_before_match() {
        let mut pred = TableFilterPredicate::default();
        pred.column_includes
            .insert("STATUS".to_string(), vec![Regex::new("Running").unwrap()]);
        let hdr = header(&["NAME", "STATUS"]);

        // Cell contains ANSI green color around "Running"
        let item = make_item(&["pod-a", "\x1b[32mRunning\x1b[0m"]);
        assert!(pred.matches(&item, &hdr));
    }

    #[test]
    fn ansi_escape_does_not_pollute_anchor_match() {
        let mut pred = TableFilterPredicate::default();
        // Anchored regex: would fail if ANSI bytes were left in the string
        pred.column_includes
            .insert("STATUS".to_string(), vec![Regex::new("^Ready$").unwrap()]);
        let hdr = header(&["NAME", "STATUS"]);

        // After stripping ANSI, the cell is "Ready" — anchored regex must match
        let item = make_item(&["pod-a", "\x1b[31mReady\x1b[0m"]);
        assert!(pred.matches(&item, &hdr));
    }

    #[test]
    fn ansi_escape_in_cell_not_matched_as_part_of_value() {
        let mut pred = TableFilterPredicate::default();
        // This regex matches the literal ANSI escape sequence fragment
        pred.column_includes
            .insert("STATUS".to_string(), vec![Regex::new(r"\[31m").unwrap()]);
        let hdr = header(&["NAME", "STATUS"]);

        // After ANSI stripping, "[31m" is gone — regex must NOT match
        let item = make_item(&["pod-a", "\x1b[31mReady\x1b[0m"]);
        assert!(!pred.matches(&item, &hdr));
    }

    #[test]
    fn is_empty_is_false_when_only_label_selector_is_set() {
        let pred = TableFilterPredicate {
            label_selector: Some("app=foo".to_string()),
            ..TableFilterPredicate::default()
        };
        assert!(!pred.is_empty());
    }

    #[test]
    fn is_empty_is_false_when_only_raw_is_set() {
        let pred = TableFilterPredicate {
            raw: "STATUS:Running".to_string(),
            ..TableFilterPredicate::default()
        };
        assert!(!pred.is_empty());
    }

    fn invoke_parser(a: &TableFilterApplicator, input: &str) -> TableFilterPredicate {
        (a.parser.closure)(input).expect("test input must parse")
    }

    #[test]
    fn substring_applicator_empty_input_matches_everything() {
        let a = substring_applicator("NAME");
        let p = invoke_parser(&a, "");
        let h = header(&["NAME"]);
        assert!(p.matches(&make_item(&["nginx"]), &h));
        assert!(p.matches(&make_item(&[""]), &h));
        assert!(p.is_empty());
    }

    #[test]
    fn substring_applicator_single_pattern_substring_match() {
        let a = substring_applicator("NAME");
        let p = invoke_parser(&a, "nginx");
        let h = header(&["NAME"]);
        assert!(p.matches(&make_item(&["abc-nginx-prod"]), &h));
        assert!(!p.matches(&make_item(&["web-server"]), &h));
    }

    #[test]
    fn substring_applicator_space_separated_is_or() {
        // 既存挙動と等価: "nginx web" は NAME に nginx OR web
        let a = substring_applicator("NAME");
        let p = invoke_parser(&a, "nginx web");
        let h = header(&["NAME"]);
        assert!(p.matches(&make_item(&["nginx-x"]), &h));
        assert!(p.matches(&make_item(&["web-y"]), &h));
        assert!(!p.matches(&make_item(&["api-z"]), &h));
    }

    #[test]
    fn substring_applicator_special_chars_are_escaped() {
        // "a.b" がリテラルとして扱われる（regex の `.` ではない）
        let a = substring_applicator("NAME");
        let p = invoke_parser(&a, "a.b");
        let h = header(&["NAME"]);
        assert!(p.matches(&make_item(&["x-a.b-y"]), &h));
        assert!(!p.matches(&make_item(&["x-a_b-y"]), &h)); // regex の . なら通るがリテラルなので落ちる
    }

    #[test]
    fn substring_applicator_strategy_is_live() {
        let a = substring_applicator("NAME");
        assert_eq!(a.strategy, ApplyStrategy::Live);
    }

    #[test]
    fn substring_applicator_help_dialog_id_is_none() {
        let a = substring_applicator("NAME");
        assert_eq!(a.help_dialog_id, None);
    }

    #[test]
    fn substring_applicator_on_apply_is_none() {
        let a = substring_applicator("NAME");
        assert!(a.on_apply.is_none());
    }
}
