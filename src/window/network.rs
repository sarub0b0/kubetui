use std::{cell::RefCell, rc::Rc};

use crate::action::view_id;
use crate::clipboard_wrapper::ClipboardContextWrapper;
use crate::context::Namespace;
use crate::event::kubernetes::network::{NetworkMessage, Request};
use crate::event::Event;
use crate::tui_wrapper::event::EventResult;
use crate::tui_wrapper::widget::{Table, WidgetTrait};
use crate::tui_wrapper::WindowEvent;
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
        let namespaces = self.namespaces.clone();
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
            .on_select(move |w, v| {
                w.widget_clear(view_id::tab_network_widget_description);

                let namespaces = namespaces.borrow();
                let parsed = parse(&v, &namespaces.selected);

                if let Ok(p) = parsed {
                    *(w.find_widget_mut(view_id::tab_network_widget_description)
                        .widget_config_mut()
                        .append_title_mut()) = Some((format!(" : {}", p.name)).into());

                    match p.kind {
                        "Pod" => {
                            let req = Request::Pod(p.name.to_string(), p.namespace.to_string());
                            tx.send(NetworkMessage::Request(req).into()).unwrap();
                        }
                        "Service" => {
                            let req = Request::Service(p.name.to_string(), p.namespace.to_string());
                            tx.send(NetworkMessage::Request(req).into()).unwrap();
                        }
                        "Ingress" => {
                            let req = Request::Ingress(p.name.to_string(), p.namespace.to_string());
                            tx.send(NetworkMessage::Request(req).into()).unwrap();
                        }
                        _ => {}
                    }
                }

                EventResult::Window(WindowEvent::Continue)
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

struct Param<'a> {
    name: &'a str,
    kind: &'a str,
    namespace: &'a str,
}

fn parse<'a>(row: &'a [String], namespace: &'a [String]) -> Result<Param<'a>, String> {
    if namespace.len() == 1 {
        if 2 <= row.len() {
            Ok(Param {
                name: &row[1],
                kind: &row[0],
                namespace: &namespace[0],
            })
        } else {
            Err("invalid row".into())
        }
    } else if 3 <= row.len() {
        Ok(Param {
            name: &row[2],
            kind: &row[1],
            namespace: &row[0],
        })
    } else {
        Err("invalid row".into())
    }
}
