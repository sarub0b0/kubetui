use std::{cell::RefCell, rc::Rc};

use crate::clipboard_wrapper::ClipboardContextWrapper;

use crate::action::view_id;

use crate::tui_wrapper::{
    tab::WidgetData,
    widget::{config::WidgetConfig, Text, WidgetTrait},
    Tab,
};

pub struct EventsTabBuilder<'a> {
    title: &'a str,
    clipboard: &'a Option<Rc<RefCell<ClipboardContextWrapper>>>,
}

pub struct EventsTab {
    pub tab: Tab<'static>,
}

impl<'a> EventsTabBuilder<'a> {
    pub fn new(
        title: &'a str,
        clipboard: &'a Option<Rc<RefCell<ClipboardContextWrapper>>>,
    ) -> Self {
        Self { title, clipboard }
    }

    pub fn build(self) -> EventsTab {
        let event = self.event();

        EventsTab {
            tab: Tab::new(view_id::tab_event, self.title, [WidgetData::new(event)]),
        }
    }

    fn event(&self) -> Text<'static> {
        let builder = Text::builder()
            .id(view_id::tab_event_widget_event)
            .widget_config(&WidgetConfig::builder().title("Event").build())
            .wrap()
            .follow()
            .block_injection(|text: &Text, selected: bool| {
                let (index, _) = text.state().selected();

                let mut config = text.widget_config().clone();

                *config.append_title_mut() =
                    Some(format!(" [{}/{}]", index, text.rows_size()).into());

                config.render_block_with_title(text.focusable() && selected)
            });

        if let Some(cb) = self.clipboard {
            builder.clipboard(cb.clone())
        } else {
            builder
        }
        .build()
    }
}
