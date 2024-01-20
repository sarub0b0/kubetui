use std::{cell::RefCell, rc::Rc};

use crate::{
    action::view_id,
    clipboard::Clipboard,
    ui::widget::{config::WidgetConfig, Text, Widget, WidgetTrait},
};

pub struct YamlPopup {
    pub popup: Widget<'static>,
}

pub struct YamlPopupBuilder<'a> {
    clipboard: &'a Option<Rc<RefCell<Clipboard>>>,
}

impl<'a> YamlPopupBuilder<'a> {
    pub fn new(clipboard: &'a Option<Rc<RefCell<Clipboard>>>) -> Self {
        Self { clipboard }
    }

    pub fn build(&self) -> YamlPopup {
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

        if let Some(clipboard) = self.clipboard {
            builder = builder.clipboard(clipboard.clone());
        }

        YamlPopup {
            popup: builder.build().into(),
        }
    }
}
