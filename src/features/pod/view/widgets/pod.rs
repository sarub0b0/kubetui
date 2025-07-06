use crossbeam::channel::Sender;

use crate::{
    config::theme::WidgetThemeConfig,
    features::{
        component_id::{
            POD_COLUMNS_DIALOG_ID, POD_LOG_QUERY_WIDGET_ID, POD_LOG_WIDGET_ID, POD_WIDGET_ID,
        },
        pod::{
            kube::{LogConfig, LogPrefixType},
            message::LogMessage,
        },
    },
    kube::context::Namespace,
    message::Message,
    ui::{
        Window, WindowAction,
        event::EventResult,
        widget::{
            FilterForm, FilterFormTheme, Item, Table, TableItem, TableTheme, Widget, WidgetBase,
            WidgetTheme, WidgetTrait as _,
        },
    },
};

pub fn pod_widget(tx: &Sender<Message>, theme: WidgetThemeConfig) -> Widget<'static> {
    let tx = tx.clone();

    let widget_theme = WidgetTheme::from(theme.clone());
    let table_theme = TableTheme::from(theme.clone());

    let widget_base = WidgetBase::builder()
        .title("Pod")
        .theme(widget_theme)
        .build();

    let filter_form_theme = FilterFormTheme::from(theme.clone());

    let filter_form = FilterForm::builder().theme(filter_form_theme).build();

    Table::builder()
        .id(POD_WIDGET_ID)
        .widget_base(widget_base)
        .filter_form(filter_form)
        .theme(table_theme)
        .filtered_key("NAME")
        .action('t', open_pod_columns_dialog())
        .block_injection(block_injection())
        .on_select(on_select(tx))
        .build()
        .into()
}

fn open_pod_columns_dialog() -> impl Fn(&mut Window) -> EventResult {
    move |w: &mut Window| {
        w.open_dialog(POD_COLUMNS_DIALOG_ID);

        EventResult::Nop
    }
}

fn block_injection() -> impl Fn(&Table) -> WidgetBase {
    |table: &Table| {
        let index = if let Some(index) = table.state().selected() {
            index + 1
        } else {
            0
        };

        let mut base = table.widget_base().clone();

        *base.append_title_mut() = Some(format!(" [{}/{}]", index, table.items().len()).into());

        base
    }
}

fn on_select(tx: Sender<Message>) -> impl Fn(&mut Window, &TableItem) -> EventResult {
    move |w: &mut Window, v: &TableItem| {
        w.widget_clear(POD_LOG_WIDGET_ID);

        let Some(ref metadata) = v.metadata else {
            return EventResult::Ignore;
        };

        let Some(ref namespace) = metadata.get("namespace") else {
            return EventResult::Ignore;
        };

        let Some(ref name) = metadata.get("name") else {
            return EventResult::Ignore;
        };

        let query_form = w.find_widget_mut(POD_LOG_QUERY_WIDGET_ID);

        query_form.update_widget_item(Item::Single(format!("pod/{name}").into()));

        let namespaces = Namespace(vec![namespace.to_string()]);

        let config = LogConfig::new(
            format!("pod/{name}"),
            namespaces.to_owned(),
            LogPrefixType::OnlyContainer,
            false,
        );

        tx.send(LogMessage::Request(config).into())
            .expect("Failed to send LogMessage::Request");

        EventResult::WindowAction(WindowAction::Continue)
    }
}
