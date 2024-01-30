use std::{cell::RefCell, rc::Rc};

use crossbeam::channel::Sender;
use crossterm::event::KeyCode;

use crate::{
    action::view_id,
    context::Namespace,
    features::pod::{
        kube::{LogConfig, LogPrefixType},
        message::LogMessage,
    },
    message::{Message, UserEvent},
    ui::{
        event::EventResult,
        widget::{
            config::WidgetConfig, input::InputFormBuilder, SelectedItem, Widget, WidgetTrait as _,
        },
        Window,
    },
};

pub fn log_query_widget(tx: &Sender<Message>, namespaces: Rc<RefCell<Namespace>>) -> Widget<'static> {
    let tx = tx.clone();

    InputFormBuilder::default()
        .id(view_id::tab_pod_widget_log_query)
        .widget_config(WidgetConfig::builder().title("Log Query").build())
        .actions(UserEvent::from(KeyCode::Enter), exec_query(tx, namespaces))
        .build()
        .into()
}

fn exec_query(
    tx: Sender<Message>,
    namespaces: Rc<RefCell<Namespace>>,
) -> impl Fn(&mut Window) -> EventResult {
    move |w: &mut Window| {
        let widget = w.find_widget_mut(view_id::tab_pod_widget_log_query);

        let Some(SelectedItem::Literal { metadata: _, item }) = widget.widget_item() else {
            return EventResult::Ignore;
        };

        if item == "?" || item == "help" {
            widget.clear();
            w.open_popup(view_id::tab_pod_widget_log_query_help);
            return EventResult::Nop;
        }

        w.widget_clear(view_id::tab_pod_widget_log);

        let namespaces = namespaces.borrow();

        let prefix_type = if 1 < namespaces.len() {
            LogPrefixType::All
        } else {
            LogPrefixType::PodAndContainer
        };

        let config = LogConfig::new(item, namespaces.to_owned(), prefix_type);

        tx.send(LogMessage::Request(config).into())
            .expect("Failed to send LogMessage::Request");

        EventResult::Ignore
    }
}
