use crossbeam::channel::Sender;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

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
        event::EventResult,
        widget::{
            single_select::{
                FilterForm, FilterFormTheme, SelectForm, SelectFormTheme, SingleSelectTheme,
            },
            LiteralItem, SelectedItem, SingleSelect, Widget, WidgetBase, WidgetTheme,
            WidgetTrait as _,
        },
        Window,
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
        .action(
            KeyEvent::new(KeyCode::Char(' '), KeyModifiers::CONTROL),
            switch_context_with_namespace(tx.clone()),
        )
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

        switch_context(&tx, w, item, false);

        EventResult::Nop
    }
}

fn clear_widgets_and_close_dialog(w: &mut Window) {
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

    w.find_widget_mut(MULTIPLE_NAMESPACES_DIALOG_ID)
        .as_mut_multiple_select()
        .unselect_all();

    w.find_widget_mut(API_DIALOG_ID)
        .as_mut_multiple_select()
        .unselect_all();
}

fn switch_context(
    tx: &Sender<Message>,
    w: &mut Window,
    context_name: String,
    keep_namespace: bool,
) {
    tx.send(
        ContextRequest::Set {
            name: context_name,
            keep_namespace,
        }
        .into(),
    )
    .expect("Failed to send ContextRequest::Set");

    clear_widgets_and_close_dialog(w);
}

fn switch_context_with_namespace(tx: Sender<Message>) -> impl Fn(&mut Window) -> EventResult {
    move |w: &mut Window| {
        let widget = w.find_widget(CONTEXT_DIALOG_ID).as_single_select();

        let Some(selected_item) = widget.widget_item() else {
            crate::logger!(error, "ContextDialog: No item selected");
            return EventResult::Nop;
        };

        let SelectedItem::Literal { item, .. } = selected_item else {
            crate::logger!(error, "ContextDialog: Selected item is not a LiteralItem");
            return EventResult::Nop;
        };

        switch_context(&tx, w, item, true);

        EventResult::Nop
    }
}
