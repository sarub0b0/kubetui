use crossbeam::channel::Sender;
use ratatui::layout::{Constraint, Direction};

use crate::{
    config::theme::WidgetThemeConfig,
    features::{component_id::NODE_TAB_ID, node::NodeColumns},
    message::Message,
    ui::{
        tab::{LayoutElement, NestedLayoutElement, NestedWidgetLayout, TabLayout},
        widget::Widget,
        Tab,
    },
};

use super::widgets::{node_columns_dialog, node_widget};

pub struct NodeTab {
    pub tab: Tab<'static>,
    pub node_columns_dialog: Widget<'static>,
}

impl NodeTab {
    pub fn new(
        title: &'static str,
        tx: &Sender<Message>,
        default_columns: Option<NodeColumns>,
        theme: WidgetThemeConfig,
    ) -> Self {
        let node_widget = node_widget(theme.clone());
        let node_columns_dialog = node_columns_dialog(tx, default_columns, theme);

        let tab = Tab::new(
            NODE_TAB_ID,
            title,
            [node_widget],
            TabLayout::new(layout, Direction::Vertical),
        );

        Self {
            tab,
            node_columns_dialog,
        }
    }
}

fn layout(_split_direction: Direction) -> NestedWidgetLayout {
    NestedWidgetLayout::default().nested_widget_layout([NestedLayoutElement(
        Constraint::Percentage(100),
        LayoutElement::WidgetIndex(0),
    )])
}
