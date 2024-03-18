use crossterm::event::KeyCode;

use crate::{
    action::view_id,
    ui::{
        event::EventResult,
        widget::{config::WidgetConfig, Text, Widget},
        Window,
    },
};

pub fn not_found_popup() -> Widget<'static> {
    Text::builder()
        .id(view_id::popup_yaml_return)
        .widget_config(&WidgetConfig::builder().title("Name").build())
        .items(
            [
                "No resources found.",
                "",
                "Press \x1b[1mEnter\x1b[0m or \x1b[1mEsc\x1b[0m to return to resource selection.",
            ]
            .into_iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        )
        .wrap()
        .action(KeyCode::Enter, open_kind_popup())
        .action(KeyCode::Esc, open_kind_popup())
        .build()
        .into()
}

fn open_kind_popup() -> impl Fn(&mut Window) -> EventResult {
    move |w: &mut Window| {
        w.open_popup(view_id::popup_yaml_kind);

        if let Widget::SingleSelect(w) = w.find_widget_mut(view_id::popup_yaml_kind) {
            w.clear_filter();
        }

        EventResult::Nop
    }
}
