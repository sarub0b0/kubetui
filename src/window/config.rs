use crossbeam::channel::Sender;
use std::{cell::RefCell, rc::Rc};

use crate::{
    action::view_id,
    clipboard_wrapper::ClipboardContextWrapper,
    event::{kubernetes::config::ConfigMessage, Event},
    tui_wrapper::{
        event::EventResult,
        tab::WidgetData,
        tui::layout::{Constraint, Direction, Layout},
        widget::{config::WidgetConfig, Table, Text, WidgetTrait},
        Tab, WindowEvent,
    },
};

pub struct ConfigTabBuilder<'a> {
    title: &'static str,
    tx: &'a Sender<Event>,
    clipboard: &'a Option<Rc<RefCell<ClipboardContextWrapper>>>,
    split_mode: Direction,
}

pub struct ConfigTab {
    pub tab: Tab<'static>,
}

impl<'a> ConfigTabBuilder<'a> {
    pub fn new(
        title: &'static str,
        tx: &'a Sender<Event>,
        clipboard: &'a Option<Rc<RefCell<ClipboardContextWrapper>>>,
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
                w.widget_clear(view_id::tab_config_widget_raw_data);

                v.metadata.as_ref().map_or(EventResult::Ignore, |metadata| {
                    metadata
                        .get("namespace")
                        .map_or(EventResult::Ignore, |namespace| {
                            metadata.get("name").map_or(EventResult::Ignore, |name| {
                                metadata.get("kind").map_or(EventResult::Ignore, |kind| {
                                    *(w.find_widget_mut(view_id::tab_config_widget_raw_data)
                                        .widget_config_mut()
                                        .append_title_mut()) =
                                        Some((format!(" : {}", name)).into());

                                    tx.send(
                                        ConfigMessage::DataRequest {
                                            namespace: namespace.to_string(),
                                            kind: kind.to_string(),
                                            name: name.to_string(),
                                        }
                                        .into(),
                                    )
                                    .unwrap();

                                    EventResult::Window(WindowEvent::Continue)
                                })
                            })
                        })
                })
            })
            .build()
    }

    fn raw_data(&self) -> Text<'static> {
        let builder = Text::builder()
            .id(view_id::tab_config_widget_raw_data)
            .widget_config(&WidgetConfig::builder().title("Raw Data").build())
            .wrap()
            .block_injection(|text: &Text, selected: bool| {
                let (index, _) = text.state().selected();

                let mut config = text.widget_config().clone();

                *config.title_mut() = format!("Raw Data [{}/{}]", index, text.rows_size()).into();

                config.render_block_with_title(text.focusable() && selected)
            });

        if let Some(cb) = self.clipboard {
            builder.clipboard(cb.clone())
        } else {
            builder
        }
        .build()
    }
}
