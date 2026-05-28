//! Node tab filter: parser + `TableFilterApplicator` factory.
//!
//! The parser produces a `TableFilterPredicate` directly (no Node-specific
//! predicate type). The factory wires the parser into the Table widget's
//! filter framework with `ApplyStrategy::EnterToConfirm`. Server-side
//! `labelSelector` forwarding is handled via the `on_apply` callback that
//! sends `NodeMessage::Filter` to the poller. Typing `?` or `help` in the
//! filter input opens the `NODE_FILTER_HELP_DIALOG_ID` dialog.

mod parser;

use crossbeam::channel::Sender;

use crate::{
    features::{
        component_id::NODE_FILTER_HELP_DIALOG_ID,
        node::{message::NodeMessage, node_columns::NodeLabelColumn},
    },
    message::Message,
    ui::{
        widget::{
            ApplyStrategy,
            OnFilterApply,
            OnFilterCancel,
            TableFilterApplicator,
            TableFilterParser,
        },
        Window,
    },
};

pub use parser::parse_node_filter;

/// Build the Node tab's filter applicator.
///
/// `label_registry` is captured by value and used by the parser for
/// column-name validation. `tx` is captured by the `on_apply` callback to
/// forward the parsed `label_selector` to the Node poller via
/// `NodeMessage::Filter`.
///
/// The applicator uses `EnterToConfirm` strategy so that the parser only
/// runs on Enter (avoids spamming the kube API mid-typing).
pub fn node_filter_applicator(
    label_registry: Vec<NodeLabelColumn>,
    tx: Sender<Message>,
) -> TableFilterApplicator {
    let parser: TableFilterParser =
        (move |input: &str| parse_node_filter(input, &label_registry)).into();

    let tx_apply = tx.clone();
    let tx_cancel = tx;

    let on_apply: OnFilterApply = (move |predicate: &crate::ui::widget::TableFilterPredicate,
                                         _window: &mut Window| {
        tx_apply
            .send(NodeMessage::Filter(predicate.label_selector.clone()).into())
            .expect("Failed to send NodeMessage::Filter");
    })
    .into();

    let on_cancel: OnFilterCancel = (move |_window: &mut Window| {
        tx_cancel
            .send(NodeMessage::Filter(None).into())
            .expect("Failed to send NodeMessage::Filter(None) on cancel");
    })
    .into();

    TableFilterApplicator::new(parser, ApplyStrategy::EnterToConfirm)
        .with_help_dialog(NODE_FILTER_HELP_DIALOG_ID)
        .with_on_apply(on_apply)
        .with_on_cancel(on_cancel)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applicator_constructs_without_panic() {
        let (tx, _rx) = crossbeam::channel::bounded(1);
        let _ = node_filter_applicator(Vec::new(), tx);
    }
}
