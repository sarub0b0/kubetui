use indoc::indoc;
use ratatui::crossterm::event::KeyCode;

use crate::{
    features::component_id::NODE_FILTER_HELP_DIALOG_ID,
    message::UserEvent,
    ui::{
        event::EventResult,
        widget::{Text, Widget, WidgetBase},
        Window,
    },
};

pub fn node_filter_help_widget() -> Widget<'static> {
    Text::builder()
        .id(NODE_FILTER_HELP_DIALOG_ID)
        .widget_base(WidgetBase::builder().title("Node Filter Help").build())
        .items(content())
        .action(UserEvent::from(KeyCode::Enter), close_dialog())
        .build()
        .into()
}

fn content() -> Vec<String> {
    indoc! {r#"
        Usage: TERM [ TERM ]...

        Terms:
           <value>            Plain value: NAME include (regex).
           NAME:<regex>       Include nodes where NAME matches.
           STATUS:<regex>     Include where STATUS matches. Multiple
                              same-column includes are OR (in-list).
           !<COL>:<regex>     Exclude nodes whose COL matches.
           label:<selector>   Kubernetes labelSelector, applied
                              server-side (e.g. role=worker,zone=us-west).
                              Last 'label:' wins if repeated.

        Combining:
           Same column, multiple includes  ->  OR (in-list)
           Different columns, includes     ->  AND across columns
           Any matching exclude            ->  row excluded
           Bare values                     ->  treated as NAME includes

        Examples
           worker                          Show nodes whose NAME matches 'worker'
           NAME:gke STATUS:Ready           NAME~gke AND STATUS~Ready
           STATUS:Ready STATUS:Pending     STATUS in (Ready, Pending)
           !NAME:control label:zone=us     Server-side label filter + name exclude

        Column names are case-insensitive. Unknown columns produce a
        parse error. Press Enter to apply, Esc to cancel. Type ? or
        help in the filter input to open this help.
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
