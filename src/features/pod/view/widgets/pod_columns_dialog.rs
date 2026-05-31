use std::{collections::BTreeMap, str::FromStr as _};

use crossbeam::channel::Sender;
use strum::IntoEnumIterator;

use crate::{
    config::theme::WidgetThemeConfig,
    features::{
        component_id::POD_COLUMNS_DIALOG_ID,
        pod::{
            message::PodMessage, PodColumn, PodColumnSpec, PodColumns, PodLabelColumn,
        },
    },
    message::Message,
    ui::{
        event::EventResult,
        widget::{CheckList, CheckListItem, CheckListTheme, Widget, WidgetBase, WidgetTheme},
        Window,
    },
};

pub fn pod_columns_dialog(
    tx: &Sender<Message>,
    default_columns: Option<PodColumns>,
    label_registry: Vec<PodLabelColumn>,
    theme: WidgetThemeConfig,
) -> Widget<'static> {
    let check_list_theme = CheckListTheme::from(theme.clone());
    let widget_theme = WidgetTheme::from(theme.clone());
    let widget_base = WidgetBase::builder()
        .title("Pod Columns")
        .theme(widget_theme)
        .build();

    let check_list_items = build_check_list_items(default_columns, &label_registry);

    CheckList::builder()
        .id(POD_COLUMNS_DIALOG_ID)
        .widget_base(widget_base)
        .theme(check_list_theme)
        .items(check_list_items)
        .on_change(on_change(tx.clone()))
        .build()
        .into()
}

/// All candidate columns: every builtin, then every defined label column.
fn candidate_specs(label_registry: &[PodLabelColumn]) -> Vec<PodColumnSpec> {
    PodColumn::iter()
        .map(PodColumnSpec::Builtin)
        .chain(label_registry.iter().map(|lc| {
            PodColumnSpec::Label {
                key: lc.key.clone(),
                header: lc.header.clone(),
            }
        }))
        .collect()
}

/// Build the checklist items: the currently-active columns first (in their
/// order, checked), then the remaining candidates (unchecked). Each item
/// carries its `PodColumnSpec` in `metadata` so the selection can be rebuilt
/// from the items alone — independent of any reordering done in the dialog.
fn build_check_list_items(
    default_columns: Option<PodColumns>,
    label_registry: &[PodLabelColumn],
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

fn make_item(spec: &PodColumnSpec, checked: bool) -> CheckListItem {
    CheckListItem {
        label: spec.header(),
        checked,
        required: matches!(spec, PodColumnSpec::Builtin(PodColumn::Name)),
        metadata: Some(metadata_for(spec)),
    }
}

fn metadata_for(spec: &PodColumnSpec) -> BTreeMap<String, String> {
    match spec {
        PodColumnSpec::Builtin(c) => {
            BTreeMap::from([
                ("kind".to_string(), "builtin".to_string()),
                ("id".to_string(), c.as_str().to_string()),
            ])
        }
        PodColumnSpec::Label { key, header } => {
            BTreeMap::from([
                ("kind".to_string(), "label".to_string()),
                ("key".to_string(), key.clone()),
                ("header".to_string(), header.clone()),
            ])
        }
    }
}

fn spec_from_item(item: &CheckListItem) -> Option<PodColumnSpec> {
    let md = item.metadata.as_ref()?;
    match md.get("kind").map(String::as_str) {
        Some("builtin") => {
            PodColumn::from_str(md.get("id")?)
                .ok()
                .map(PodColumnSpec::Builtin)
        }
        Some("label") => {
            Some(PodColumnSpec::Label {
                key: md.get("key")?.clone(),
                header: md.get("header")?.clone(),
            })
        }
        _ => None,
    }
}

/// Collect the selected columns from the checklist items, preserving the
/// items' current display order (which the user can reorder in the dialog).
fn collect_columns(items: &[CheckListItem]) -> PodColumns {
    let specs: Vec<PodColumnSpec> = items
        .iter()
        .filter(|item| item.required || item.checked)
        .filter_map(spec_from_item)
        .collect();

    PodColumns::new(specs).ensure_name_column()
}

fn on_change(tx: Sender<Message>) -> impl Fn(&mut Window, &CheckListItem) -> EventResult {
    move |w: &mut Window, _v| {
        let widget = w.find_widget_mut(POD_COLUMNS_DIALOG_ID).as_mut_check_list();

        let columns = collect_columns(widget.items());

        tx.send(PodMessage::Request(columns).into())
            .expect("Failed to send PodColumnsRequest::Set");

        EventResult::Nop
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn label_spec(key: &str, header: &str) -> PodColumnSpec {
        PodColumnSpec::Label {
            key: key.into(),
            header: header.into(),
        }
    }

    #[test]
    fn 選択列を先頭にその他候補を未チェックで並べる() {
        let registry = vec![PodLabelColumn {
            name: "version".into(),
            key: "app.kubernetes.io/version".into(),
            header: "VERSION".into(),
        }];
        let current = PodColumns::new([
            PodColumnSpec::Builtin(PodColumn::Name),
            label_spec("app.kubernetes.io/version", "VERSION"),
        ]);

        let items = build_check_list_items(Some(current), &registry);

        // 先頭2件は選択済み(NAME, VERSION)、以降は未チェック。
        assert_eq!(items[0].label, "NAME");
        assert!(items[0].checked);
        assert_eq!(items[1].label, "VERSION");
        assert!(items[1].checked);
        assert!(items[2..].iter().all(|i| !i.checked));
    }

    #[test]
    fn 並べ替え後も表示順どおりに列を収集する() {
        let name = make_item(&PodColumnSpec::Builtin(PodColumn::Name), true);
        let version = make_item(&label_spec("app.kubernetes.io/version", "VERSION"), true);
        let status = make_item(&PodColumnSpec::Builtin(PodColumn::Status), false);

        // 表示順: VERSION, NAME, STATUS(未チェック)
        let reordered = vec![version, name, status];

        let columns = collect_columns(&reordered);

        // STATUS は混入せず、表示順(VERSION, NAME)どおりに収集される。
        assert_eq!(
            columns.specs(),
            &[
                label_spec("app.kubernetes.io/version", "VERSION"),
                PodColumnSpec::Builtin(PodColumn::Name),
            ]
        );
    }

    #[test]
    fn メタデータからspecを復元できる() {
        let builtin = make_item(&PodColumnSpec::Builtin(PodColumn::IP), true);
        let label = make_item(&label_spec("k", "MIG"), true);

        assert_eq!(
            spec_from_item(&builtin),
            Some(PodColumnSpec::Builtin(PodColumn::IP))
        );
        assert_eq!(spec_from_item(&label), Some(label_spec("k", "MIG")));
    }
}
