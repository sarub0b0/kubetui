use indoc::indoc;
use ratatui::crossterm::event::KeyCode;

use crate::{
    features::component_id::POD_LOG_QUERY_HELP_POPUP_ID,
    message::UserEvent,
    ui::{
        event::EventResult,
        widget::{config::WidgetConfig, Text, Widget},
        Window,
    },
};

pub fn log_query_help_widget() -> Widget<'static> {
    Text::builder()
        .id(POD_LOG_QUERY_HELP_POPUP_ID)
        .widget_config(&WidgetConfig::builder().title("Log Query Help").build())
        .items(content())
        .action(UserEvent::from(KeyCode::Enter), close_popup())
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

fn close_popup() -> impl Fn(&mut Window) -> EventResult {
    move |w: &mut Window| {
        w.close_popup();
        EventResult::Nop
    }
}
