use crossbeam::channel::Sender;

use crate::{
    config::theme::ThemeConfig,
    features::{
        component_id::{
            API_WIDGET_ID, CONFIG_RAW_DATA_WIDGET_ID, CONFIG_WIDGET_ID, EVENT_WIDGET_ID,
            MULTIPLE_NAMESPACES_DIALOG_ID, NETWORK_DESCRIPTION_WIDGET_ID, NETWORK_WIDGET_ID,
            POD_LOG_QUERY_WIDGET_ID, POD_LOG_WIDGET_ID, POD_WIDGET_ID, YAML_WIDGET_ID,
        },
        namespace::message::NamespaceRequest,
    },
    message::Message,
    ui::{
        event::EventResult,
        widget::{
            multiple_select::{
                FilterForm, FilterFormTheme, MultipleSelectTheme, SelectForm, SelectFormTheme,
            },
            LiteralItem, MultipleSelect, Widget, WidgetBase, WidgetTheme,
        },
        Window,
    },
};

pub struct MultipleNamespacesDialog {
    pub widget: Widget<'static>,
}

impl MultipleNamespacesDialog {
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
    let multiple_select_theme =
        MultipleSelectTheme::default().status_style(theme.component.list.status);

    let filter_form = FilterForm::builder().theme(filter_theme).build();

    let select_form = SelectForm::builder()
        .theme(select_theme)
        .on_select_selected(on_select(tx.clone()))
        .on_select_unselected(on_select(tx))
        .build();

    let widget_base = WidgetBase::builder()
        .title("Namespace")
        .theme(widget_theme)
        .build();

    MultipleSelect::builder()
        .id(MULTIPLE_NAMESPACES_DIALOG_ID)
        .widget_base(widget_base)
        .filter_form(filter_form)
        .select_form(select_form)
        .theme(multiple_select_theme)
        .build()
        .into()
}

fn on_select(
    tx: Sender<Message>,
) -> impl Fn(&mut Window, &LiteralItem) -> EventResult + 'static + Clone {
    move |w: &mut Window, _| {
        let widget = w
            .find_widget_mut(MULTIPLE_NAMESPACES_DIALOG_ID)
            .as_mut_multiple_select();

        let mut items: Vec<String> = widget
            .selected_items()
            .iter()
            .map(|i| i.item.to_string())
            .collect();

        if items.is_empty() {
            items = vec!["None".to_string()];
        }

        tx.send(NamespaceRequest::Set(items).into())
            .expect("Failed to send NamespaceRequest::Set");

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

        EventResult::Nop
    }
}
