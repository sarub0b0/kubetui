use std::{cell::RefCell, rc::Rc};

use crate::{
    action::view_id,
    clipboard::Clipboard,
    ui::widget::{config::WidgetConfig, Text, Widget, WidgetTrait},
};

pub struct YamlPopup {
    pub popup: Widget<'static>,
}

impl YamlPopup {
    pub fn new(clipboard: &Option<Rc<RefCell<Clipboard>>>) -> Self {
        Self {
            popup: popup(clipboard),
        }
    }
}

pub fn popup(clipboard: &Option<Rc<RefCell<Clipboard>>>) -> Widget<'static> {
    let mut builder = Text::builder()
        .id(view_id::popup_yaml)
        .widget_config(&WidgetConfig::builder().title("Yaml").build())
        .block_injection(|text: &Text, is_active: bool, is_mouse_over: bool| {
            let (index, size) = text.state();

            let mut config = text.widget_config().clone();

            *config.title_mut() = format!("Yaml [{}/{}]", index, size).into();

            config.render_block(text.can_activate() && is_active, is_mouse_over)
        })
        .wrap();

    if let Some(clipboard) = clipboard {
        builder = builder.clipboard(clipboard.clone());
    }

    builder.build().into()
}
