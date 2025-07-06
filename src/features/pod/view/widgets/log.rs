use std::{cell::RefCell, rc::Rc};

use crossbeam::channel::Sender;
use ratatui::{crossterm::event::KeyCode, widgets::Block};

use crate::{
    clipboard::Clipboard,
    config::theme::WidgetThemeConfig,
    features::{component_id::POD_LOG_WIDGET_ID, pod::message::LogMessage},
    message::{Message, UserEvent},
    ui::{
        Window,
        event::EventResult,
        widget::{
            Item, SearchForm, SearchFormTheme, Text, TextTheme, Widget, WidgetBase, WidgetTheme,
            WidgetTrait as _,
        },
    },
};

pub fn log_widget(
    tx: &Sender<Message>,
    clipboard: &Option<Rc<RefCell<Clipboard>>>,
    theme: WidgetThemeConfig,
) -> Widget<'static> {
    let widget_theme = WidgetTheme::from(theme.clone());

    let widget_base = WidgetBase::builder()
        .title("Log")
        .theme(widget_theme)
        .build();

    let search_form_theme = SearchFormTheme::from(theme.clone());

    let search_form = SearchForm::builder().theme(search_form_theme).build();

    let text_theme = TextTheme::from(theme);

    let builder = Text::builder()
        .id(POD_LOG_WIDGET_ID)
        .widget_base(widget_base)
        .search_form(search_form)
        .theme(text_theme)
        .wrap()
        .follow()
        .block_injection(block_injection())
        .action(UserEvent::from(KeyCode::Enter), add_blankline())
        .action(
            UserEvent::from(KeyCode::Char('f')),
            toggle_json_pretty_print(tx.clone()),
        )
        .action(
            UserEvent::from(KeyCode::Char('p')),
            toggle_json_pretty_print(tx.clone()),
        );

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

        *base.title_mut() = format!("Log [{index}/{size}]").into();

        base.render_block(text.can_activate() && is_active, is_mouse_over)
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

fn toggle_json_pretty_print(tx: Sender<Message>) -> impl Fn(&mut Window) -> EventResult {
    move |w: &mut Window| {
        let w = w.find_widget_mut(POD_LOG_WIDGET_ID);

        w.clear();

        tx.send(LogMessage::ToggleJsonPrettyPrint.into())
            .expect("Failed to send LogMessage::ToggleJsonPrettyPrint");

        EventResult::Nop
    }
}
