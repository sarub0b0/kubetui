//! Node tab filter: parser + `TableFilterApplicator` factory.
//!
//! The parser produces a `TableFilterPredicate` directly (no Node-specific
//! predicate type). The factory wires the parser into the Table widget's
//! filter framework with `ApplyStrategy::EnterToConfirm`. Server-side
//! `labelSelector` forwarding and the help-dialog dispatch are wired in
//! later tasks (T7 for the message, T10 for the help dialog).

mod parser;

use crate::{
    features::node::node_columns::NodeLabelColumn,
    ui::widget::{ApplyStrategy, TableFilterApplicator, TableFilterParser},
};

// Consumed by the node_filter_applicator factory in this file.
#[allow(unused_imports)]
pub use parser::parse_node_filter;

/// Build the Node tab's filter applicator.
///
/// `label_registry` is captured by value and used by the parser for
/// column-name validation. The applicator uses `EnterToConfirm` strategy
/// so that the parser only runs on Enter (avoids spamming the kube API
/// mid-typing once server-side labelSelector forwarding lands in Task 7).
#[allow(dead_code)]
pub fn node_filter_applicator(label_registry: Vec<NodeLabelColumn>) -> TableFilterApplicator {
    let parser: TableFilterParser =
        (move |input: &str| parse_node_filter(input, &label_registry)).into();

    TableFilterApplicator::new(parser, ApplyStrategy::EnterToConfirm)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applicator_constructs_without_panic() {
        // 単に factory が構築できることを exercise する。
        // 後続 task の widget 切替まで applicator は consume されないので、
        // ここで型エラーやクロージャ捕捉エラーを早期検出する。
        let _ = node_filter_applicator(Vec::new());
    }
}
