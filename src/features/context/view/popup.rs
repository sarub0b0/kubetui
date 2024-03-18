use crossbeam::channel::Sender;

use crate::{
    action::view_id,
    features::context::message::ContextRequest,
    message::Message,
    ui::{
        event::EventResult,
        widget::{config::WidgetConfig, LiteralItem, SingleSelect, Widget},
        Window,
    },
};

pub struct ContextPopup {
    pub popup: Widget<'static>,
}

impl ContextPopup {
    pub fn new(tx: &Sender<Message>) -> Self {
        Self {
            popup: popup(tx.clone()),
        }
    }
}

fn popup(tx: Sender<Message>) -> Widget<'static> {
    SingleSelect::builder()
        .id(view_id::popup_ctx)
        .widget_config(&WidgetConfig::builder().title("Context").build())
        .on_select(on_select(tx))
        .build()
        .into()
}

fn on_select(tx: Sender<Message>) -> impl Fn(&mut Window, &LiteralItem) -> EventResult {
    move |w, v| {
        let item = v.item.to_string();

        tx.send(ContextRequest::Set(item).into())
            .expect("Failed to send ContextRequest::Set");

        w.close_popup();

        w.widget_clear(view_id::tab_pod_widget_log);
        w.widget_clear(view_id::tab_pod_widget_log_query);
        w.widget_clear(view_id::tab_config_widget_raw_data);
        w.widget_clear(view_id::tab_network_widget_description);
        w.widget_clear(view_id::tab_event_widget_event);
        w.widget_clear(view_id::tab_list_widget_list);
        w.widget_clear(view_id::tab_yaml_widget_yaml);

        let widget = w
            .find_widget_mut(view_id::popup_ns)
            .as_mut_multiple_select();

        widget.unselect_all();

        let widget = w
            .find_widget_mut(view_id::popup_list)
            .as_mut_multiple_select();

        widget.unselect_all();

        EventResult::Nop
    }
}
