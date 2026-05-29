use indoc::indoc;
use ratatui::crossterm::event::KeyCode;

use crate::{
    config::theme::WidgetThemeConfig,
    features::component_id::NODE_FILTER_HELP_DIALOG_ID,
    message::UserEvent,
    ui::{
        event::EventResult,
        widget::{SearchForm, SearchFormTheme, Text, TextTheme, Widget, WidgetBase, WidgetTheme},
        Window,
    },
};

pub fn node_filter_help_widget(theme: WidgetThemeConfig) -> Widget<'static> {
    let widget_theme = WidgetTheme::from(theme.clone());
    let text_theme = TextTheme::from(theme.clone());
    let search_theme = SearchFormTheme::from(theme);

    let widget_base = WidgetBase::builder()
        .title("Node Filter Help")
        .theme(widget_theme)
        .build();

    let search_form = SearchForm::builder().theme(search_theme).build();

    Text::builder()
        .id(NODE_FILTER_HELP_DIALOG_ID)
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

        Quoting (values with spaces):
           "value with spaces"           Double-quoted value
           'value with spaces'           Single-quoted value
           \" \' \\                      Literal " ' \ inside quotes
           \<other>                      Backslash preserved (regex \s etc.)

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
           OS-IMAGE:"Ubuntu 22.04 LTS"     Quoted value with whitespace
           NAME:"foo bar"                  Quote bare value too

        Columns must be builtin or defined label columns; unknown
        columns produce an error. A term on a column that is not
        currently shown becomes inactive (kept, but not applied) until
        that column is shown again; the title shows (inactive: ...).
        Column names ignore case, spaces, '-' and '_'. Press Enter to
        apply, Esc to cancel. Type ? or help in the filter input to
        open this help.
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
