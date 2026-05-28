use indoc::indoc;
use ratatui::crossterm::event::KeyCode;

use crate::{
    config::theme::WidgetThemeConfig,
    features::component_id::POD_LOG_QUERY_HELP_DIALOG_ID,
    message::UserEvent,
    ui::{
        event::EventResult,
        widget::{SearchForm, SearchFormTheme, Text, TextTheme, Widget, WidgetBase, WidgetTheme},
        Window,
    },
};

pub fn log_query_help_widget(theme: WidgetThemeConfig) -> Widget<'static> {
    let widget_theme = WidgetTheme::from(theme.clone());
    let text_theme = TextTheme::from(theme.clone());
    let search_theme = SearchFormTheme::from(theme);

    let widget_base = WidgetBase::builder()
        .title("Log Query Help")
        .theme(widget_theme)
        .build();

    let search_form = SearchForm::builder().theme(search_theme).build();

    Text::builder()
        .id(POD_LOG_QUERY_HELP_DIALOG_ID)
        .widget_base(widget_base)
        .search_form(search_form)
        .theme(text_theme)
        .items(content())
        .action(UserEvent::from(KeyCode::Enter), close_dialog())
        .build()
        .into()
}

fn content() -> Vec<String> {
    indoc! {r#"
        Usage: QUERY [ QUERY ]...

        Queries:
           pod:<regex>           (alias: pods, po, p)
           !pod:<regex>          (alias: !pods, !po, p)
           container:<regex>     (alias: containers, co, c)
           !container:<regex>    (alias: !containers, !co, !c)
           log:<regex>           (alias: logs, lo, l)
           !log:<regex>          (alias: !logs, !lo, !l)
           label:<selector>      (alias: labels)
           field:<selector>      (alias: fields)
           jq:<expr>
           jmespath:<expr>       (alias: jmes, jm)
           limit:<number>        (alias: lim)
           <resource>/<name>

        Resources:
           pod            (alias: pods, po)
           replicaset     (alias: replicasets, rs)
           deployment     (alias: deployments, deploy)
           statefulset    (alias: statefulsets, sts)
           daemonset      (alias: daemonsets, ds)
           service        (alias: services, svc)
           job            (alias: jobs)
    "# }
    .lines()
    .map(ToString::to_string)
    .collect()
}

fn close_dialog() -> impl Fn(&mut Window) -> EventResult {
    move |w: &mut Window| {
        w.close_dialog();
        EventResult::Nop
    }
}
