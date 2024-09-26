use ratatui::crossterm::event::KeyCode;

use crate::{
    features::component_id::{YAML_KIND_POPUP_ID, YAML_NOT_FOUND_POPUP_ID},
    ui::{
        event::EventResult,
        widget::{base::WidgetBase, Text, Widget},
        Window,
    },
};

pub fn not_found_popup() -> Widget<'static> {
    Text::builder()
        .id(YAML_NOT_FOUND_POPUP_ID)
        .widget_base(&WidgetBase::builder().title("Name").build())
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
        w.open_popup(YAML_KIND_POPUP_ID);

        if let Widget::SingleSelect(w) = w.find_widget_mut(YAML_KIND_POPUP_ID) {
            w.clear_filter();
        }

        EventResult::Nop
    }
}
