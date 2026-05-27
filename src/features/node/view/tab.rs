use ratatui::layout::{Constraint, Direction};

use crate::{
    config::theme::WidgetThemeConfig,
    features::component_id::NODE_TAB_ID,
    ui::{
        tab::{LayoutElement, NestedLayoutElement, NestedWidgetLayout, TabLayout},
        Tab,
    },
};

use super::widgets::node_widget;

pub struct NodeTab {
    pub tab: Tab<'static>,
}

impl NodeTab {
    pub fn new(title: &'static str, theme: WidgetThemeConfig) -> Self {
        let node_widget = node_widget(theme);

        let tab = Tab::new(
            NODE_TAB_ID,
            title,
            [node_widget],
            TabLayout::new(layout, Direction::Vertical),
        );

        Self { tab }
    }
}

fn layout(_split_direction: Direction) -> NestedWidgetLayout {
    NestedWidgetLayout::default().nested_widget_layout([NestedLayoutElement(
        Constraint::Percentage(100),
        LayoutElement::WidgetIndex(0),
    )])
}
