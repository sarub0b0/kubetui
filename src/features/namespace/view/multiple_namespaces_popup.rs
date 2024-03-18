use crossbeam::channel::Sender;

use crate::{
    action::view_id,
    features::namespace::message::NamespaceRequest,
    message::Message,
    ui::{
        event::EventResult,
        widget::{config::WidgetConfig, LiteralItem, MultipleSelect, Widget},
        Window,
    },
};

pub struct MultipleNamespacesPopup {
    pub popup: Widget<'static>,
}

impl MultipleNamespacesPopup {
    pub fn new(tx: &Sender<Message>) -> Self {
        Self {
            popup: popup(tx.clone()),
        }
    }
}
fn popup(tx: Sender<Message>) -> Widget<'static> {
    MultipleSelect::builder()
        .id(view_id::popup_ns)
        .widget_config(&WidgetConfig::builder().title("Namespace").build())
        .on_select(on_select(tx))
        .build()
        .into()
}

fn on_select(
    tx: Sender<Message>,
) -> impl Fn(&mut Window, &LiteralItem) -> EventResult + 'static + Clone {
    move |w: &mut Window, _| {
        let widget = w
            .find_widget_mut(view_id::popup_ns)
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

        w.widget_clear(view_id::tab_pod_widget_log);
        w.widget_clear(view_id::tab_pod_widget_log_query);
        w.widget_clear(view_id::tab_config_widget_raw_data);
        w.widget_clear(view_id::tab_network_widget_description);
        w.widget_clear(view_id::tab_event_widget_event);
        w.widget_clear(view_id::tab_list_widget_list);
        w.widget_clear(view_id::tab_yaml_widget_yaml);

        EventResult::Nop
    }
}
