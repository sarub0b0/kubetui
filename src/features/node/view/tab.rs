use std::{cell::RefCell, rc::Rc};

use crossbeam::channel::Sender;
use ratatui::layout::{Constraint, Direction};

use crate::{
    clipboard::Clipboard,
    config::theme::WidgetThemeConfig,
    features::{
        component_id::NODE_TAB_ID,
        node::{NodeColumns, NodeLabelColumn},
    },
    message::Message,
    ui::{
        tab::{LayoutElement, NestedLayoutElement, NestedWidgetLayout, TabLayout},
        widget::Widget,
        Tab,
    },
};

use super::widgets::{node_columns_dialog, node_detail_widget, node_widget};

pub struct NodeTab {
    pub tab: Tab<'static>,
    pub node_columns_dialog: Widget<'static>,
}

impl NodeTab {
    pub fn new(
        title: &'static str,
        tx: &Sender<Message>,
        clipboard: &Option<Rc<RefCell<Clipboard>>>,
        split_direction: Direction,
        default_columns: Option<NodeColumns>,
        label_registry: Vec<NodeLabelColumn>,
        theme: WidgetThemeConfig,
    ) -> Self {
        let node_widget = node_widget(tx.clone(), theme.clone());
        let detail_widget = node_detail_widget(clipboard, theme.clone());
        let node_columns_dialog = node_columns_dialog(tx, default_columns, label_registry, theme);

        let tab = Tab::new(
            NODE_TAB_ID,
            title,
            [node_widget, detail_widget],
            TabLayout::new(layout, split_direction),
        );

        Self {
            tab,
            node_columns_dialog,
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
