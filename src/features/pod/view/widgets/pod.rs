use crossbeam::channel::Sender;

use crate::{
    action::view_id,
    features::pod::{
        kube::{LogConfig, LogPrefixType},
        message::LogMessage,
    },
    kube::context::Namespace,
    message::Message,
    ui::{
        event::EventResult,
        widget::{config::WidgetConfig, Item, Table, TableItem, Widget, WidgetTrait as _},
        Window, WindowAction,
    },
};

pub fn pod_widget(tx: &Sender<Message>) -> Widget<'static> {
    let tx = tx.clone();

    Table::builder()
        .id(view_id::tab_pod_widget_pod)
        .widget_config(&WidgetConfig::builder().title("Pod").build())
        .filtered_key("NAME")
        .block_injection(block_injection())
        .on_select(on_select(tx))
        .build()
        .into()
}

fn block_injection() -> impl Fn(&Table) -> WidgetConfig {
    |table: &Table| {
        let index = if let Some(index) = table.state().selected() {
            index + 1
        } else {
            0
        };

        let mut widget_config = table.widget_config().clone();

        *widget_config.append_title_mut() =
            Some(format!(" [{}/{}]", index, table.items().len()).into());

        widget_config
    }
}

fn on_select(tx: Sender<Message>) -> impl Fn(&mut Window, &TableItem) -> EventResult {
    move |w: &mut Window, v: &TableItem| {
        w.widget_clear(view_id::tab_pod_widget_log);

        let Some(ref metadata) = v.metadata else {
            return EventResult::Ignore;
        };

        let Some(ref namespace) = metadata.get("namespace") else {
            return EventResult::Ignore;
        };

        let Some(ref name) = metadata.get("name") else {
            return EventResult::Ignore;
        };

        let query_form = w.find_widget_mut(view_id::tab_pod_widget_log_query);

        query_form.update_widget_item(Item::Single(format!("pod/{}", name).into()));

        let namespaces = Namespace(vec![namespace.to_string()]);

        let config = LogConfig::new(
            format!("pod/{}", name),
            namespaces.to_owned(),
            LogPrefixType::OnlyContainer,
        );

        tx.send(LogMessage::Request(config).into())
            .expect("Failed to send LogMessage::Request");

        EventResult::WindowAction(WindowAction::Continue)
    }
}
