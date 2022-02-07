use std::{cell::RefCell, rc::Rc};

use crate::action::view_id;
use crate::clipboard_wrapper::ClipboardContextWrapper;
use crate::context::Namespace;
use crate::event::Event;
use crate::tui_wrapper::{
    tab::WidgetData,
    tui::layout::{Constraint, Direction, Layout},
    widget::{config::WidgetConfig, List, Text},
    Tab,
};

use crossbeam::channel::Sender;

pub struct NetworkTab {
    pub tab: Tab<'static>,
}

pub struct NetworkTabBuilder<'a> {
    title: &'a str,
    tx: &'a Sender<Event>,
    namespaces: &'a Rc<RefCell<Namespace>>,
    clipboard: &'a Option<Rc<RefCell<ClipboardContextWrapper>>>,
    split_mode: Direction,
}

impl<'a> NetworkTabBuilder<'a> {
    pub fn new(
        title: &'static str,
        tx: &'a Sender<Event>,
        namespaces: &'a Rc<RefCell<Namespace>>,
        clipboard: &'a Option<Rc<RefCell<ClipboardContextWrapper>>>,
        split_mode: Direction,
    ) -> Self {
        Self {
            title,
            tx,
            namespaces,
            clipboard,
            split_mode,
        }
    }

    pub fn build(self) -> NetworkTab {
        NetworkTab {
            tab: Tab::new(
                view_id::tab_network,
                self.title,
                [
                    WidgetData::new(self.network()).chunk_index(0),
                    WidgetData::new(self.description()).chunk_index(1),
                ],
            )
            .layout(
                Layout::default()
                    .direction(self.split_mode)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)]),
            ),
        }
    }

    fn network(&self) -> List<'static> {
        List::builder()
            .id(view_id::tab_network_widget_network)
            .widget_config(&WidgetConfig::builder().title("Network").build())
            .build()
    }

    fn description(&self) -> Text<'static> {
        Text::builder()
            .id(view_id::tab_network_widget_description)
            .widget_config(&WidgetConfig::builder().title("Description").build())
            .build()
    }
}
