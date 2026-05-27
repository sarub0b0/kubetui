use std::str::FromStr as _;

use crossbeam::channel::Sender;
use strum::IntoEnumIterator;

use crate::{
    config::theme::WidgetThemeConfig,
    features::{
        component_id::NODE_COLUMNS_DIALOG_ID,
        node::{message::NodeMessage, NodeColumn, NodeColumns, DEFAULT_NODE_COLUMNS},
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
    theme: WidgetThemeConfig,
) -> Widget<'static> {
    let check_list_theme = CheckListTheme::from(theme.clone());
    let widget_theme = WidgetTheme::from(theme.clone());
    let widget_base = WidgetBase::builder()
        .title("Node Columns")
        .theme(widget_theme)
        .build();

    let check_list_items = build_check_list_items(default_columns);

    CheckList::builder()
        .id(NODE_COLUMNS_DIALOG_ID)
        .widget_base(widget_base)
        .theme(check_list_theme)
        .items(check_list_items)
        .on_change(on_change(tx.clone()))
        .build()
        .into()
}

fn on_change(tx: Sender<Message>) -> impl Fn(&mut Window, &CheckListItem) -> EventResult {
    move |w: &mut Window, _v| {
        let widget = w
            .find_widget_mut(NODE_COLUMNS_DIALOG_ID)
            .as_mut_check_list();

        let items = widget
            .items()
            .iter()
            .filter(|item| item.required || item.checked)
            .filter_map(|i| NodeColumn::from_str(&i.label).ok())
            .collect::<Vec<_>>();

        tx.send(NodeMessage::Request(NodeColumns::new(items)).into())
            .expect("Failed to send NodeMessage::Request");

        EventResult::Nop
    }
}

fn build_check_list_items(default_columns: Option<NodeColumns>) -> Vec<CheckListItem> {
    match default_columns {
        Some(columns) => {
            build_check_list_items_from_existing(columns.ensure_name_column().dedup_columns())
        }
        None => build_default_check_list_items(),
    }
}

fn build_check_list_items_from_existing(node_columns: NodeColumns) -> Vec<CheckListItem> {
    node_columns
        .columns()
        .iter()
        .map(|column| make_item(*column, true))
        .chain(
            NodeColumn::iter()
                .filter(|c| !node_columns.columns().contains(c))
                .map(|column| make_item(column, false)),
        )
        .collect()
}

fn build_default_check_list_items() -> Vec<CheckListItem> {
    NodeColumn::iter()
        .map(|column| {
            let checked = DEFAULT_NODE_COLUMNS.contains(&column);
            make_item(column, checked)
        })
        .collect()
}

fn make_item(column: NodeColumn, checked: bool) -> CheckListItem {
    CheckListItem {
        label: column.display().to_string(),
        checked,
        required: column == NodeColumn::Name,
        metadata: None,
    }
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case)]
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn 既存カラムをチェック済みで先頭に_残りを未チェックで並べる() {
        let node_columns = NodeColumns::new([NodeColumn::Name, NodeColumn::Status]);
        let items = build_check_list_items_from_existing(node_columns);

        assert_eq!(items[0].label, "NAME");
        assert!(items[0].checked && items[0].required);
        assert_eq!(items[1].label, "STATUS");
        assert!(items[1].checked && !items[1].required);
        assert_eq!(items.len(), NodeColumn::iter().count());
        assert!(items[2..].iter().all(|i| !i.checked));
    }

    #[test]
    fn デフォルトカラムがチェック済みで構築される() {
        let items = build_default_check_list_items();
        let checked: Vec<&str> = items
            .iter()
            .filter(|i| i.checked)
            .map(|i| i.label.as_str())
            .collect();
        assert_eq!(checked, vec!["NAME", "STATUS", "ROLES", "AGE", "VERSION"]);
        assert!(items.iter().find(|i| i.label == "NAME").unwrap().required);
    }
}
