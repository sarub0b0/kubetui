//! Pod tab filter: parser + `TableFilterApplicator` factory.
//!
//! The applicator wires `parse_pod_filter` (which builds on the shared
//! `parse_table_filter`) into the Table widget with `EnterToConfirm` strategy.
//! Server-side `labelSelector` is forwarded to the Pod poller via
//! `PodMessage::Filter` from `on_apply`/`on_cancel`. Typing `?` or `help` in
//! the filter input opens the `POD_FILTER_HELP_DIALOG_ID` dialog.

mod parser;

use crossbeam::channel::Sender;

use crate::{
    features::{component_id::POD_FILTER_HELP_DIALOG_ID, pod::message::PodMessage},
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

pub use parser::parse_pod_filter;

/// Build the Pod tab's filter applicator.
///
/// `tx` is captured by `on_apply`/`on_cancel` to forward the parsed
/// `label_selector` to the Pod poller via `PodMessage::Filter`.
///
/// The applicator uses `EnterToConfirm` so the parser only runs on Enter
/// (avoids server-side roundtrips mid-typing).
pub fn pod_filter_applicator(tx: Sender<Message>) -> TableFilterApplicator {
    let parser: TableFilterParser = (move |input: &str| parse_pod_filter(input)).into();

    let tx_apply = tx.clone();
    let tx_cancel = tx;

    let on_apply: OnFilterApply = (move |predicate: &crate::ui::widget::TableFilterPredicate, _window: &mut Window| {
        tx_apply
            .send(PodMessage::Filter(predicate.label_selector.clone()).into())
            .expect("Failed to send PodMessage::Filter");
    })
    .into();

    let on_cancel: OnFilterCancel = (move |_window: &mut Window| {
        tx_cancel
            .send(PodMessage::Filter(None).into())
            .expect("Failed to send PodMessage::Filter(None) on cancel");
    })
    .into();

    TableFilterApplicator::new(parser, ApplyStrategy::EnterToConfirm)
        .with_help_dialog(POD_FILTER_HELP_DIALOG_ID)
        .with_on_apply(on_apply)
        .with_on_cancel(on_cancel)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applicator_constructs_without_panic() {
        let (tx, _rx) = crossbeam::channel::bounded(1);
        let _ = pod_filter_applicator(tx);
    }
}
