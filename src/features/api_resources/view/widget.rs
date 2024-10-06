use std::{cell::RefCell, rc::Rc};

use crossbeam::channel::Sender;

use crate::{
    clipboard::Clipboard,
    features::{
        api_resources::message::ApiRequest,
        component_id::{LIST_DIALOG_ID, LIST_WIDGET_ID},
    },
    message::Message,
    ui::{
        event::EventResult,
        widget::{Text, Widget, WidgetBase, WidgetTrait as _},
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
        w.open_dialog(LIST_DIALOG_ID);
        EventResult::Nop
    };

    let builder = Text::builder()
        .id(LIST_WIDGET_ID)
        .widget_base(WidgetBase::builder().title("List").build())
        .block_injection(|text: &Text, is_active: bool, is_mouse_over: bool| {
            let (index, size) = text.state();

            let mut base = text.widget_base().clone();

            *base.append_title_mut() = Some(format!(" [{}/{}]", index, size).into());

            base.render_block(text.can_activate() && is_active, is_mouse_over)
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
