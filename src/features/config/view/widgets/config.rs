use crossbeam::channel::Sender;

use crate::{
    action::view_id,
    features::config::message::{ConfigRequest, RequestData},
    message::Message,
    ui::{
        event::EventResult,
        widget::{config::WidgetConfig, Table, TableItem, Widget, WidgetTrait as _},
        Window, WindowEvent,
    },
};

pub fn config_widget(tx: &Sender<Message>) -> Widget<'static> {
    let tx = tx.clone();

    Table::builder()
        .id(view_id::tab_config_widget_config)
        .widget_config(&WidgetConfig::builder().title("Config").build())
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
    move |w, v| {
        w.widget_clear(view_id::tab_config_widget_raw_data);

        let Some(metadata) = v.metadata.as_ref() else {
            return EventResult::Ignore;
        };

        let Some(namespace) = metadata.get("namespace") else {
            return EventResult::Ignore;
        };

        let Some(name) = metadata.get("name") else {
            return EventResult::Ignore;
        };

        let Some(kind) = metadata.get("kind") else {
            return EventResult::Ignore;
        };

        *(w.find_widget_mut(view_id::tab_config_widget_raw_data)
            .widget_config_mut()
            .append_title_mut()) = Some((format!(" : {}", name)).into());

        let request_data = RequestData {
            namespace: namespace.to_string(),
            name: name.to_string(),
        };

        match kind.as_str() {
            "ConfigMap" => {
                tx.send(ConfigRequest::ConfigMap(request_data).into())
                    .expect("Failed to ConfigRequest::ConfigMap");
            }
            "Secret" => {
                tx.send(ConfigRequest::Secret(request_data).into())
                    .expect("Failed to send ConfigRequest::Secret");
            }
            _ => {}
        }

        EventResult::Window(WindowEvent::Continue)
    }
}
