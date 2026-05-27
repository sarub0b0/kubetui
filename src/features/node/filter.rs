//! Node tab filter: parser + `TableFilterApplicator` factory.
//!
//! The parser produces a `TableFilterPredicate` directly (no Node-specific
//! predicate type). The factory wires the parser into the Table widget's
//! filter framework with `ApplyStrategy::EnterToConfirm`, a help-dialog
//! dispatch, and an `on_apply` callback that forwards the parsed
//! `labelSelector` to the Node poller via `NodeFilterMessage::Apply`.

mod parser;

// Consumed by the node_filter_applicator factory in Task 6 (this PR).
#[allow(unused_imports)]
pub use parser::parse_node_filter;
