use crossbeam::channel::Sender;

use crate::{
    action::view_id,
    features::network::message::{NetworkRequest, RequestData},
    message::Message,
    ui::{
        event::EventResult,
        widget::{config::WidgetConfig, Table, TableItem, Widget, WidgetTrait as _},
        Window, WindowEvent,
    },
};

pub fn network_widget(tx: &Sender<Message>) -> Widget<'static> {
    let tx = tx.clone();

    Table::builder()
        .id(view_id::tab_network_widget_network)
        .widget_config(&WidgetConfig::builder().title("Network").build())
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
        w.widget_clear(view_id::tab_network_widget_description);

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

        *(w.find_widget_mut(view_id::tab_network_widget_description)
            .widget_config_mut()
            .append_title_mut()) = Some((format!(" : {}", name)).into());

        let request_data = RequestData {
            namespace: namespace.to_string(),
            name: name.to_string(),
        };

        match kind.as_str() {
            "Pod" => {
                tx.send(NetworkRequest::Pod(request_data).into())
                    .expect("Failed to send NetworkRequest::Pod");
            }
            "Service" => {
                tx.send(NetworkRequest::Service(request_data).into())
                    .expect("Failed to send NetworkRequest::Service");
            }
            "Ingress" => {
                tx.send(NetworkRequest::Ingress(request_data).into())
                    .expect("Failed to send NetworkRequest::Ingress");
            }
            "NetworkPolicy" => {
                tx.send(NetworkRequest::NetworkPolicy(request_data).into())
                    .expect("Failed to send NetworkRequest::NetworkPolicy");
            }
            _ => {}
        }

        EventResult::Window(WindowEvent::Continue)
    }
}
