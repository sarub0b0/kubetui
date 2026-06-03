//! Config tab filter: parser + `TableFilterApplicator` factory.
//!
//! The applicator wires `parse_config_filter` (which builds on the shared
//! `parse_table_filter`) into the Table widget with `EnterToConfirm` strategy.
//! Server-side `labelSelector` is forwarded to the Config poller via
//! `ConfigMessage::Filter` from `on_apply`/`on_cancel`. Typing `?` or `help`
//! in the filter input opens the `CONFIG_FILTER_HELP_DIALOG_ID` dialog.

mod parser;

use crossbeam::channel::Sender;

use crate::{
    features::{
        component_id::CONFIG_FILTER_HELP_DIALOG_ID,
        config::{message::ConfigMessage, ConfigLabelColumn},
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

pub use parser::parse_config_filter;

/// Build the Config tab's filter applicator.
///
/// `label_registry` is captured by the parser closure so registered label
/// column headers are accepted as valid filter columns. `tx` is captured by
/// `on_apply`/`on_cancel` to forward the parsed `label_selector` to the
/// Config poller via `ConfigMessage::Filter`.
///
/// The applicator uses `EnterToConfirm` so the parser only runs on Enter
/// (avoids server-side roundtrips mid-typing).
pub fn config_filter_applicator(
    label_registry: Vec<ConfigLabelColumn>,
    tx: Sender<Message>,
) -> TableFilterApplicator {
    let parser: TableFilterParser =
        (move |input: &str| parse_config_filter(input, &label_registry)).into();

    let tx_apply = tx.clone();
    let tx_cancel = tx;

    let on_apply: OnFilterApply = (move |predicate: &crate::ui::widget::TableFilterPredicate,
                                         _window: &mut Window| {
        tx_apply
            .send(ConfigMessage::Filter(predicate.label_selector.clone()).into())
            .expect("Failed to send ConfigMessage::Filter");
    })
    .into();

    let on_cancel: OnFilterCancel = (move |_window: &mut Window| {
        tx_cancel
            .send(ConfigMessage::Filter(None).into())
            .expect("Failed to send ConfigMessage::Filter(None) on cancel");
    })
    .into();

    TableFilterApplicator::new(parser, ApplyStrategy::EnterToConfirm)
        .with_help_dialog(CONFIG_FILTER_HELP_DIALOG_ID)
        .with_on_apply(on_apply)
        .with_on_cancel(on_cancel)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applicator_constructs_without_panic() {
        let (tx, _rx) = crossbeam::channel::bounded(1);
        let _ = config_filter_applicator(Vec::new(), tx);
    }
}
