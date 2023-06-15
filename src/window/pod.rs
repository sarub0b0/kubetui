use crossbeam::channel::Sender;
use crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Direction, Layout};
use std::{cell::RefCell, rc::Rc};

use crate::{
    action::view_id,
    clipboard_wrapper::Clipboard,
    event::{kubernetes::log::LogStreamMessage, Event, UserEvent},
    tui_wrapper::{
        event::EventResult,
        tab::WidgetChunk,
        widget::{config::WidgetConfig, Item, Table, Text, WidgetTrait},
        Tab, Window, WindowEvent,
    },
};

pub struct PodTabBuilder<'a> {
    title: &'a str,
    tx: &'a Sender<Event>,
    clipboard: &'a Option<Rc<RefCell<Clipboard>>>,
    split_mode: Direction,
}

pub struct PodsTab {
    pub tab: Tab<'static>,
}

impl<'a> PodTabBuilder<'a> {
    pub fn new(
        title: &'static str,
        tx: &'a Sender<Event>,
        clipboard: &'a Option<Rc<RefCell<Clipboard>>>,
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
                    WidgetChunk::new(pod).chunk_index(0),
                    WidgetChunk::new(log).chunk_index(1),
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
                w.widget_clear(view_id::tab_pod_widget_log);

                let Some(ref metadata) = v.metadata else {return EventResult::Ignore};
                let Some(ref namespace) = metadata.get("namespace") else {return EventResult::Ignore};

                let Some(ref name) = metadata.get("name") else {return EventResult::Ignore};

                *(w.find_widget_mut(view_id::tab_pod_widget_log).widget_config_mut().append_title_mut()) = Some((format!(" : {}", name)).into());

                tx.send(
                    LogStreamMessage::Request {
                        namespace: namespace.to_string(),
                        name: name.to_string(),
                    }
                    .into(),)
                    .expect("Failed to send LogStreamMessage::Request");

                EventResult::Window(WindowEvent::Continue)
            })
            .build()
    }

    fn log(&self) -> Text {
        let add_newline = move |w: &mut Window| {
            let w = w.find_widget_mut(view_id::tab_pod_widget_log);

            w.select_last();
            w.append_widget_item(Item::Single(Default::default()));

            EventResult::Nop
        };

        let builder = Text::builder()
            .id(view_id::tab_pod_widget_log)
            .widget_config(&WidgetConfig::builder().title("Log").build())
            .wrap()
            .follow()
            .block_injection(|text: &Text, is_active: bool| {
                let (index, size) = text.state();

                let mut config = text.widget_config().clone();

                *config.title_mut() = format!("Log [{}/{}]", index, size).into();

                config.render_block(text.can_activate() && is_active)
            })
            .action(UserEvent::from(KeyCode::Enter), add_newline);

        if let Some(cb) = self.clipboard {
            builder.clipboard(cb.clone())
        } else {
            builder
        }
        .build()
    }
}
