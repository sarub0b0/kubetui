use std::{cell::RefCell, rc::Rc};

use ratatui::widgets::Block;

use crate::{
    clipboard::Clipboard,
    features::component_id::EVENT_WIDGET_ID,
    ui::widget::{base::WidgetBase, Text, Widget, WidgetTrait as _},
};

pub fn event_widget(clipboard: &Option<Rc<RefCell<Clipboard>>>) -> Widget<'static> {
    let builder = Text::builder()
        .id(EVENT_WIDGET_ID)
        .widget_base(&WidgetBase::builder().title("Event").build())
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

        let mut base = text.widget_base().clone();

        *base.append_title_mut() = Some(format!(" [{}/{}]", index, size).into());

        base.render_block(text.can_activate() && is_active, is_mouse_over)
    }
}
