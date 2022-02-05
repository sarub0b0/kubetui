use crossbeam::channel::Sender;

use std::{cell::RefCell, rc::Rc};

use crate::clipboard_wrapper::ClipboardContextWrapper;

use crate::event::{kubernetes::*, Event};

use crate::action::view_id;
use crate::context::Namespace;

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
    namespaces: &'a Rc<RefCell<Namespace>>,
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
        namespaces: &'a Rc<RefCell<Namespace>>,
        clipboard: &'a Option<Rc<RefCell<ClipboardContextWrapper>>>,
        split_mode: Direction,
    ) -> Self {
        PodTabBuilder {
            title,
            tx,
            namespaces,
            clipboard,
            split_mode,
        }
    }

    pub fn build(self) -> PodsTab {
        let pods = self.pods();
        let logs = self.logs();

        PodsTab {
            tab: Tab::new(
                view_id::tab_pods,
                self.title,
                [
                    WidgetData::new(pods).chunk_index(0),
                    WidgetData::new(logs).chunk_index(1),
                ],
            )
            .layout(
                Layout::default()
                    .direction(self.split_mode)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref()),
            ),
        }
    }

    fn pods(&self) -> Table<'static> {
        let tx = self.tx.clone();
        let namespace = self.namespaces.clone();

        Table::builder()
            .id(view_id::tab_pods_widget_pods)
            .widget_config(&WidgetConfig::builder().title("Pods").build())
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
                w.widget_clear(view_id::tab_pods_widget_logs);

                let selected = &namespace.borrow().selected;

                let (ns, pod_name) = log_stream_request_param(v, selected);

                *(w.find_widget_mut(view_id::tab_pods_widget_logs)
                    .widget_config_mut()
                    .append_title_mut()) = Some((format!(" : {}", pod_name)).into());

                tx.send(Event::Kube(Kube::LogStreamRequest(ns, pod_name)))
                    .unwrap();

                EventResult::Window(WindowEvent::Continue)
            })
            .build()
    }

    fn logs(&self) -> Text<'static> {
        let builder = Text::builder()
            .id(view_id::tab_pods_widget_logs)
            .widget_config(&WidgetConfig::builder().title("Logs").build())
            .wrap()
            .follow()
            .block_injection(|text: &Text, selected: bool| {
                let (index, _) = text.state().selected();

                let mut config = text.widget_config().clone();

                *config.title_mut() = format!("Logs [{}/{}]", index, text.rows_size()).into();

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

fn log_stream_request_param(value: &[String], namespace: &[String]) -> (String, String) {
    if namespace.len() == 1 {
        (namespace[0].to_string(), value[0].to_string())
    } else {
        (value[0].to_string(), value[1].to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_stream_request_param_single_namespace() {
        let value = vec![
            "name".to_string(),
            "ready".to_string(),
            "status".to_string(),
            "age".to_string(),
        ];
        let namespace = vec!["ns".to_string()];

        let actual = log_stream_request_param(&value, &namespace);

        assert_eq!(("ns".to_string(), "name".to_string()), actual)
    }

    #[test]
    fn log_stream_request_param_multiple_namespaces() {
        let value = vec![
            "ns-1".to_string(),
            "name".to_string(),
            "ready".to_string(),
            "status".to_string(),
            "age".to_string(),
        ];
        let namespace = vec!["ns-0".to_string(), "ns-1".to_string()];

        let actual = log_stream_request_param(&value, &namespace);

        assert_eq!(("ns-1".to_string(), "name".to_string()), actual)
    }
}
