use std::{cell::RefCell, rc::Rc};

use crate::{
    clipboard::Clipboard,
    config::theme::ThemeConfig,
    features::component_id::YAML_DIALOG_ID,
    ui::widget::{
        SearchForm, SearchFormTheme, Text, TextTheme, Widget, WidgetBase, WidgetTheme, WidgetTrait,
    },
};

pub struct YamlDialog {
    pub widget: Widget<'static>,
}

impl YamlDialog {
    pub fn new(clipboard: &Option<Rc<RefCell<Clipboard>>>, theme: ThemeConfig) -> Self {
        Self {
            widget: widget(clipboard, theme),
        }
    }
}

fn widget(clipboard: &Option<Rc<RefCell<Clipboard>>>, theme: ThemeConfig) -> Widget<'static> {
    let widget_theme = WidgetTheme::from(theme.component.clone());
    let search_theme = SearchFormTheme::from(theme.component.clone());
    let text_theme = TextTheme::from(theme.component);

    let widget_base = WidgetBase::builder()
        .title("Yaml")
        .theme(widget_theme)
        .build();

    let search_form = SearchForm::builder().theme(search_theme).build();

    let mut builder = Text::builder()
        .id(YAML_DIALOG_ID)
        .widget_base(widget_base)
        .search_form(search_form)
        .theme(text_theme)
        .block_injection(|text: &Text, is_active: bool, is_mouse_over: bool| {
            let (index, size) = text.state();

            let mut base = text.widget_base().clone();

            *base.title_mut() = format!("Yaml [{}/{}]", index, size).into();

            base.render_block(text.can_activate() && is_active, is_mouse_over)
        })
        .wrap();

    if let Some(clipboard) = clipboard {
        builder = builder.clipboard(clipboard.clone());
    }

    builder.build().into()
}
