use crossbeam::channel::Sender;
use crossterm::event::KeyCode;

use crate::{
    action::view_id,
    features::yaml::{kube::select::SelectedYaml, message::YamlRequest},
    logger,
    message::Message,
    ui::{
        event::EventResult,
        widget::{config::WidgetConfig, LiteralItem, SingleSelect, Widget},
        Window,
    },
};

pub fn name_popup(tx: &Sender<Message>) -> Widget<'static> {
    let tx = tx.clone();

    SingleSelect::builder()
        .id(view_id::popup_yaml_name)
        .widget_config(&WidgetConfig::builder().title("Name").build())
        .on_select(on_select(tx))
        .action(KeyCode::Esc, open_kind_popup())
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

        let Some(namespace) = metadata.get("namespace") else {
            unreachable!()
        };

        let Some(name) = metadata.get("name") else {
            unreachable!()
        };

        let Some(key) = metadata.get("key") else {
            unreachable!()
        };

        let Ok(kind) = serde_json::from_str(key) else {
            unreachable!()
        };

        tx.send(
            YamlRequest::SelectedYaml(SelectedYaml {
                kind,
                name: name.to_string(),
                namespace: namespace.to_string(),
            })
            .into(),
        )
        .expect("Failed to send YamlRequest::Yaml");

        EventResult::Nop
    }
}

fn open_kind_popup() -> impl Fn(&mut Window) -> EventResult {
    move |w: &mut Window| {
        w.open_popup(view_id::popup_yaml_kind);
        if let Widget::SingleSelect(w) = w.find_widget_mut(view_id::popup_yaml_kind) {
            w.clear_filter();
        }
        EventResult::Nop
    }
}
