use std::{cell::RefCell, rc::Rc};

use crossbeam::channel::Sender;
use ratatui::layout::{Constraint, Direction};

use crate::{
    action::view_id,
    clipboard::Clipboard,
    features::network::view::widgets::{description_widget, network_widget},
    message::Message,
    ui::{
        tab::{LayoutElement, NestedLayoutElement, NestedWidgetLayout},
        Tab,
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
        split_mode: Direction,
    ) -> Self {
        let network_widget = network_widget(tx);
        let description_widget = description_widget(clipboard);

        let layout = layout(split_mode);

        NetworkTab {
            tab: Tab::new(
                view_id::tab_network,
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
