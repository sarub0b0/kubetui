use std::{cell::RefCell, rc::Rc};

use crossbeam::channel::Sender;

use crate::{
    clipboard::Clipboard,
    config::theme::WidgetThemeConfig,
    features::{
        api_resources::message::ApiRequest,
        component_id::{API_DIALOG_ID, API_WIDGET_ID},
    },
    message::Message,
    ui::{
        Window,
        event::EventResult,
        widget::{
            SearchForm, SearchFormTheme, Text, TextTheme, Widget, WidgetBase, WidgetTheme,
            WidgetTrait as _,
        },
    },
};

pub fn api_widget(
    tx: &Sender<Message>,
    clipboard: &Option<Rc<RefCell<Clipboard>>>,
    theme: WidgetThemeConfig,
) -> Widget<'static> {
    let tx = tx.clone();

    let open_subwin = move |w: &mut Window| {
        tx.send(ApiRequest::Get.into())
            .expect("Failed to send ApiRequest::Get");
        w.open_dialog(API_DIALOG_ID);
        EventResult::Nop
    };

    let widget_theme = WidgetTheme::from(theme.clone());
    let search_theme = SearchFormTheme::from(theme.clone());
    let text_theme = TextTheme::from(theme);

    let widget_base = WidgetBase::builder()
        .title("API")
        .theme(widget_theme)
        .build();

    let search_form = SearchForm::builder().theme(search_theme).build();

    let builder = Text::builder()
        .id(API_WIDGET_ID)
        .widget_base(widget_base)
        .search_form(search_form)
        .theme(text_theme)
        .block_injection(|text: &Text, is_active: bool, is_mouse_over: bool| {
            let (index, size) = text.state();

            let mut base = text.widget_base().clone();

            *base.append_title_mut() = Some(format!(" [{index}/{size}]").into());

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
