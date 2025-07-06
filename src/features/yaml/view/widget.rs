use std::{cell::RefCell, rc::Rc};

use crossbeam::channel::Sender;
use ratatui::widgets::Block;

use crate::{
    clipboard::Clipboard,
    config::theme::WidgetThemeConfig,
    features::{
        component_id::{YAML_KIND_DIALOG_ID, YAML_WIDGET_ID},
        yaml::message::YamlRequest,
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

pub fn yaml_widget(
    tx: &Sender<Message>,
    clipboard: &Option<Rc<RefCell<Clipboard>>>,
    theme: WidgetThemeConfig,
) -> Widget<'static> {
    let tx = tx.clone();

    let widget_theme = WidgetTheme::from(theme.clone());
    let search_theme = SearchFormTheme::from(theme.clone());
    let text_theme = TextTheme::from(theme);

    let widget_base = WidgetBase::builder()
        .title("Yaml")
        .theme(widget_theme)
        .build();

    let search_form = SearchForm::builder().theme(search_theme).build();

    let builder = Text::builder()
        .id(YAML_WIDGET_ID)
        .widget_base(widget_base)
        .search_form(search_form)
        .theme(text_theme)
        .block_injection(block_injection())
        .action('f', open_kind_dialog(tx))
        .wrap();

    if let Some(cb) = clipboard {
        builder.clipboard(cb.clone())
    } else {
        builder
    }
    .build()
    .into()
}

fn open_kind_dialog(tx: Sender<Message>) -> impl Fn(&mut Window) -> EventResult {
    move |w: &mut Window| {
        tx.send(YamlRequest::APIs.into())
            .expect("YamlRequest::APIs");
        w.open_dialog(YAML_KIND_DIALOG_ID);
        EventResult::Nop
    }
}

fn block_injection() -> impl Fn(&Text, bool, bool) -> Block<'static> {
    |text: &Text, is_active: bool, is_mouse_over: bool| {
        let (index, size) = text.state();

        let mut base = text.widget_base().clone();

        *base.append_title_mut() = Some(format!(" [{index}/{size}]").into());

        base.render_block(text.can_activate() && is_active, is_mouse_over)
    }
}
