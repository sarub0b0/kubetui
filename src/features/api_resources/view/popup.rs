use crossbeam::channel::Sender;

use crate::{
    action::view_id,
    features::api_resources::message::ApiRequest,
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
        .id(view_id::popup_list)
        .widget_config(&WidgetConfig::builder().title("List").build())
        .on_select(move |w: &mut Window, _: &LiteralItem| {
            let widget = w
                .find_widget_mut(view_id::popup_list)
                .as_mut_multiple_select();

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
                w.widget_clear(view_id::tab_list_widget_list)
            }

            EventResult::Nop
        })
        .build()
        .into()
}
