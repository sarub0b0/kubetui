use std::{cell::RefCell, rc::Rc};
use tui_wrapper::{tab::WidgetData, Tab};

use clipboard_wrapper::ClipboardContextWrapper;

use crate::action::view_id;

use tui_wrapper::widget::{config::WidgetConfig, Text, WidgetTrait};

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
        let events = self.events();

        EventsTab {
            tab: Tab::new(view_id::tab_event, self.title, [WidgetData::new(events)]),
        }
    }

    fn events(&self) -> Text<'static> {
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
