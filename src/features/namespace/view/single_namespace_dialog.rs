use crossbeam::channel::Sender;

use crate::{
    config::theme::ThemeConfig,
    features::{
        component_id::{
            API_WIDGET_ID, CONFIG_RAW_DATA_WIDGET_ID, CONFIG_WIDGET_ID, EVENT_WIDGET_ID,
            MULTIPLE_NAMESPACES_DIALOG_ID, NETWORK_DESCRIPTION_WIDGET_ID, NETWORK_WIDGET_ID,
            POD_LOG_QUERY_WIDGET_ID, POD_LOG_WIDGET_ID, POD_WIDGET_ID, SINGLE_NAMESPACE_DIALOG_ID,
            YAML_WIDGET_ID,
        },
        namespace::message::NamespaceRequest,
    },
    message::Message,
    ui::{
        event::EventResult,
        widget::{
            single_select::{
                FilterForm, FilterFormTheme, SelectForm, SelectFormTheme, SingleSelectTheme,
            },
            LiteralItem, SingleSelect, Widget, WidgetBase, WidgetTheme,
        },
        Window,
    },
};

pub struct SingleNamespaceDialog {
    pub widget: Widget<'static>,
}

impl SingleNamespaceDialog {
    pub fn new(tx: &Sender<Message>, theme: ThemeConfig) -> Self {
        Self {
            widget: widget(tx.clone(), theme),
        }
    }
}

fn widget(tx: Sender<Message>, theme: ThemeConfig) -> Widget<'static> {
    let widget_theme = WidgetTheme::from(theme.component.clone());
    let filter_theme = FilterFormTheme::from(theme.component.clone());
    let select_theme = SelectFormTheme::from(theme.component.clone());
    let single_select_theme =
        SingleSelectTheme::default().status_style(theme.component.list.status);

    let filter_form = FilterForm::builder().theme(filter_theme).build();

    let select_form = SelectForm::builder()
        .on_select(on_select(tx.clone()))
        .theme(select_theme)
        .build();

    let widget_base = WidgetBase::builder()
        .title("Namespace")
        .theme(widget_theme)
        .build();

    SingleSelect::builder()
        .id(SINGLE_NAMESPACE_DIALOG_ID)
        .widget_base(widget_base)
        .filter_form(filter_form)
        .select_form(select_form)
        .theme(single_select_theme)
        .build()
        .into()
}

fn on_select(tx: Sender<Message>) -> impl Fn(&mut Window, &LiteralItem) -> EventResult {
    move |w: &mut Window, v| {
        let items = vec![v.item.to_string()];
        tx.send(NamespaceRequest::Set(items).into())
            .expect("Failed to send NamespaceRequest::Set");

        w.close_dialog();

        w.widget_clear(POD_WIDGET_ID);
        w.widget_clear(POD_LOG_WIDGET_ID);
        w.widget_clear(POD_LOG_QUERY_WIDGET_ID);
        w.widget_clear(CONFIG_WIDGET_ID);
        w.widget_clear(CONFIG_RAW_DATA_WIDGET_ID);
        w.widget_clear(NETWORK_WIDGET_ID);
        w.widget_clear(NETWORK_DESCRIPTION_WIDGET_ID);
        w.widget_clear(EVENT_WIDGET_ID);
        w.widget_clear(API_WIDGET_ID);
        w.widget_clear(YAML_WIDGET_ID);

        let widget = w
            .find_widget_mut(MULTIPLE_NAMESPACES_DIALOG_ID)
            .as_mut_multiple_select();

        widget.unselect_all();

        widget.select_item(v);

        EventResult::Nop
    }
}
