use std::{collections::BTreeMap, str::FromStr as _};

use crossbeam::channel::Sender;
use strum::IntoEnumIterator;

use crate::{
    config::theme::WidgetThemeConfig,
    features::{
        component_id::CONFIG_COLUMNS_DIALOG_ID,
        config::{
            message::ConfigMessage,
            ConfigColumn,
            ConfigColumnSpec,
            ConfigColumns,
            ConfigLabelColumn,
        },
    },
    message::Message,
    ui::{
        event::EventResult,
        widget::{CheckList, CheckListItem, CheckListTheme, Widget, WidgetBase, WidgetTheme},
        Window,
    },
};

pub fn config_columns_dialog(
    tx: &Sender<Message>,
    default_columns: ConfigColumns,
    label_registry: Vec<ConfigLabelColumn>,
    theme: WidgetThemeConfig,
) -> Widget<'static> {
    let check_list_theme = CheckListTheme::from(theme.clone());
    let widget_theme = WidgetTheme::from(theme.clone());
    let widget_base = WidgetBase::builder()
        .title("Config Columns")
        .theme(widget_theme)
        .build();

    let items = build_check_list_items(default_columns, &label_registry);

    CheckList::builder()
        .id(CONFIG_COLUMNS_DIALOG_ID)
        .widget_base(widget_base)
        .theme(check_list_theme)
        .items(items)
        .on_change(on_change(tx.clone()))
        .build()
        .into()
}

/// All candidate columns: every builtin, then every defined label column.
fn candidate_specs(label_registry: &[ConfigLabelColumn]) -> Vec<ConfigColumnSpec> {
    ConfigColumn::iter()
        .map(ConfigColumnSpec::Builtin)
        .chain(label_registry.iter().map(|lc| {
            ConfigColumnSpec::Label {
                key: lc.key.clone(),
                header: lc.header.clone(),
            }
        }))
        .collect()
}

fn build_check_list_items(
    default_columns: ConfigColumns,
    label_registry: &[ConfigLabelColumn],
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

fn make_item(spec: &ConfigColumnSpec, checked: bool) -> CheckListItem {
    CheckListItem {
        label: spec.header(),
        checked,
        required: matches!(
            spec,
            ConfigColumnSpec::Builtin(ConfigColumn::Kind)
                | ConfigColumnSpec::Builtin(ConfigColumn::Name)
        ),
        metadata: Some(metadata_for(spec)),
    }
}

fn metadata_for(spec: &ConfigColumnSpec) -> BTreeMap<String, String> {
    match spec {
        ConfigColumnSpec::Builtin(c) => {
            BTreeMap::from([
                ("kind".to_string(), "builtin".to_string()),
                ("id".to_string(), c.as_str().to_string()),
            ])
        }
        ConfigColumnSpec::Label { key, header } => {
            BTreeMap::from([
                ("kind".to_string(), "label".to_string()),
                ("key".to_string(), key.clone()),
                ("header".to_string(), header.clone()),
            ])
        }
    }
}

fn spec_from_item(item: &CheckListItem) -> Option<ConfigColumnSpec> {
    let md = item.metadata.as_ref()?;
    match md.get("kind").map(String::as_str) {
        Some("builtin") => {
            ConfigColumn::from_str(md.get("id")?)
                .ok()
                .map(ConfigColumnSpec::Builtin)
        }
        Some("label") => {
            Some(ConfigColumnSpec::Label {
                key: md.get("key")?.clone(),
                header: md.get("header")?.clone(),
            })
        }
        _ => None,
    }
}

fn collect_columns(items: &[CheckListItem]) -> ConfigColumns {
    let specs: Vec<ConfigColumnSpec> = items
        .iter()
        .filter(|item| item.required || item.checked)
        .filter_map(spec_from_item)
        .collect();

    ConfigColumns::new(specs).ensure_required()
}

fn on_change(tx: Sender<Message>) -> impl Fn(&mut Window, &CheckListItem) -> EventResult {
    move |w: &mut Window, _v| {
        let widget = w
            .find_widget_mut(CONFIG_COLUMNS_DIALOG_ID)
            .as_mut_check_list();
        let columns = collect_columns(widget.items());
        tx.send(ConfigMessage::ColumnsRequest(columns).into())
            .expect("Failed to send ConfigMessage::ColumnsRequest");
        EventResult::Nop
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn label_spec(key: &str, header: &str) -> ConfigColumnSpec {
        ConfigColumnSpec::Label {
            key: key.into(),
            header: header.into(),
        }
    }

    #[test]
    fn 選択列を先頭にその他候補を未チェックで並べる() {
        let registry = vec![ConfigLabelColumn {
            name: "app".into(),
            key: "app.kubernetes.io/name".into(),
            header: "APP".into(),
        }];
        let current = ConfigColumns::new([
            ConfigColumnSpec::Builtin(ConfigColumn::Kind),
            ConfigColumnSpec::Builtin(ConfigColumn::Name),
            label_spec("app.kubernetes.io/name", "APP"),
        ]);

        let items = build_check_list_items(current, &registry);

        assert_eq!(items[0].label, "KIND");
        assert!(items[0].checked);
        assert_eq!(items[1].label, "NAME");
        assert!(items[1].checked);
        assert_eq!(items[2].label, "APP");
        assert!(items[2].checked);
        // Remaining items are unchecked candidates (DATA, AGE).
        assert!(items[3..].iter().all(|i| !i.checked));
    }

    #[test]
    fn collect_columns_は表示順を維持しensure_requiredが補う() {
        let items = vec![
            make_item(&label_spec("app.kubernetes.io/name", "APP"), true),
            make_item(&ConfigColumnSpec::Builtin(ConfigColumn::Kind), true),
            make_item(&ConfigColumnSpec::Builtin(ConfigColumn::Name), true),
            make_item(&ConfigColumnSpec::Builtin(ConfigColumn::Data), false),
        ];

        let columns = collect_columns(&items);

        // Both Kind/Name already present so ensure_required is a no-op for order.
        // APP is first because it appeared first in items.
        assert_eq!(
            columns.specs(),
            &[
                label_spec("app.kubernetes.io/name", "APP"),
                ConfigColumnSpec::Builtin(ConfigColumn::Kind),
                ConfigColumnSpec::Builtin(ConfigColumn::Name),
            ]
        );
    }

    #[test]
    fn メタデータからspecを復元できる() {
        let builtin = make_item(&ConfigColumnSpec::Builtin(ConfigColumn::Data), true);
        let label = make_item(&label_spec("k", "MIG"), true);

        assert_eq!(
            spec_from_item(&builtin),
            Some(ConfigColumnSpec::Builtin(ConfigColumn::Data))
        );
        assert_eq!(spec_from_item(&label), Some(label_spec("k", "MIG")));
    }

    #[test]
    fn kind_と_name_は_required() {
        let kind = make_item(&ConfigColumnSpec::Builtin(ConfigColumn::Kind), true);
        let name = make_item(&ConfigColumnSpec::Builtin(ConfigColumn::Name), true);
        let data = make_item(&ConfigColumnSpec::Builtin(ConfigColumn::Data), true);
        let label = make_item(&label_spec("k", "APP"), true);

        assert!(kind.required);
        assert!(name.required);
        assert!(!data.required);
        assert!(!label.required);
    }
}
