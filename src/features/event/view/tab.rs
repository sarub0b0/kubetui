use std::{cell::RefCell, rc::Rc};

use ratatui::prelude::Constraint;

use crate::{
    clipboard::Clipboard,
    features::component_id::EVENT_TAB_ID,
    ui::{
        tab::{LayoutElement, NestedLayoutElement, NestedWidgetLayout},
        Tab,
    },
};

use super::widget::event_widget;

pub struct EventTab {
    pub tab: Tab<'static>,
}

impl EventTab {
    pub fn new(title: &str, clipboard: &Option<Rc<RefCell<Clipboard>>>) -> Self {
        let event_widget = event_widget(clipboard);

        EventTab {
            tab: Tab::new(
                EVENT_TAB_ID,
                title,
                [event_widget],
                NestedWidgetLayout::default().nested_widget_layout([NestedLayoutElement(
                    Constraint::Percentage(100),
                    LayoutElement::WidgetIndex(0),
                )]),
            ),
        }
    }
}
