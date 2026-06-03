use std::{collections::BTreeMap, str::FromStr as _};

use crossbeam::channel::Sender;
use strum::IntoEnumIterator;

use crate::{
    config::theme::WidgetThemeConfig,
    features::{
        component_id::NETWORK_COLUMNS_DIALOG_ID,
        network::{
            message::NetworkMessage,
            NetworkColumn,
            NetworkColumnSpec,
            NetworkColumns,
            NetworkLabelColumn,
        },
    },
    message::Message,
    ui::{
        event::EventResult,
        widget::{CheckList, CheckListItem, CheckListTheme, Widget, WidgetBase, WidgetTheme},
        Window,
    },
};

pub fn network_columns_dialog(
    tx: &Sender<Message>,
    default_columns: NetworkColumns,
    label_registry: Vec<NetworkLabelColumn>,
    theme: WidgetThemeConfig,
) -> Widget<'static> {
    let check_list_theme = CheckListTheme::from(theme.clone());
    let widget_theme = WidgetTheme::from(theme.clone());
    let widget_base = WidgetBase::builder()
        .title("Network Columns")
        .theme(widget_theme)
        .build();

    let items = build_check_list_items(default_columns, &label_registry);

    CheckList::builder()
        .id(NETWORK_COLUMNS_DIALOG_ID)
        .widget_base(widget_base)
        .theme(check_list_theme)
        .items(items)
        .on_change(on_change(tx.clone()))
        .build()
        .into()
}

/// All candidate columns: every builtin, then every defined label column.
fn candidate_specs(label_registry: &[NetworkLabelColumn]) -> Vec<NetworkColumnSpec> {
    NetworkColumn::iter()
        .map(NetworkColumnSpec::Builtin)
        .chain(label_registry.iter().map(|lc| {
            NetworkColumnSpec::Label {
                key: lc.key.clone(),
                header: lc.header.clone(),
            }
        }))
        .collect()
}

fn build_check_list_items(
    default_columns: NetworkColumns,
    label_registry: &[NetworkLabelColumn],
) -> Vec<CheckListItem> {
    let candidates = candidate_specs(label_registry);
    let current = default_columns;

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

fn make_item(spec: &NetworkColumnSpec, checked: bool) -> CheckListItem {
    CheckListItem {
        label: spec.header(),
        checked,
        required: matches!(
            spec,
            NetworkColumnSpec::Builtin(NetworkColumn::Kind)
                | NetworkColumnSpec::Builtin(NetworkColumn::Name)
        ),
        metadata: Some(metadata_for(spec)),
    }
}

fn metadata_for(spec: &NetworkColumnSpec) -> BTreeMap<String, String> {
    match spec {
        NetworkColumnSpec::Builtin(c) => {
            BTreeMap::from([
                ("kind".to_string(), "builtin".to_string()),
                ("id".to_string(), c.as_str().to_string()),
            ])
        }
        NetworkColumnSpec::Label { key, header } => {
            BTreeMap::from([
                ("kind".to_string(), "label".to_string()),
                ("key".to_string(), key.clone()),
                ("header".to_string(), header.clone()),
            ])
        }
    }
}

fn spec_from_item(item: &CheckListItem) -> Option<NetworkColumnSpec> {
    let md = item.metadata.as_ref()?;
    match md.get("kind").map(String::as_str) {
        Some("builtin") => {
            NetworkColumn::from_str(md.get("id")?)
                .ok()
                .map(NetworkColumnSpec::Builtin)
        }
        Some("label") => {
            Some(NetworkColumnSpec::Label {
                key: md.get("key")?.clone(),
                header: md.get("header")?.clone(),
            })
        }
        _ => None,
    }
}

fn collect_columns(items: &[CheckListItem]) -> NetworkColumns {
    let specs: Vec<NetworkColumnSpec> = items
        .iter()
        .filter(|item| item.required || item.checked)
        .filter_map(spec_from_item)
        .collect();

    NetworkColumns::new(specs).ensure_required()
}

fn on_change(tx: Sender<Message>) -> impl Fn(&mut Window, &CheckListItem) -> EventResult {
    move |w: &mut Window, _v| {
        let widget = w
            .find_widget_mut(NETWORK_COLUMNS_DIALOG_ID)
            .as_mut_check_list();
        let columns = collect_columns(widget.items());
        tx.send(NetworkMessage::ColumnsRequest(columns).into())
            .expect("Failed to send NetworkMessage::ColumnsRequest");
        EventResult::Nop
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn label_spec(key: &str, header: &str) -> NetworkColumnSpec {
        NetworkColumnSpec::Label {
            key: key.into(),
            header: header.into(),
        }
    }

    #[test]
    fn 選択列を先頭にその他候補を未チェックで並べる() {
        let registry = vec![NetworkLabelColumn {
            name: "app".into(),
            key: "app.kubernetes.io/name".into(),
            header: "APP".into(),
        }];
        let current = NetworkColumns::new([
            NetworkColumnSpec::Builtin(NetworkColumn::Kind),
            NetworkColumnSpec::Builtin(NetworkColumn::Name),
            label_spec("app.kubernetes.io/name", "APP"),
        ]);

        let items = build_check_list_items(current, &registry);

        assert_eq!(items[0].label, "KIND");
        assert!(items[0].checked);
        assert_eq!(items[1].label, "NAME");
        assert!(items[1].checked);
        assert_eq!(items[2].label, "APP");
        assert!(items[2].checked);
        assert!(items[3..].iter().all(|i| !i.checked));
    }

    #[test]
    fn collect_columns_は表示順を維持しensure_requiredが補う() {
        let items = vec![
            make_item(&label_spec("app.kubernetes.io/name", "APP"), true),
            make_item(&NetworkColumnSpec::Builtin(NetworkColumn::Kind), true),
            make_item(&NetworkColumnSpec::Builtin(NetworkColumn::Name), true),
            make_item(&NetworkColumnSpec::Builtin(NetworkColumn::Age), false),
        ];

        let columns = collect_columns(&items);

        assert_eq!(
            columns.specs(),
            &[
                label_spec("app.kubernetes.io/name", "APP"),
                NetworkColumnSpec::Builtin(NetworkColumn::Kind),
                NetworkColumnSpec::Builtin(NetworkColumn::Name),
            ]
        );
    }

    #[test]
    fn メタデータからspecを復元できる() {
        let builtin = make_item(&NetworkColumnSpec::Builtin(NetworkColumn::Age), true);
        let label = make_item(&label_spec("k", "APP"), true);

        assert_eq!(
            spec_from_item(&builtin),
            Some(NetworkColumnSpec::Builtin(NetworkColumn::Age))
        );
        assert_eq!(spec_from_item(&label), Some(label_spec("k", "APP")));
    }

    #[test]
    fn kind_と_name_は_required() {
        let kind = make_item(&NetworkColumnSpec::Builtin(NetworkColumn::Kind), true);
        let name = make_item(&NetworkColumnSpec::Builtin(NetworkColumn::Name), true);
        let age = make_item(&NetworkColumnSpec::Builtin(NetworkColumn::Age), true);
        let label = make_item(&label_spec("k", "APP"), true);

        assert!(kind.required);
        assert!(name.required);
        assert!(!age.required);
        assert!(!label.required);
    }
}
