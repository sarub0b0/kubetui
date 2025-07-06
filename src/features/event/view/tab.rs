use std::{cell::RefCell, rc::Rc};

use ratatui::prelude::Constraint;

use crate::{
    clipboard::Clipboard,
    config::theme::WidgetThemeConfig,
    features::component_id::EVENT_TAB_ID,
    ui::{
        Tab,
        tab::{LayoutElement, NestedLayoutElement, NestedWidgetLayout, TabLayout},
    },
};

use super::widget::event_widget;

pub struct EventTab {
    pub tab: Tab<'static>,
}

impl EventTab {
    pub fn new(
        title: &str,
        clipboard: &Option<Rc<RefCell<Clipboard>>>,
        theme: WidgetThemeConfig,
    ) -> Self {
        let event_widget = event_widget(clipboard, theme);

        let layout = TabLayout::new(
            |_| {
                NestedWidgetLayout::default().nested_widget_layout([NestedLayoutElement(
                    Constraint::Percentage(100),
                    LayoutElement::WidgetIndex(0),
                )])
            },
            Default::default(),
        );

        EventTab {
            tab: Tab::new(EVENT_TAB_ID, title, [event_widget], layout),
        }
    }
}
