use crossbeam::channel::Sender;

use crate::{
    features::{
        component_id::{CONFIG_RAW_DATA_WIDGET_ID, CONFIG_WIDGET_ID},
        config::message::{ConfigRequest, RequestData},
    },
    message::Message,
    ui::{
        event::EventResult,
        widget::{Table, TableItem, Widget, WidgetBase, WidgetTrait as _},
        Window, WindowAction,
    },
};

pub fn config_widget(tx: &Sender<Message>) -> Widget<'static> {
    let tx = tx.clone();

    Table::builder()
        .id(CONFIG_WIDGET_ID)
        .widget_base(&WidgetBase::builder().title("Config").build())
        .filtered_key("NAME")
        .block_injection(block_injection())
        .on_select(on_select(tx))
        .build()
        .into()
}

fn block_injection() -> impl Fn(&Table) -> WidgetBase {
    |table: &Table| {
        let index = if let Some(index) = table.state().selected() {
            index + 1
        } else {
            0
        };

        let mut base = table.widget_base().clone();

        *base.append_title_mut() = Some(format!(" [{}/{}]", index, table.items().len()).into());

        base
    }
}

fn on_select(tx: Sender<Message>) -> impl Fn(&mut Window, &TableItem) -> EventResult {
    move |w, v| {
        w.widget_clear(CONFIG_RAW_DATA_WIDGET_ID);

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

        *(w.find_widget_mut(CONFIG_RAW_DATA_WIDGET_ID)
            .widget_base_mut()
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

        EventResult::WindowAction(WindowAction::Continue)
    }
}
