use crossbeam::channel::Sender;

use crate::{
    cmd::POD_COLUMN_MAP,
    features::{component_id::POD_COLUMNS_DIALOG_ID, pod::message::PodColumnsRequest},
    message::Message,
    ui::{
        event::EventResult,
        widget::{
            multiple_select::SelectForm, Item, LiteralItem, MultipleSelect, Widget, WidgetBase,
            WidgetTrait,
        },
        Window,
    },
};

pub fn pod_columns_dialog(
    tx: Sender<Message>,
    default_columns: &[&'static str],
) -> Widget<'static> {
    let select_form = SelectForm::builder()
        .on_select_selected(on_select(tx.clone()))
        .on_select_unselected(on_select(tx))
        .build();

    let mut widget = MultipleSelect::builder()
        .id(POD_COLUMNS_DIALOG_ID)
        .widget_base(WidgetBase::builder().title("Pod Columns").build())
        .select_form(select_form)
        .build();

    widget.update_widget_item(Item::Array(
        default_columns
            .iter()
            .map(|&col| LiteralItem::new(col.to_uppercase(), None))
            .collect(),
    ));

    widget.select_all();

    widget.into()
}

fn on_select(tx: Sender<Message>) -> impl Fn(&mut Window, &LiteralItem) -> EventResult {
    move |w: &mut Window, v| {
        let widget = w
            .find_widget_mut(POD_COLUMNS_DIALOG_ID)
            .as_mut_multiple_select();

        widget.select_item(v);

        let items = widget
            .selected_items()
            .iter()
            .map(|i| i.item.to_lowercase())
            .collect::<Vec<_>>();

        let mut items = POD_COLUMN_MAP
            .iter()
            .filter_map(|(k, v)| {
                if items.contains(&k.to_string()) {
                    Some(*v)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        if !items.contains(&"Name") {
            items.insert(0, "Name");
            widget.select_item(&LiteralItem::new("NAME".to_string(), None));
        }

        tx.send(PodColumnsRequest::Set(items).into())
            .expect("Failed to send PodColumnsRequest::Set");

        EventResult::Nop
    }
}
