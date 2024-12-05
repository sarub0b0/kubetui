use crossbeam::channel::Sender;

use crate::{
    config::theme::WidgetThemeConfig,
    features::{
        api_resources::message::ApiRequest,
        component_id::{LIST_DIALOG_ID, LIST_WIDGET_ID},
    },
    message::Message,
    ui::{
        event::EventResult,
        widget::{
            multiple_select::{
                FilterForm, FilterFormTheme, MultipleSelectTheme, SelectForm, SelectFormTheme,
            },
            LiteralItem, MultipleSelect, SelectedItem, Widget, WidgetBase, WidgetTheme,
            WidgetTrait as _,
        },
        Window,
    },
};

pub fn dialog_widget(tx: &Sender<Message>, theme: WidgetThemeConfig) -> Widget<'static> {
    let tx = tx.clone();

    let widget_theme = WidgetTheme::from(theme.clone());
    let filter_theme = FilterFormTheme::from(theme.clone());
    let select_theme = SelectFormTheme::from(theme.clone());
    let multiple_select_theme = MultipleSelectTheme::default().status_style(theme.list.status);

    let widget_base = WidgetBase::builder()
        .title("List")
        .theme(widget_theme)
        .build();

    let filter_form = FilterForm::builder().theme(filter_theme).build();

    let select_form = SelectForm::builder()
        .theme(select_theme)
        .on_select_selected(on_select(tx.clone()))
        .on_select_unselected(on_select(tx))
        .build();

    MultipleSelect::builder()
        .id(LIST_DIALOG_ID)
        .widget_base(widget_base)
        .filter_form(filter_form)
        .select_form(select_form)
        .theme(multiple_select_theme)
        .build()
        .into()
}

fn on_select(tx: Sender<Message>) -> impl Fn(&mut Window, &LiteralItem) -> EventResult {
    move |w: &mut Window, _| {
        let widget = w.find_widget_mut(LIST_DIALOG_ID).as_mut_multiple_select();

        if let Some(SelectedItem::Array(items)) = widget.widget_item() {
            let list = items
                .iter()
                .map(|item| {
                    let Some(metadata) = &item.metadata else {
                        unreachable!()
                    };

                    let Some(key) = metadata.get("key") else {
                        unreachable!()
                    };

                    let Ok(key) = serde_json::from_str(key) else {
                        unreachable!()
                    };

                    key
                })
                .collect();

            tx.send(ApiRequest::Set(list).into())
                .expect("Failed to send ApiRequest::Set");
        }

        if widget.selected_items().is_empty() {
            w.widget_clear(LIST_WIDGET_ID)
        }

        EventResult::Nop
    }
}
