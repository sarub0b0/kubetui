use crossbeam::channel::Sender;

use crate::{
    features::{
        api_resources::message::ApiRequest,
        component_id::{API_DIALOG_ID, API_WIDGET_ID},
    },
    message::Message,
    ui::{
        event::EventResult,
        widget::{
            multiple_select::SelectForm, LiteralItem, MultipleSelect, SelectedItem, Widget,
            WidgetBase, WidgetTrait as _,
        },
        Window,
    },
};

pub fn dialog_widget(tx: &Sender<Message>) -> Widget<'static> {
    let tx = tx.clone();

    let select_form = SelectForm::builder()
        .on_select_selected(on_select(tx.clone()))
        .on_select_unselected(on_select(tx))
        .build();

    MultipleSelect::builder()
        .id(API_DIALOG_ID)
        .widget_base(WidgetBase::builder().title("API").build())
        .select_form(select_form)
        .build()
        .into()
}

fn on_select(tx: Sender<Message>) -> impl Fn(&mut Window, &LiteralItem) -> EventResult {
    move |w: &mut Window, _| {
        let widget = w.find_widget_mut(API_DIALOG_ID).as_mut_multiple_select();

        if let Some(SelectedItem::Array(items)) = widget.widget_item() {
            let apis = items
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

            tx.send(ApiRequest::Set(apis).into())
                .expect("Failed to send ApiRequest::Set");
        }

        if widget.selected_items().is_empty() {
            w.widget_clear(API_WIDGET_ID)
        }

        EventResult::Nop
    }
}
