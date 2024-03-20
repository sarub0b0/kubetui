use std::{cell::RefCell, rc::Rc};

use crossbeam::channel::Sender;
use ratatui::layout::{Constraint, Direction};

use crate::{
    clipboard::Clipboard,
    features::component_id::{POD_TAB_ID, POD_WIDGET_ID},
    kube::context::Namespace,
    message::Message,
    ui::{
        tab::{LayoutElement, NestedLayoutElement, NestedWidgetLayout},
        widget::Widget,
        Tab,
    },
};

use super::widgets::{log_query_help_widget, log_query_widget, log_widget, pod_widget};

pub struct PodTab {
    pub tab: Tab<'static>,
    pub log_query_help_popup: Widget<'static>,
}

impl PodTab {
    pub fn new(
        title: &'static str,
        tx: &Sender<Message>,
        clipboard: &Option<Rc<RefCell<Clipboard>>>,
        split_direction: Direction,
        namespaces: Rc<RefCell<Namespace>>,
    ) -> Self {
        let pod_widget = pod_widget(tx);
        let log_query_widget = log_query_widget(tx, namespaces);
        let log_widget = log_widget(clipboard);
        let log_query_help_widget = log_query_help_widget();

        let layout = layout(split_direction);

        let mut tab = Tab::new(
            POD_TAB_ID,
            title,
            [pod_widget, log_query_widget, log_widget],
            layout,
        );

        tab.activate_widget_by_id(POD_WIDGET_ID);

        Self {
            tab,
            log_query_help_popup: log_query_help_widget,
        }
    }
}

fn layout(split_direction: Direction) -> NestedWidgetLayout {
    let (pod_constraint, log_constraint) = match split_direction {
        Direction::Horizontal => (Constraint::Percentage(50), Constraint::Percentage(50)),
        Direction::Vertical => (Constraint::Percentage(45), Constraint::Percentage(55)),
    };

    let pod_layout = NestedLayoutElement(pod_constraint, LayoutElement::WidgetIndex(0));

    let log_query_layout =
        NestedLayoutElement(Constraint::Length(3), LayoutElement::WidgetIndex(1));

    let log_layout = NestedLayoutElement(
        log_constraint,
        LayoutElement::NestedElement(
            NestedWidgetLayout::default()
                .direction(Direction::Vertical)
                .nested_widget_layout([
                    log_query_layout,
                    NestedLayoutElement(Constraint::Min(3), LayoutElement::WidgetIndex(2)),
                ]),
        ),
    );

    NestedWidgetLayout::default()
        .direction(split_direction)
        .nested_widget_layout([pod_layout, log_layout])
}
