use std::str::FromStr as _;

use crossbeam::channel::Sender;
use strum::IntoEnumIterator;

use crate::{
    config::theme::WidgetThemeConfig,
    features::{
        component_id::POD_COLUMNS_DIALOG_ID,
        pod::{message::PodMessage, PodColumn, PodColumns},
    },
    message::Message,
    ui::{
        event::EventResult,
        widget::{CheckList, CheckListItem, Widget, WidgetBase},
        Window,
    },
};

pub fn pod_columns_dialog(
    tx: &Sender<Message>,
    default_columns: Option<PodColumns>,
    _theme: WidgetThemeConfig,
) -> Widget<'static> {
    let default_columns = default_columns.unwrap_or_default();

    let columns = PodColumn::iter()
        .map(|column| {
            let name = column.display().to_string();
            let checked = default_columns.contains(&column);
            let required = column == PodColumn::Name;

            CheckListItem {
                label: name,
                checked,
                required,
                metadata: None,
            }
        })
        .collect::<Vec<_>>();

    CheckList::builder()
        .id(POD_COLUMNS_DIALOG_ID)
        .widget_base(WidgetBase::builder().title("Pod Columns").build())
        .items(columns)
        .on_change(on_change(tx.clone()))
        .build()
        .into()
}

fn on_change(tx: Sender<Message>) -> impl Fn(&mut Window, &CheckListItem) -> EventResult {
    move |w: &mut Window, _v| {
        let widget = w.find_widget_mut(POD_COLUMNS_DIALOG_ID).as_mut_check_list();

        let items = widget
            .items()
            .iter()
            .filter(|item| item.required || item.checked)
            .filter_map(|i| PodColumn::from_str(&i.label).ok())
            .collect::<Vec<_>>();

        tx.send(PodMessage::Request(PodColumns::new(items)).into())
            .expect("Failed to send PodColumnsRequest::Set");

        EventResult::Nop
    }
}
