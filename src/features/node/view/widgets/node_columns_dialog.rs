use std::{collections::BTreeMap, str::FromStr as _};

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

    let items = build_check_list_items(default_columns, &label_registry);

    CheckList::builder()
        .id(NODE_COLUMNS_DIALOG_ID)
        .widget_base(widget_base)
        .theme(check_list_theme)
        .items(items)
        .on_change(on_change(tx.clone()))
        .build()
        .into()
}

/// All candidate columns: every builtin, then every defined label column.
fn candidate_specs(label_registry: &[NodeLabelColumn]) -> Vec<NodeColumnSpec> {
    NodeColumn::iter()
        .map(NodeColumnSpec::Builtin)
        .chain(label_registry.iter().map(|lc| {
            NodeColumnSpec::Label {
                key: lc.key.clone(),
                header: lc.header.clone(),
            }
        }))
        .collect()
}

/// Build the checklist items: the currently-active columns first (in their
/// order, checked), then the remaining candidates (unchecked). Each item
/// carries its `NodeColumnSpec` in `metadata` so the selection can be rebuilt
/// from the items alone — independent of any reordering done in the dialog.
fn build_check_list_items(
    default_columns: Option<NodeColumns>,
    label_registry: &[NodeLabelColumn],
) -> Vec<CheckListItem> {
    let candidates = candidate_specs(label_registry);
    let current = default_columns.unwrap_or_default();

    current
        .specs()
        .iter()
        .map(|spec| make_item(spec, true))
        .chain(
            candidates
                .iter()
                .filter(|spec| !current.specs().contains(spec))
                .map(|spec| make_item(spec, false)),
        )
        .collect()
}

fn make_item(spec: &NodeColumnSpec, checked: bool) -> CheckListItem {
    CheckListItem {
        label: spec.header(),
        checked,
        required: matches!(spec, NodeColumnSpec::Builtin(NodeColumn::Name)),
        metadata: Some(metadata_for(spec)),
    }
}

fn metadata_for(spec: &NodeColumnSpec) -> BTreeMap<String, String> {
    match spec {
        NodeColumnSpec::Builtin(c) => {
            BTreeMap::from([
                ("kind".to_string(), "builtin".to_string()),
                ("id".to_string(), c.as_str().to_string()),
            ])
        }
        NodeColumnSpec::Label { key, header } => {
            BTreeMap::from([
                ("kind".to_string(), "label".to_string()),
                ("key".to_string(), key.clone()),
                ("header".to_string(), header.clone()),
            ])
        }
    }
}

fn spec_from_item(item: &CheckListItem) -> Option<NodeColumnSpec> {
    let md = item.metadata.as_ref()?;
    match md.get("kind").map(String::as_str) {
        Some("builtin") => {
            NodeColumn::from_str(md.get("id")?)
                .ok()
                .map(NodeColumnSpec::Builtin)
        }
        Some("label") => {
            Some(NodeColumnSpec::Label {
                key: md.get("key")?.clone(),
                header: md.get("header")?.clone(),
            })
        }
        _ => None,
    }
}

/// Collect the selected columns from the checklist items, preserving the
/// items' current display order (which the user can reorder in the dialog).
fn collect_columns(items: &[CheckListItem]) -> NodeColumns {
    let specs: Vec<NodeColumnSpec> = items
        .iter()
        .filter(|item| item.required || item.checked)
        .filter_map(spec_from_item)
        .collect();

    NodeColumns::new(specs).ensure_name_column()
}

fn on_change(tx: Sender<Message>) -> impl Fn(&mut Window, &CheckListItem) -> EventResult {
    move |w: &mut Window, _v| {
        let widget = w
            .find_widget_mut(NODE_COLUMNS_DIALOG_ID)
            .as_mut_check_list();

        let columns = collect_columns(widget.items());

        tx.send(NodeMessage::Request(columns).into())
            .expect("Failed to send NodeMessage::Request");

        EventResult::Nop
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn label_spec(key: &str, header: &str) -> NodeColumnSpec {
        NodeColumnSpec::Label {
            key: key.into(),
            header: header.into(),
        }
    }

    #[test]
    fn 選択列を先頭にその他候補を未チェックで並べる() {
        let registry = vec![NodeLabelColumn {
            name: "zone".into(),
            key: "topology.kubernetes.io/zone".into(),
            header: "ZONE".into(),
        }];
        let current = NodeColumns::new([
            NodeColumnSpec::Builtin(NodeColumn::Name),
            label_spec("topology.kubernetes.io/zone", "ZONE"),
        ]);

        let items = build_check_list_items(Some(current), &registry);

        // 先頭2件は選択済み(NAME, ZONE)、以降は未チェック。
        assert_eq!(items[0].label, "NAME");
        assert!(items[0].checked);
        assert_eq!(items[1].label, "ZONE");
        assert!(items[1].checked);
        assert!(items[2..].iter().all(|i| !i.checked));
    }

    #[test]
    fn 並べ替え後も表示順どおりに列を収集する() {
        // [NAME(必須), ZONE] が選択された状態で、ZONE を NAME より前へ移動。
        let name = make_item(&NodeColumnSpec::Builtin(NodeColumn::Name), true);
        let zone = make_item(&label_spec("topology.kubernetes.io/zone", "ZONE"), true);
        let status = make_item(&NodeColumnSpec::Builtin(NodeColumn::Status), false);

        // 移動後の表示順: ZONE, NAME, STATUS(未チェック)
        let reordered = vec![zone, name, status];

        let columns = collect_columns(&reordered);

        // STATUS は混入せず、表示順(ZONE, NAME)どおりに収集される。
        assert_eq!(
            columns.specs(),
            &[
                label_spec("topology.kubernetes.io/zone", "ZONE"),
                NodeColumnSpec::Builtin(NodeColumn::Name),
            ]
        );
    }

    #[test]
    fn メタデータからspecを復元できる() {
        let builtin = make_item(&NodeColumnSpec::Builtin(NodeColumn::InternalIP), true);
        let label = make_item(&label_spec("k", "MIG"), true);

        assert_eq!(
            spec_from_item(&builtin),
            Some(NodeColumnSpec::Builtin(NodeColumn::InternalIP))
        );
        assert_eq!(spec_from_item(&label), Some(label_spec("k", "MIG")));
    }
}
