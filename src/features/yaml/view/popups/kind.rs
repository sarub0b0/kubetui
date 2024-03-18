use crossbeam::channel::Sender;

use crate::{
    action::view_id,
    features::yaml::message::YamlRequest,
    logger,
    message::Message,
    ui::{
        event::EventResult,
        widget::{config::WidgetConfig, LiteralItem, SingleSelect, Widget},
        Window,
    },
};

pub fn kind_popup(tx: &Sender<Message>) -> Widget<'static> {
    let tx = tx.clone();

    SingleSelect::builder()
        .id(view_id::popup_yaml_kind)
        .widget_config(&WidgetConfig::builder().title("Kind").build())
        .on_select(on_select(tx))
        .build()
        .into()
}

fn on_select(tx: Sender<Message>) -> impl Fn(&mut Window, &LiteralItem) -> EventResult {
    move |w, v| {
        logger!(info, "Select Item: {:?}", v);

        w.close_popup();

        let Some(metadata) = v.metadata.as_ref() else {
            unreachable!()
        };

        let Some(key) = metadata.get("key") else {
            unreachable!()
        };

        let Ok(kind) = serde_json::from_str(key) else {
            unreachable!()
        };

        tx.send(YamlRequest::Resource(kind).into())
            .expect("Failed to send YamlRequest::Resource");

        EventResult::Nop
    }
}
