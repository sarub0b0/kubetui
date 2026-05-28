use crossbeam::channel::Sender;

use crate::{
    config::theme::WidgetThemeConfig,
    features::{
        component_id::{NODE_COLUMNS_DIALOG_ID, NODE_DETAIL_WIDGET_ID, NODE_WIDGET_ID},
        node::{
            filter::node_filter_applicator,
            message::NodeDetailMessage,
            node_columns::NodeLabelColumn,
        },
    },
    message::Message,
    ui::{
        event::EventResult,
        widget::{
            FilterForm,
            FilterFormTheme,
            Table,
            TableItem,
            TableTheme,
            Widget,
            WidgetBase,
            WidgetTheme,
            WidgetTrait as _,
        },
        Window,
    },
};

pub fn node_widget(
    tx: Sender<Message>,
    label_registry: Vec<NodeLabelColumn>,
    theme: WidgetThemeConfig,
) -> Widget<'static> {
    let widget_theme = WidgetTheme::from(theme.clone());
    let table_theme = TableTheme::from(theme.clone());

    let widget_base = WidgetBase::builder()
        .title("Node")
        .theme(widget_theme)
        .build();

    let filter_form_theme = FilterFormTheme::from(theme.clone());
    let filter_form = FilterForm::builder().theme(filter_form_theme).build();

    Table::builder()
        .id(NODE_WIDGET_ID)
        .widget_base(widget_base)
        .filter_form(filter_form)
        .filter_applicator(node_filter_applicator(label_registry, tx.clone()))
        .theme(table_theme)
        .action('t', open_node_columns_dialog())
        .on_select(on_select(tx))
        .build()
        .into()
}

fn open_node_columns_dialog() -> impl Fn(&mut Window) -> EventResult {
    move |w: &mut Window| {
        w.open_dialog(NODE_COLUMNS_DIALOG_ID);
        EventResult::Nop
    }
}

/// Pure helper: turn the selected `TableItem` into a detail-fetch request.
/// Returns `None` if the row's `name` metadata is missing.
fn build_detail_request(item: &TableItem) -> Option<NodeDetailMessage> {
    let name = item.metadata.as_ref()?.get("name")?.clone();
    Some(NodeDetailMessage::Request { name })
}

fn on_select(tx: Sender<Message>) -> impl Fn(&mut Window, &TableItem) -> EventResult {
    move |w: &mut Window, item: &TableItem| {
        // Clear the previous node's detail; new content arrives on the next
        // worker tick.
        w.widget_clear(NODE_DETAIL_WIDGET_ID);

        let Some(req) = build_detail_request(item) else {
            return EventResult::Ignore;
        };

        // Show the selected node name in the detail pane title (matches the
        // Network description pane idiom).
        if let NodeDetailMessage::Request { name } = &req {
            *(w.find_widget_mut(NODE_DETAIL_WIDGET_ID)
                .widget_base_mut()
                .append_title_mut()) = Some((format!(" : {}", name)).into());
        }

        tx.send(req.into())
            .expect("Failed to send NodeDetailMessage::Request");
        EventResult::Nop
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn build_detail_request_uses_name_metadata() {
        let item = TableItem {
            item: vec!["node-a".to_string()],
            metadata: Some(BTreeMap::from([("name".to_string(), "node-a".to_string())])),
        };

        let req = build_detail_request(&item).expect("name metadata should be present");
        match req {
            NodeDetailMessage::Request { name } => assert_eq!(name, "node-a"),
            _ => panic!("expected Request"),
        }
    }

    #[test]
    fn build_detail_request_returns_none_without_name() {
        let item = TableItem {
            item: vec![],
            metadata: None,
        };
        assert!(build_detail_request(&item).is_none());
    }
}
