use std::{cell::RefCell, rc::Rc};

use crossbeam::channel::Sender;
use strum::IntoEnumIterator;

use crate::{
    config::theme::WidgetThemeConfig,
    features::{
        component_id::NODE_COLUMNS_DIALOG_ID,
        node::{message::NodeMessage, NodeColumn, NodeColumnSpec, NodeColumns, NodeLabelColumn},
    },
    message::Message,
    ui::{
        event::EventResult,
        widget::{CheckList, CheckListItem, CheckListTheme, Widget, WidgetBase, WidgetTheme},
        Window,
    },
};

pub fn node_columns_dialog(
    tx: &Sender<Message>,
    default_columns: Option<NodeColumns>,
    label_registry: Vec<NodeLabelColumn>,
    theme: WidgetThemeConfig,
) -> Widget<'static> {
    let check_list_theme = CheckListTheme::from(theme.clone());
    let widget_theme = WidgetTheme::from(theme.clone());
    let widget_base = WidgetBase::builder()
        .title("Node Columns")
        .theme(widget_theme)
        .build();

    // All candidate columns in checklist order: every builtin, then every
    // defined label column.
    let candidates: Vec<NodeColumnSpec> = NodeColumn::iter()
        .map(NodeColumnSpec::Builtin)
        .chain(label_registry.iter().map(|lc| {
            NodeColumnSpec::Label {
                key: lc.key.clone(),
                header: lc.header.clone(),
            }
        }))
        .collect();

    // Current column set (the order-preservation baseline).
    let current = default_columns.unwrap_or_default();

    let items: Vec<CheckListItem> = candidates
        .iter()
        .map(|spec| {
            CheckListItem {
                label: spec.header(),
                checked: current.specs().contains(spec),
                required: matches!(spec, NodeColumnSpec::Builtin(NodeColumn::Name)),
                metadata: None,
            }
        })
        .collect();

    let state = Rc::new(RefCell::new(current));

    CheckList::builder()
        .id(NODE_COLUMNS_DIALOG_ID)
        .widget_base(widget_base)
        .theme(check_list_theme)
        .items(items)
        .on_change(on_change(tx.clone(), candidates, state))
        .build()
        .into()
}

fn on_change(
    tx: Sender<Message>,
    candidates: Vec<NodeColumnSpec>,
    state: Rc<RefCell<NodeColumns>>,
) -> impl Fn(&mut Window, &CheckListItem) -> EventResult {
    move |w: &mut Window, _v| {
        let widget = w
            .find_widget_mut(NODE_COLUMNS_DIALOG_ID)
            .as_mut_check_list();

        let checked: Vec<NodeColumnSpec> = widget
            .items()
            .iter()
            .enumerate()
            .filter(|(_, item)| item.required || item.checked)
            .map(|(idx, _)| candidates[idx].clone())
            .collect();

        let rebuilt = rebuild_columns(&state.borrow(), &checked);
        *state.borrow_mut() = rebuilt.clone();

        tx.send(NodeMessage::Request(rebuilt).into())
            .expect("Failed to send NodeMessage::Request");

        EventResult::Nop
    }
}

/// Keep the current order for still-checked columns, drop unchecked ones, and
/// append newly-checked columns at the end.
fn rebuild_columns(current: &NodeColumns, checked: &[NodeColumnSpec]) -> NodeColumns {
    let mut result: Vec<NodeColumnSpec> = current
        .specs()
        .iter()
        .filter(|s| checked.contains(s))
        .cloned()
        .collect();

    for s in checked {
        if !result.contains(s) {
            result.push(s.clone());
        }
    }

    NodeColumns::new(result).ensure_name_column()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn label(key: &str, header: &str) -> NodeColumnSpec {
        NodeColumnSpec::Label {
            key: key.into(),
            header: header.into(),
        }
    }

    #[test]
    fn rebuild_preserves_order_and_appends_new() {
        let current = NodeColumns::new([
            NodeColumnSpec::Builtin(NodeColumn::Name),
            label("k", "MIG"),
            NodeColumnSpec::Builtin(NodeColumn::Status),
        ]);
        let checked = vec![
            NodeColumnSpec::Builtin(NodeColumn::Name),
            label("k", "MIG"),
            NodeColumnSpec::Builtin(NodeColumn::Roles),
        ];
        let rebuilt = rebuild_columns(&current, &checked);
        assert_eq!(
            rebuilt.specs(),
            &[
                NodeColumnSpec::Builtin(NodeColumn::Name),
                label("k", "MIG"),
                NodeColumnSpec::Builtin(NodeColumn::Roles),
            ]
        );
    }

    #[test]
    fn rebuild_ensures_name_column() {
        let current = NodeColumns::new([NodeColumnSpec::Builtin(NodeColumn::Status)]);
        let checked = vec![NodeColumnSpec::Builtin(NodeColumn::Status)];
        let rebuilt = rebuild_columns(&current, &checked);
        assert_eq!(
            rebuilt.specs()[0],
            NodeColumnSpec::Builtin(NodeColumn::Name)
        );
    }
}
