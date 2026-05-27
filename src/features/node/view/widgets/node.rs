use crate::{
    config::theme::WidgetThemeConfig,
    features::component_id::{NODE_COLUMNS_DIALOG_ID, NODE_WIDGET_ID},
    ui::{
        event::EventResult,
        widget::{
            substring_applicator,
            FilterForm,
            FilterFormTheme,
            Table,
            TableTheme,
            Widget,
            WidgetBase,
            WidgetTheme,
        },
        Window,
    },
};

pub fn node_widget(theme: WidgetThemeConfig) -> Widget<'static> {
    let widget_theme = WidgetTheme::from(theme.clone());
    let table_theme = TableTheme::from(theme.clone());

    let widget_base = WidgetBase::builder()
        .title("Node")
        .theme(widget_theme)
        .build();

    let filter_form_theme = FilterFormTheme::from(theme.clone());
    let filter_form = FilterForm::builder().theme(filter_form_theme).build();

    Table::builder()
        .id(NODE_WIDGET_ID)
        .widget_base(widget_base)
        .filter_form(filter_form)
        .filter_applicator(substring_applicator("NAME"))
        .theme(table_theme)
        .action('t', open_node_columns_dialog())
        .build()
        .into()
}

fn open_node_columns_dialog() -> impl Fn(&mut Window) -> EventResult {
    move |w: &mut Window| {
        w.open_dialog(NODE_COLUMNS_DIALOG_ID);
        EventResult::Nop
    }
}
