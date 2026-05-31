use indoc::indoc;
use ratatui::crossterm::event::KeyCode;

use crate::{
    config::theme::WidgetThemeConfig,
    features::component_id::CONFIG_FILTER_HELP_DIALOG_ID,
    message::UserEvent,
    ui::{
        event::EventResult,
        widget::{SearchForm, SearchFormTheme, Text, TextTheme, Widget, WidgetBase, WidgetTheme},
        Window,
    },
};

pub fn config_filter_help_widget(theme: WidgetThemeConfig) -> Widget<'static> {
    let widget_theme = WidgetTheme::from(theme.clone());
    let text_theme = TextTheme::from(theme.clone());
    let search_theme = SearchFormTheme::from(theme);

    let widget_base = WidgetBase::builder()
        .title("Config Filter Help")
        .theme(widget_theme)
        .build();

    let search_form = SearchForm::builder().theme(search_theme).build();

    Text::builder()
        .id(CONFIG_FILTER_HELP_DIALOG_ID)
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
           NAME:<regex>       Include rows where NAME matches.
           KIND:<regex>       Include where KIND matches (ConfigMap, Secret).
                              Multiple same-column includes are OR (in-list).
           !<COL>:<regex>     Exclude rows whose COL matches.
           label:<selector>   Kubernetes labelSelector, applied
                              server-side (e.g. app=nginx,env=prod).
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
           cm-                             NAME contains 'cm-'
           KIND:ConfigMap                  Only ConfigMaps
           !KIND:Secret                    Exclude Secrets
           NAME:web KIND:ConfigMap         AND across columns
           label:app=nginx                 Server-side label filter

        Columns are the builtin Config columns (NAME / KIND / DATA / AGE);
        unknown columns produce an error. Column names ignore case, spaces,
        '-' and '_'. The 'namespace' column is not filterable — use the
        namespace selector. Press Enter to apply, Esc to cancel. Type ? or
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
