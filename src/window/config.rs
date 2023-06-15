use crossbeam::channel::Sender;
use std::{cell::RefCell, rc::Rc};

use crate::{
    action::view_id,
    clipboard_wrapper::Clipboard,
    event::{
        kubernetes::config::{ConfigRequest, RequestData},
        Event,
    },
    tui_wrapper::{
        event::EventResult,
        tab::WidgetData,
        widget::{config::WidgetConfig, Table, Text, WidgetTrait},
        Tab, WindowEvent,
    },
};
use ratatui::layout::{Constraint, Direction, Layout};

pub struct ConfigTabBuilder<'a> {
    title: &'static str,
    tx: &'a Sender<Event>,
    clipboard: &'a Option<Rc<RefCell<Clipboard>>>,
    split_mode: Direction,
}

pub struct ConfigTab {
    pub tab: Tab<'static>,
}

impl<'a> ConfigTabBuilder<'a> {
    pub fn new(
        title: &'static str,
        tx: &'a Sender<Event>,
        clipboard: &'a Option<Rc<RefCell<Clipboard>>>,
        split_mode: Direction,
    ) -> Self {
        ConfigTabBuilder {
            title,
            tx,
            clipboard,
            split_mode,
        }
    }

    pub fn build(self) -> ConfigTab {
        let config = self.config();
        let raw_data = self.raw_data();

        ConfigTab {
            tab: Tab::new(
                view_id::tab_config,
                self.title,
                [
                    WidgetData::new(config).chunk_index(0),
                    WidgetData::new(raw_data).chunk_index(1),
                ],
            )
            .layout(
                Layout::default()
                    .direction(self.split_mode.clone())
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref()),
            ),
        }
    }

    fn config(&self) -> Table<'static> {
        let tx = self.tx.clone();
        Table::builder()
            .id(view_id::tab_config_widget_config)
            .widget_config(&WidgetConfig::builder().title("Config").build())
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
                w.widget_clear(view_id::tab_config_widget_raw_data);

                let Some(metadata) = v.metadata.as_ref() else { return EventResult::Ignore };

                let Some(namespace) = metadata.get("namespace") else { return EventResult::Ignore };

                let Some(name) = metadata.get("name") else { return EventResult::Ignore };

                let Some(kind) = metadata.get("kind") else { return EventResult::Ignore };

                *(w.find_widget_mut(view_id::tab_config_widget_raw_data)
                    .widget_config_mut()
                    .append_title_mut()) = Some((format!(" : {}", name)).into());

                let request_data = RequestData {
                    namespace: namespace.to_string(),
                    name: name.to_string(),
                };

                match kind.as_str() {
                    "ConfigMap" => {
                        tx.send(ConfigRequest::ConfigMap(request_data).into())
                            .expect("Failed to ConfigRequest::ConfigMap");
                    }
                    "Secret" => {
                        tx.send(ConfigRequest::Secret(request_data).into())
                            .expect("Failed to send ConfigRequest::Secret");
                    }
                    _ => {}
                }

                EventResult::Window(WindowEvent::Continue)
            })
            .build()
    }

    fn raw_data(&self) -> Text {
        let builder = Text::builder()
            .id(view_id::tab_config_widget_raw_data)
            .widget_config(&WidgetConfig::builder().title("Raw Data").build())
            .wrap()
            .block_injection(|text: &Text, is_active: bool| {
                let (index, size) = text.state();

                let mut config = text.widget_config().clone();

                *config.title_mut() = format!("Raw Data [{}/{}]", index, size).into();

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
