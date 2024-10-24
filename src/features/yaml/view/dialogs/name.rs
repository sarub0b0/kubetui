use crossbeam::channel::Sender;
use ratatui::crossterm::event::KeyCode;

use crate::{
    features::{
        component_id::{YAML_KIND_DIALOG_ID, YAML_NAME_DIALOG_ID},
        yaml::message::{YamlRequest, YamlTarget},
    },
    logger,
    message::Message,
    ui::{
        event::EventResult,
        widget::{single_select::SelectForm, LiteralItem, SingleSelect, Widget, WidgetBase},
        Window,
    },
};

pub fn name_dialog(tx: &Sender<Message>) -> Widget<'static> {
    let tx = tx.clone();

    let select_form = SelectForm::builder().on_select(on_select(tx)).build();

    SingleSelect::builder()
        .id(YAML_NAME_DIALOG_ID)
        .widget_base(WidgetBase::builder().title("Name").build())
        .select_form(select_form)
        .action(KeyCode::Esc, open_kind_dialog())
        .build()
        .into()
}

fn on_select(tx: Sender<Message>) -> impl Fn(&mut Window, &LiteralItem) -> EventResult {
    move |w, v| {
        logger!(info, "Select Item: {:?}", v);

        w.close_dialog();

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
            YamlRequest::Yaml(YamlTarget {
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

fn open_kind_dialog() -> impl Fn(&mut Window) -> EventResult {
    move |w: &mut Window| {
        w.open_dialog(YAML_KIND_DIALOG_ID);
        if let Widget::SingleSelect(w) = w.find_widget_mut(YAML_KIND_DIALOG_ID) {
            w.clear_filter();
        }
        EventResult::Nop
    }
}
