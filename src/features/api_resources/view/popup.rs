use crossbeam::channel::Sender;

use crate::{
    features::{
        api_resources::message::ApiRequest,
        component_id::{LIST_POPUP_ID, LIST_WIDGET_ID},
    },
    message::Message,
    ui::{
        event::EventResult,
        widget::{
            config::WidgetConfig, LiteralItem, MultipleSelect, SelectedItem, Widget,
            WidgetTrait as _,
        },
        Window,
    },
};

pub fn popup_widget(tx: &Sender<Message>) -> Widget<'static> {
    let tx = tx.clone();

    MultipleSelect::builder()
        .id(LIST_POPUP_ID)
        .widget_config(&WidgetConfig::builder().title("List").build())
        .on_select(move |w: &mut Window, _: &LiteralItem| {
            let widget = w.find_widget_mut(LIST_POPUP_ID).as_mut_multiple_select();

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
        })
        .build()
        .into()
}
