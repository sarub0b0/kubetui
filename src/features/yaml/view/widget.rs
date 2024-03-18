use std::{cell::RefCell, rc::Rc};

use crossbeam::channel::Sender;
use ratatui::widgets::Block;

use crate::{
    action::view_id,
    clipboard::Clipboard,
    features::yaml::message::YamlRequest,
    message::Message,
    ui::{
        event::EventResult,
        widget::{config::WidgetConfig, Text, Widget, WidgetTrait as _},
        Window,
    },
};

pub fn yaml_widget(
    tx: &Sender<Message>,
    clipboard: &Option<Rc<RefCell<Clipboard>>>,
) -> Widget<'static> {
    let tx = tx.clone();

    let builder = Text::builder()
        .id(view_id::tab_yaml_widget_yaml)
        .widget_config(&WidgetConfig::builder().title("Yaml").build())
        .block_injection(block_injection())
        .action('f', open_kind_popup(tx))
        .wrap();

    if let Some(cb) = clipboard {
        builder.clipboard(cb.clone())
    } else {
        builder
    }
    .build()
    .into()
}

fn open_kind_popup(tx: Sender<Message>) -> impl Fn(&mut Window) -> EventResult {
    move |w: &mut Window| {
        tx.send(YamlRequest::APIs.into())
            .expect("YamlRequest::APIs");
        w.open_popup(view_id::popup_yaml_kind);
        EventResult::Nop
    }
}

fn block_injection() -> impl Fn(&Text, bool, bool) -> Block<'static> {
    |text: &Text, is_active: bool, is_mouse_over: bool| {
        let (index, size) = text.state();

        let mut config = text.widget_config().clone();

        *config.append_title_mut() = Some(format!(" [{}/{}]", index, size).into());

        config.render_block(text.can_activate() && is_active, is_mouse_over)
    }
}
