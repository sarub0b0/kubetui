use std::str::FromStr as _;

use crossbeam::channel::Sender;
use strum::IntoEnumIterator;

use crate::{
    config::theme::WidgetThemeConfig,
    features::{
        component_id::POD_COLUMNS_DIALOG_ID,
        pod::{PodColumn, PodColumns, message::PodMessage, pod_columns::DEFAULT_POD_COLUMNS},
    },
    message::Message,
    ui::{
        Window,
        event::EventResult,
        widget::{CheckList, CheckListItem, CheckListTheme, Widget, WidgetBase, WidgetTheme},
    },
};

pub fn pod_columns_dialog(
    tx: &Sender<Message>,
    default_columns: Option<PodColumns>,
    theme: WidgetThemeConfig,
) -> Widget<'static> {
    let check_list_theme = CheckListTheme::from(theme.clone());
    let widget_theme = WidgetTheme::from(theme.clone());
    let widget_base = WidgetBase::builder()
        .title("Pod Columns")
        .theme(widget_theme)
        .build();

    let check_list_items = build_check_list_items(default_columns);

    CheckList::builder()
        .id(POD_COLUMNS_DIALOG_ID)
        .widget_base(widget_base)
        .theme(check_list_theme)
        .items(check_list_items)
        .on_change(on_change(tx.clone()))
        .build()
        .into()
}

fn on_change(tx: Sender<Message>) -> impl Fn(&mut Window, &CheckListItem) -> EventResult {
    move |w: &mut Window, _v| {
        let widget = w.find_widget_mut(POD_COLUMNS_DIALOG_ID).as_mut_check_list();

        let items = widget
            .items()
            .iter()
            .filter(|item| item.required || item.checked)
            .filter_map(|i| PodColumn::from_str(&i.label).ok())
            .collect::<Vec<_>>();

        tx.send(PodMessage::Request(PodColumns::new(items)).into())
            .expect("Failed to send PodColumnsRequest::Set");

        EventResult::Nop
    }
}

fn build_check_list_items(default_columns: Option<PodColumns>) -> Vec<CheckListItem> {
    match default_columns {
        Some(columns) => {
            build_check_list_items_from_existing(columns.ensure_name_column().dedup_columns())
        }
        None => build_default_check_list_items(),
    }
}

fn build_check_list_items_from_existing(pod_columns: PodColumns) -> Vec<CheckListItem> {
    pod_columns
        .columns()
        .iter()
        .map(|column| make_item(*column, true))
        .chain(
            PodColumn::iter()
                .filter(|c| !pod_columns.columns().contains(c))
                .map(|column| make_item(column, false)),
        )
        .collect()
}

fn build_default_check_list_items() -> Vec<CheckListItem> {
    PodColumn::iter()
        .map(|column| {
            let checked = DEFAULT_POD_COLUMNS.contains(&column);

            make_item(column, checked)
        })
        .collect()
}

fn make_item(column: PodColumn, checked: bool) -> CheckListItem {
    CheckListItem {
        label: column.display().to_string(),
        checked,
        required: column == PodColumn::Name,
        metadata: None,
    }
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case)]

    use super::*;

    mod build_check_list_items_from_existing {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn ユーザーが指定したカラムをチェック済みで最初に配置して残りのカラムを未チェック状態で追加する(
        ) {
            let pod_columns =
                PodColumns::new([PodColumn::Name, PodColumn::Ready, PodColumn::Status]);
            let columns = build_check_list_items_from_existing(pod_columns);

            let expected: Vec<CheckListItem> = vec![
                CheckListItem {
                    label: "NAME".to_string(),
                    checked: true,
                    required: true,
                    metadata: None,
                },
                CheckListItem {
                    label: "READY".to_string(),
                    checked: true,
                    required: false,
                    metadata: None,
                },
                CheckListItem {
                    label: "STATUS".to_string(),
                    checked: true,
                    required: false,
                    metadata: None,
                },
                CheckListItem {
                    label: "RESTARTS".to_string(),
                    checked: false,
                    required: false,
                    metadata: None,
                },
                CheckListItem {
                    label: "AGE".to_string(),
                    checked: false,
                    required: false,
                    metadata: None,
                },
                CheckListItem {
                    label: "IP".to_string(),
                    checked: false,
                    required: false,
                    metadata: None,
                },
                CheckListItem {
                    label: "NODE".to_string(),
                    checked: false,
                    required: false,
                    metadata: None,
                },
                CheckListItem {
                    label: "NOMINATED NODE".to_string(),
                    checked: false,
                    required: false,
                    metadata: None,
                },
                CheckListItem {
                    label: "READINESS GATES".to_string(),
                    checked: false,
                    required: false,
                    metadata: None,
                },
            ];

            assert_eq!(columns, expected);
        }
    }

    mod build_default_check_list_items {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn デフォルトのカラムがチェック済みの状態で構築できる() {
            let columns = build_default_check_list_items();
            let expected: Vec<CheckListItem> = vec![
                CheckListItem {
                    label: "NAME".to_string(),
                    checked: true,
                    required: true,
                    metadata: None,
                },
                CheckListItem {
                    label: "READY".to_string(),
                    checked: true,
                    required: false,
                    metadata: None,
                },
                CheckListItem {
                    label: "STATUS".to_string(),
                    checked: true,
                    required: false,
                    metadata: None,
                },
                CheckListItem {
                    label: "RESTARTS".to_string(),
                    checked: false,
                    required: false,
                    metadata: None,
                },
                CheckListItem {
                    label: "AGE".to_string(),
                    checked: true,
                    required: false,
                    metadata: None,
                },
                CheckListItem {
                    label: "IP".to_string(),
                    checked: false,
                    required: false,
                    metadata: None,
                },
                CheckListItem {
                    label: "NODE".to_string(),
                    checked: false,
                    required: false,
                    metadata: None,
                },
                CheckListItem {
                    label: "NOMINATED NODE".to_string(),
                    checked: false,
                    required: false,
                    metadata: None,
                },
                CheckListItem {
                    label: "READINESS GATES".to_string(),
                    checked: false,
                    required: false,
                    metadata: None,
                },
            ];

            assert_eq!(columns, expected);
        }
    }
}
