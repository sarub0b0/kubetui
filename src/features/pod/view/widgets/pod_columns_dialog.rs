use crossbeam::channel::Sender;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{
    cmd::POD_COLUMN_MAP,
    features::{component_id::POD_COLUMNS_DIALOG_ID, pod::message::PodColumnsRequest},
    logger,
    message::{Message, UserEvent},
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
        .on_select_unselected(on_select(tx.clone()))
        .build();

    let mut widget = MultipleSelect::builder()
        .id(POD_COLUMNS_DIALOG_ID)
        .widget_base(WidgetBase::builder().title("Pod Columns").build())
        .select_form(select_form)
        .action(
            UserEvent::from(KeyEvent::new(KeyCode::Up, KeyModifiers::SHIFT)),
            on_move_up(tx.clone()),
        )
        .action(
            UserEvent::from(KeyEvent::new(KeyCode::Down, KeyModifiers::SHIFT)),
            on_move_down(tx.clone()),
        )
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

        let mut items: Vec<_> = items
            .iter()
            .filter_map(|item| {
                POD_COLUMN_MAP.iter().find_map(|(k, v)| {
                    if item == &k.to_string() {
                        Some(*v)
                    } else {
                        None
                    }
                })
            })
            .collect();

        if !items.contains(&"Name") {
            items.insert(0, "Name");
            widget.select_item(&LiteralItem::new("NAME".to_string(), None));
        }

        tx.send(PodColumnsRequest::Set(items).into())
            .expect("Failed to send PodColumnsRequest::Set");

        EventResult::Nop
    }
}

fn on_move_up(tx: Sender<Message>) -> impl Fn(&mut Window) -> EventResult {
    move |w: &mut Window| {
        let widget = w
            .find_widget_mut(POD_COLUMNS_DIALOG_ID)
            .as_mut_multiple_select();

        widget.select_form_mut().selected_widget_mut().move_up();

        let items = widget
            .select_form()
            .selected_widget()
            .items()
            .iter()
            .map(|i| i.item.to_lowercase())
            .collect::<Vec<_>>();

        logger!(error, "Items before move up: {:?}", items);

        let mut items: Vec<_> = items
            .iter()
            .filter_map(|item| {
                POD_COLUMN_MAP.iter().find_map(|(k, v)| {
                    if item == &k.to_string() {
                        Some(*v)
                    } else {
                        None
                    }
                })
            })
            .collect();

        logger!(error, "Items after move up: {:?}", items);

        if !items.contains(&"Name") {
            items.insert(0, "Name");
            widget.select_item(&LiteralItem::new("NAME".to_string(), None));
        }

        tx.send(PodColumnsRequest::Set(items).into())
            .expect("Failed to send PodColumnsRequest::Set");

        EventResult::Nop
    }
}

fn on_move_down(tx: Sender<Message>) -> impl Fn(&mut Window) -> EventResult {
    move |w: &mut Window| {
        let widget = w
            .find_widget_mut(POD_COLUMNS_DIALOG_ID)
            .as_mut_multiple_select();

        widget.select_form_mut().selected_widget_mut().move_down();

        let items = widget
            .select_form()
            .selected_widget()
            .items()
            .iter()
            .map(|i| i.item.to_lowercase())
            .collect::<Vec<_>>();

        logger!(error, "Items before move down: {:?}", items);

        let mut items: Vec<_> = items
            .iter()
            .filter_map(|item| {
                POD_COLUMN_MAP.iter().find_map(|(k, v)| {
                    if item == &k.to_string() {
                        Some(*v)
                    } else {
                        None
                    }
                })
            })
            .collect();

        logger!(error, "Items after move down: {:?}", items);

        if !items.contains(&"Name") {
            items.insert(0, "Name");
            widget.select_item(&LiteralItem::new("NAME".to_string(), None));
        }

        tx.send(PodColumnsRequest::Set(items).into())
            .expect("Failed to send PodColumnsRequest::Set");

        EventResult::Nop
    }
}
