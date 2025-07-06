use ratatui::crossterm::event::KeyCode;

use crate::{
    config::theme::WidgetThemeConfig,
    features::component_id::{YAML_KIND_DIALOG_ID, YAML_NOT_FOUND_DIALOG_ID},
    ui::{
        Window,
        event::EventResult,
        widget::{SearchForm, SearchFormTheme, Text, TextTheme, Widget, WidgetBase, WidgetTheme},
    },
};

pub fn not_found_dialog(theme: WidgetThemeConfig) -> Widget<'static> {
    let widget_theme = WidgetTheme::from(theme.clone());
    let search_theme = SearchFormTheme::from(theme.clone());
    let text_theme = TextTheme::from(theme);

    let widget_base = WidgetBase::builder()
        .title("Name")
        .theme(widget_theme)
        .build();

    let search_form = SearchForm::builder().theme(search_theme).build();

    Text::builder()
        .id(YAML_NOT_FOUND_DIALOG_ID)
        .widget_base(widget_base)
        .search_form(search_form)
        .theme(text_theme)
        .items(
            [
                "No resources found.",
                "",
                "Press \x1b[1mEnter\x1b[0m or \x1b[1mEsc\x1b[0m to return to resource selection.",
            ]
            .into_iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        )
        .wrap()
        .action(KeyCode::Enter, open_kind_dialog())
        .action(KeyCode::Esc, open_kind_dialog())
        .build()
        .into()
}

fn open_kind_dialog() -> impl Fn(&mut Window) -> EventResult {
    move |w: &mut Window| {
        w.open_dialog(YAML_KIND_DIALOG_ID);

        if let Widget::SingleSelect(w) = w.find_widget_mut(YAML_KIND_DIALOG_ID) {
            w.clear_filter();
        }

        EventResult::Nop
    }
}
