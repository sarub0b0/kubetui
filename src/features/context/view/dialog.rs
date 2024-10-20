use crossbeam::channel::Sender;

use crate::{
    features::{
        component_id::{
            CONFIG_RAW_DATA_WIDGET_ID, CONFIG_WIDGET_ID, CONTEXT_DIALOG_ID, EVENT_WIDGET_ID,
            LIST_DIALOG_ID, LIST_WIDGET_ID, MULTIPLE_NAMESPACES_DIALOG_ID,
            NETWORK_DESCRIPTION_WIDGET_ID, NETWORK_WIDGET_ID, POD_LOG_QUERY_WIDGET_ID,
            POD_LOG_WIDGET_ID, POD_WIDGET_ID, YAML_WIDGET_ID,
        },
        context::message::ContextRequest,
    },
    message::Message,
    ui::{
        event::EventResult,
        widget::{single_select::SelectForm, LiteralItem, SingleSelect, Widget, WidgetBase},
        Window,
    },
};

pub struct ContextDialog {
    pub widget: Widget<'static>,
}

impl ContextDialog {
    pub fn new(tx: &Sender<Message>) -> Self {
        Self {
            widget: widget(tx.clone()),
        }
    }
}

fn widget(tx: Sender<Message>) -> Widget<'static> {
    let select_form = SelectForm::builder().on_select(on_select(tx)).build();

    SingleSelect::builder()
        .id(CONTEXT_DIALOG_ID)
        .widget_base(WidgetBase::builder().title("Context").build())
        .select_form(select_form)
        .build()
        .into()
}

fn on_select(tx: Sender<Message>) -> impl Fn(&mut Window, &LiteralItem) -> EventResult {
    move |w, v| {
        let item = v.item.to_string();

        tx.send(ContextRequest::Set(item).into())
            .expect("Failed to send ContextRequest::Set");

        w.close_dialog();

        w.widget_clear(POD_WIDGET_ID);
        w.widget_clear(POD_LOG_WIDGET_ID);
        w.widget_clear(POD_LOG_QUERY_WIDGET_ID);
        w.widget_clear(CONFIG_WIDGET_ID);
        w.widget_clear(CONFIG_RAW_DATA_WIDGET_ID);
        w.widget_clear(NETWORK_WIDGET_ID);
        w.widget_clear(NETWORK_DESCRIPTION_WIDGET_ID);
        w.widget_clear(EVENT_WIDGET_ID);
        w.widget_clear(LIST_WIDGET_ID);
        w.widget_clear(YAML_WIDGET_ID);

        let widget = w
            .find_widget_mut(MULTIPLE_NAMESPACES_DIALOG_ID)
            .as_mut_multiple_select();

        widget.unselect_all();

        let widget = w.find_widget_mut(LIST_DIALOG_ID).as_mut_multiple_select();

        widget.unselect_all();

        EventResult::Nop
    }
}
