use std::{cell::RefCell, rc::Rc};

use crossbeam::channel::Sender;

use crate::{
    clipboard::Clipboard,
    features::{
        api_resources::message::ApiRequest,
        component_id::{LIST_POPUP_ID, LIST_WIDGET_ID},
    },
    message::Message,
    ui::{
        event::EventResult,
        widget::{config::WidgetConfig, Text, Widget, WidgetTrait as _},
        Window,
    },
};

pub fn list_widget(
    tx: &Sender<Message>,
    clipboard: &Option<Rc<RefCell<Clipboard>>>,
) -> Widget<'static> {
    let tx = tx.clone();

    let open_subwin = move |w: &mut Window| {
        tx.send(ApiRequest::Get.into())
            .expect("Failed to send ApiRequest::Get");
        w.open_popup(LIST_POPUP_ID);
        EventResult::Nop
    };

    let builder = Text::builder()
        .id(LIST_WIDGET_ID)
        .widget_config(&WidgetConfig::builder().title("List").build())
        .block_injection(|text: &Text, is_active: bool, is_mouse_over: bool| {
            let (index, size) = text.state();

            let mut config = text.widget_config().clone();

            *config.append_title_mut() = Some(format!(" [{}/{}]", index, size).into());

            config.render_block(text.can_activate() && is_active, is_mouse_over)
        })
        .action('f', open_subwin);

    if let Some(cb) = clipboard {
        builder.clipboard(cb.clone())
    } else {
        builder
    }
    .build()
    .into()
}
