use crossbeam::channel::Sender;

use crate::{
    features::{
        component_id::{
            API_WIDGET_ID, CONFIG_RAW_DATA_WIDGET_ID, CONFIG_WIDGET_ID, EVENT_WIDGET_ID,
            MULTIPLE_NAMESPACES_DIALOG_ID, NETWORK_DESCRIPTION_WIDGET_ID, NETWORK_WIDGET_ID,
            POD_LOG_QUERY_WIDGET_ID, POD_LOG_WIDGET_ID, POD_WIDGET_ID, SINGLE_NAMESPACE_DIALOG_ID,
            YAML_WIDGET_ID,
        },
        namespace::message::NamespaceRequest,
    },
    message::Message,
    ui::{
        event::EventResult,
        widget::{single_select::SelectForm, LiteralItem, SingleSelect, Widget, WidgetBase},
        Window,
    },
};

pub struct SingleNamespaceDialog {
    pub widget: Widget<'static>,
}

impl SingleNamespaceDialog {
    pub fn new(tx: &Sender<Message>) -> Self {
        Self {
            widget: widget(tx.clone()),
        }
    }
}

fn widget(tx: Sender<Message>) -> Widget<'static> {
    let select_form = SelectForm::builder()
        .on_select(on_select(tx.clone()))
        .build();

    SingleSelect::builder()
        .id(SINGLE_NAMESPACE_DIALOG_ID)
        .widget_base(WidgetBase::builder().title("Namespace").build())
        .select_form(select_form)
        .build()
        .into()
}

fn on_select(tx: Sender<Message>) -> impl Fn(&mut Window, &LiteralItem) -> EventResult {
    move |w: &mut Window, v| {
        let items = vec![v.item.to_string()];
        tx.send(NamespaceRequest::Set(items).into())
            .expect("Failed to send NamespaceRequest::Set");

        w.close_dialog();

        w.widget_clear(POD_WIDGET_ID);
        w.widget_clear(POD_LOG_WIDGET_ID);
        w.widget_clear(POD_LOG_QUERY_WIDGET_ID);
        w.widget_clear(CONFIG_WIDGET_ID);
        w.widget_clear(CONFIG_RAW_DATA_WIDGET_ID);
        w.widget_clear(NETWORK_WIDGET_ID);
        w.widget_clear(NETWORK_DESCRIPTION_WIDGET_ID);
        w.widget_clear(EVENT_WIDGET_ID);
        w.widget_clear(API_WIDGET_ID);
        w.widget_clear(YAML_WIDGET_ID);

        let widget = w
            .find_widget_mut(MULTIPLE_NAMESPACES_DIALOG_ID)
            .as_mut_multiple_select();

        widget.unselect_all();

        widget.select_item(v);

        EventResult::Nop
    }
}
