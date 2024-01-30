use std::{cell::RefCell, rc::Rc};

use ratatui::widgets::Block;

use crate::{
    action::view_id,
    clipboard::Clipboard,
    ui::widget::{config::WidgetConfig, Text, Widget, WidgetTrait as _},
};

pub fn raw_data_widget(clipboard: &Option<Rc<RefCell<Clipboard>>>) -> Widget<'static> {
    let builder = Text::builder()
        .id(view_id::tab_config_widget_raw_data)
        .widget_config(&WidgetConfig::builder().title("Raw Data").build())
        .wrap()
        .block_injection(block_injection());

    if let Some(cb) = clipboard {
        builder.clipboard(cb.clone())
    } else {
        builder
    }
    .build()
    .into()
}

fn block_injection() -> impl Fn(&Text, bool, bool) -> Block<'static> {
    |text: &Text, is_active: bool, is_mouse_over: bool| {
        let (index, size) = text.state();

        let mut config = text.widget_config().clone();

        *config.title_mut() = format!("Raw Data [{}/{}]", index, size).into();

        config.render_block(text.can_activate() && is_active, is_mouse_over)
    }
}
