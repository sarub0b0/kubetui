use std::{cell::RefCell, rc::Rc};

use crossbeam::channel::Sender;
use ratatui::layout::{Constraint, Direction};

use crate::{
    clipboard::Clipboard,
    config::theme::WidgetThemeConfig,
    features::component_id::CONFIG_TAB_ID,
    message::Message,
    ui::{
        tab::{LayoutElement, NestedLayoutElement, NestedWidgetLayout, TabLayout},
        Tab,
    },
};

use super::widgets::{config_widget, raw_data_widget};

pub struct ConfigTab {
    pub tab: Tab<'static>,
}

impl ConfigTab {
    pub fn new(
        title: &'static str,
        tx: &Sender<Message>,
        clipboard: &Option<Rc<RefCell<Clipboard>>>,
        split_direction: Direction,
        theme: WidgetThemeConfig,
    ) -> Self {
        let config_widget = config_widget(tx, theme.clone());
        let raw_data_widget = raw_data_widget(clipboard, theme);

        let layout = TabLayout::new(layout, split_direction);

        Self {
            tab: Tab::new(
                CONFIG_TAB_ID,
                title,
                [config_widget, raw_data_widget],
                layout,
            ),
        }
    }
}

fn layout(split_direction: Direction) -> NestedWidgetLayout {
    NestedWidgetLayout::default()
        .direction(split_direction)
        .nested_widget_layout([
            NestedLayoutElement(Constraint::Percentage(50), LayoutElement::WidgetIndex(0)),
            NestedLayoutElement(Constraint::Percentage(50), LayoutElement::WidgetIndex(1)),
        ])
}
