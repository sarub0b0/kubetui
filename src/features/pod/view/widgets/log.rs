use std::{cell::RefCell, rc::Rc};

use ratatui::{crossterm::event::KeyCode, widgets::Block};

use crate::{
    clipboard::Clipboard,
    features::component_id::POD_LOG_WIDGET_ID,
    message::UserEvent,
    ui::{
        event::EventResult,
        widget::{config::WidgetConfig, Item, Text, Widget, WidgetTrait as _},
        Window,
    },
};

pub fn log_widget(clipboard: &Option<Rc<RefCell<Clipboard>>>) -> Widget<'static> {
    let builder = Text::builder()
        .id(POD_LOG_WIDGET_ID)
        .widget_config(&WidgetConfig::builder().title("Log").build())
        .wrap()
        .follow()
        .block_injection(block_injection())
        .action(UserEvent::from(KeyCode::Enter), add_blankline());

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

        *config.title_mut() = format!("Log [{}/{}]", index, size).into();

        config.render_block(text.can_activate() && is_active, is_mouse_over)
    }
}

fn add_blankline() -> impl Fn(&mut Window) -> EventResult {
    move |w: &mut Window| {
        let w = w.find_widget_mut(POD_LOG_WIDGET_ID);

        w.select_last();
        w.append_widget_item(Item::Single(Default::default()));

        EventResult::Nop
    }
}
