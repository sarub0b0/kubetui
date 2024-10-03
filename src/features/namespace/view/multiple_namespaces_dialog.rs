use crossbeam::channel::Sender;

use crate::{
    features::{
        component_id::{
            CONFIG_RAW_DATA_WIDGET_ID, CONFIG_WIDGET_ID, EVENT_WIDGET_ID, LIST_WIDGET_ID,
            MULTIPLE_NAMESPACES_DIALOG_ID, NETWORK_DESCRIPTION_WIDGET_ID, NETWORK_WIDGET_ID,
            POD_LOG_QUERY_WIDGET_ID, POD_LOG_WIDGET_ID, POD_WIDGET_ID, YAML_WIDGET_ID,
        },
        namespace::message::NamespaceRequest,
    },
    message::Message,
    ui::{
        event::EventResult,
        widget::{LiteralItem, MultipleSelect, Widget, WidgetBase},
        Window,
    },
};

pub struct MultipleNamespacesDialog {
    pub widget: Widget<'static>,
}

impl MultipleNamespacesDialog {
    pub fn new(tx: &Sender<Message>) -> Self {
        Self {
            widget: widget(tx.clone()),
        }
    }
}
fn widget(tx: Sender<Message>) -> Widget<'static> {
    MultipleSelect::builder()
        .id(MULTIPLE_NAMESPACES_DIALOG_ID)
        .widget_base(&WidgetBase::builder().title("Namespace").build())
        .on_select(on_select(tx))
        .build()
        .into()
}

fn on_select(
    tx: Sender<Message>,
) -> impl Fn(&mut Window, &LiteralItem) -> EventResult + 'static + Clone {
    move |w: &mut Window, _| {
        let widget = w
            .find_widget_mut(MULTIPLE_NAMESPACES_DIALOG_ID)
            .as_mut_multiple_select();

        let mut items: Vec<String> = widget
            .selected_items()
            .iter()
            .map(|i| i.item.to_string())
            .collect();

        if items.is_empty() {
            items = vec!["None".to_string()];
        }

        tx.send(NamespaceRequest::Set(items).into())
            .expect("Failed to send NamespaceRequest::Set");

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

        EventResult::Nop
    }
}
