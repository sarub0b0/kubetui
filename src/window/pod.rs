use crossbeam::channel::Sender;

use std::{cell::RefCell, rc::Rc};

use crate::clipboard_wrapper::ClipboardContextWrapper;

use crate::event::{kubernetes::log::LogStreamMessage, Event};

use crate::action::view_id;

use crate::tui_wrapper::{
    event::EventResult,
    tab::WidgetData,
    widget::{config::WidgetConfig, Table, Text, WidgetTrait},
    Tab, WindowEvent,
};

use tui::layout::{Constraint, Direction, Layout};

pub struct PodTabBuilder<'a> {
    title: &'a str,
    tx: &'a Sender<Event>,
    clipboard: &'a Option<Rc<RefCell<ClipboardContextWrapper>>>,
    split_mode: Direction,
}

pub struct PodsTab {
    pub tab: Tab<'static>,
}

impl<'a> PodTabBuilder<'a> {
    pub fn new(
        title: &'static str,
        tx: &'a Sender<Event>,
        clipboard: &'a Option<Rc<RefCell<ClipboardContextWrapper>>>,
        split_mode: Direction,
    ) -> Self {
        PodTabBuilder {
            title,
            tx,
            clipboard,
            split_mode,
        }
    }

    pub fn build(self) -> PodsTab {
        let pod = self.pod();
        let log = self.log();

        PodsTab {
            tab: Tab::new(
                view_id::tab_pod,
                self.title,
                [
                    WidgetData::new(pod).chunk_index(0),
                    WidgetData::new(log).chunk_index(1),
                ],
            )
            .layout(
                Layout::default()
                    .direction(self.split_mode)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref()),
            ),
        }
    }

    fn pod(&self) -> Table<'static> {
        let tx = self.tx.clone();

        Table::builder()
            .id(view_id::tab_pod_widget_pod)
            .widget_config(&WidgetConfig::builder().title("Pod").build())
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
                w.widget_clear(view_id::tab_pod_widget_log);

                v.metadata.as_ref().map_or(EventResult::Ignore, |metadata| {
                    metadata
                        .get("namespace")
                        .as_ref()
                        .map_or(EventResult::Ignore, |namespace| {
                            metadata
                                .get("name")
                                .as_ref()
                                .map_or(EventResult::Ignore, |name| {
                                    *(w.find_widget_mut(view_id::tab_pod_widget_log)
                                        .widget_config_mut()
                                        .append_title_mut()) =
                                        Some((format!(" : {}", name)).into());

                                    tx.send(
                                        LogStreamMessage::Request {
                                            namespace: namespace.to_string(),
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
            .build()
    }

    fn log(&self) -> Text<'static> {
        let builder = Text::builder()
            .id(view_id::tab_pod_widget_log)
            .widget_config(&WidgetConfig::builder().title("Log").build())
            .wrap()
            .follow()
            .block_injection(|text: &Text, selected: bool| {
                let (index, _) = text.state().selected();

                let mut config = text.widget_config().clone();

                *config.title_mut() = format!("Log [{}/{}]", index, text.rows_size()).into();

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
