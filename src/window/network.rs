use crossbeam::channel::Sender;
use std::{cell::RefCell, rc::Rc};

use crate::{
    action::view_id,
    clipboard_wrapper::Clipboard,
    event::{
        kubernetes::network::{NetworkRequest, RequestData},
        Event,
    },
    ui::{
        event::EventResult,
        tab::WidgetChunk,
        widget::{config::WidgetConfig, Table, Text, WidgetTrait},
        Tab, WindowEvent,
    },
};

use ratatui::layout::{Constraint, Direction, Layout};

pub struct NetworkTab {
    pub tab: Tab<'static>,
}

pub struct NetworkTabBuilder<'a> {
    title: &'a str,
    tx: &'a Sender<Event>,
    clipboard: &'a Option<Rc<RefCell<Clipboard>>>,
    split_mode: Direction,
}

impl<'a> NetworkTabBuilder<'a> {
    pub fn new(
        title: &'static str,
        tx: &'a Sender<Event>,
        clipboard: &'a Option<Rc<RefCell<Clipboard>>>,
        split_mode: Direction,
    ) -> Self {
        Self {
            title,
            tx,
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
                    WidgetChunk::new(self.network()).chunk_index(0),
                    WidgetChunk::new(self.description()).chunk_index(1),
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
            .filtered_key("NAME")
            .block_injection(|table: &Table| {
                let index = if let Some(index) = table.state().selected() {
                    index + 1
                } else {
                    0
                };

                let mut widget_config = table.widget_config().clone();

                *widget_config.append_title_mut() =
                    Some(format!(" [{}/{}]", index, table.items().len()).into());

                widget_config
            })
            .on_select(move |w, v| {
                w.widget_clear(view_id::tab_network_widget_description);

                let Some(metadata) = v.metadata.as_ref() else { return EventResult::Ignore };

                let Some(namespace) = metadata.get("namespace") else { return EventResult::Ignore };

                let Some(name) = metadata.get("name") else { return EventResult::Ignore };

                let Some(kind) = metadata.get("kind") else { return EventResult::Ignore };

                *(w.find_widget_mut(view_id::tab_network_widget_description)
                    .widget_config_mut()
                    .append_title_mut()) = Some((format!(" : {}", name)).into());

                let request_data = RequestData {
                    namespace: namespace.to_string(),
                    name: name.to_string(),
                };

                match kind.as_str() {
                    "Pod" => {
                        tx.send(NetworkRequest::Pod(request_data).into())
                            .expect("Failed to send NetworkRequest::Pod");
                    }
                    "Service" => {
                        tx.send(NetworkRequest::Service(request_data).into())
                            .expect("Failed to send NetworkRequest::Service");
                    }
                    "Ingress" => {
                        tx.send(NetworkRequest::Ingress(request_data).into())
                            .expect("Failed to send NetworkRequest::Ingress");
                    }
                    "NetworkPolicy" => {
                        tx.send(NetworkRequest::NetworkPolicy(request_data).into())
                            .expect("Failed to send NetworkRequest::NetworkPolicy");
                    }
                    _ => {}
                }

                EventResult::Window(WindowEvent::Continue)
            })
            .build()
    }

    fn description(&self) -> Text {
        let builder = Text::builder()
            .id(view_id::tab_network_widget_description)
            .widget_config(&WidgetConfig::builder().title("Description").build())
            .block_injection(|text: &Text, is_active: bool| {
                let (index, size) = text.state();

                let mut config = text.widget_config().clone();

                *config.title_mut() = format!("Description [{}/{}]", index, size).into();

                config.render_block(text.can_activate() && is_active)
            });

        if let Some(cb) = self.clipboard {
            builder.clipboard(cb.clone())
        } else {
            builder
        }
        .build()
    }
}
