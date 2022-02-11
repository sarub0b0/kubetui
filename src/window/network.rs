use std::{cell::RefCell, rc::Rc};

use crate::action::view_id;
use crate::clipboard_wrapper::ClipboardContextWrapper;
use crate::context::Namespace;
use crate::event::kubernetes::network::{NetworkMessage, Request};
use crate::event::Event;
use crate::tui_wrapper::event::EventResult;
use crate::tui_wrapper::widget::{Table, WidgetTrait};
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

    fn network(&self) -> Table<'static> {
        let tx = self.tx.clone();
        Table::builder()
            .id(view_id::tab_network_widget_network)
            .widget_config(&WidgetConfig::builder().title("Network").build())
            .block_injection(|table: &Table, selected: bool| {
                let index = if let Some(index) = table.state().selected() {
                    index + 1
                } else {
                    0
                };

                let mut config = table.widget_config().clone();

                *config.append_title_mut() =
                    Some(format!(" [{}/{}]", index, table.items().len()).into());

                config.render_block_with_title(table.focusable() && selected)
            })
            .on_select(move |_, _| {
                tx.send(
                    NetworkMessage::Request(Request::Pod("name".into(), "namespace".into())).into(),
                )
                .unwrap();

                EventResult::Nop
            })
            .build()
    }

    fn description(&self) -> Text<'static> {
        Text::builder()
            .id(view_id::tab_network_widget_description)
            .widget_config(&WidgetConfig::builder().title("Description").build())
            .build()
    }
}
