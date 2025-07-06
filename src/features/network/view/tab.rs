use std::{cell::RefCell, rc::Rc};

use crossbeam::channel::Sender;
use ratatui::layout::{Constraint, Direction};

use crate::{
    clipboard::Clipboard,
    config::theme::WidgetThemeConfig,
    features::{
        component_id::NETWORK_TAB_ID,
        network::view::widgets::{description_widget, network_widget},
    },
    message::Message,
    ui::{
        Tab,
        tab::{LayoutElement, NestedLayoutElement, NestedWidgetLayout, TabLayout},
    },
};

pub struct NetworkTab {
    pub tab: Tab<'static>,
}

impl NetworkTab {
    pub fn new(
        title: &'static str,
        tx: &Sender<Message>,
        clipboard: &Option<Rc<RefCell<Clipboard>>>,
        split_direction: Direction,
        theme: WidgetThemeConfig,
    ) -> Self {
        let network_widget = network_widget(tx, theme.clone());
        let description_widget = description_widget(clipboard, theme);

        let layout = TabLayout::new(layout, split_direction);

        NetworkTab {
            tab: Tab::new(
                NETWORK_TAB_ID,
                title,
                [network_widget, description_widget],
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
