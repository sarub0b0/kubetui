use crossbeam::channel::Sender;

use crate::{
    config::theme::ThemeConfig,
    features::{
        component_id::{
            API_DIALOG_ID, API_WIDGET_ID, CONFIG_RAW_DATA_WIDGET_ID, CONFIG_WIDGET_ID,
            CONTEXT_DIALOG_ID, EVENT_WIDGET_ID, MULTIPLE_NAMESPACES_DIALOG_ID,
            NETWORK_DESCRIPTION_WIDGET_ID, NETWORK_WIDGET_ID, POD_LOG_QUERY_WIDGET_ID,
            POD_LOG_WIDGET_ID, POD_WIDGET_ID, YAML_WIDGET_ID,
        },
        context::message::ContextRequest,
    },
    message::Message,
    ui::{
        Window,
        event::EventResult,
        widget::{
            LiteralItem, SingleSelect, Widget, WidgetBase, WidgetTheme,
            single_select::{
                FilterForm, FilterFormTheme, SelectForm, SelectFormTheme, SingleSelectTheme,
            },
        },
    },
};

pub struct ContextDialog {
    pub widget: Widget<'static>,
}

impl ContextDialog {
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
        .theme(select_theme)
        .on_select(on_select(tx))
        .build();

    let widget_base = WidgetBase::builder()
        .title("Context")
        .theme(widget_theme)
        .build();

    SingleSelect::builder()
        .id(CONTEXT_DIALOG_ID)
        .widget_base(widget_base)
        .filter_form(filter_form)
        .select_form(select_form)
        .theme(single_select_theme)
        .build()
        .into()
}

fn on_select(tx: Sender<Message>) -> impl Fn(&mut Window, &LiteralItem) -> EventResult {
    move |w, v| {
        let item = v.item.to_string();

        tx.send(ContextRequest::Set(item).into())
            .expect("Failed to send ContextRequest::Set");

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

        let widget = w.find_widget_mut(API_DIALOG_ID).as_mut_multiple_select();

        widget.unselect_all();

        EventResult::Nop
    }
}
