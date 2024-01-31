use std::{cell::RefCell, rc::Rc};

use ratatui::widgets::Block;

use crate::{
    action::view_id,
    clipboard::Clipboard,
    ui::widget::{config::WidgetConfig, Text, Widget, WidgetTrait as _},
};

pub fn event_widget(clipboard: &Option<Rc<RefCell<Clipboard>>>) -> Widget<'static> {
    let builder = Text::builder()
        .id(view_id::tab_event_widget_event)
        .widget_config(&WidgetConfig::builder().title("Event").build())
        .wrap()
        .follow()
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

        *config.append_title_mut() = Some(format!(" [{}/{}]", index, size).into());

        config.render_block(text.can_activate() && is_active, is_mouse_over)
    }
}
