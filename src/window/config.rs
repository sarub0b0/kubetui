use crossbeam::channel::Sender;
use std::{cell::RefCell, rc::Rc};

use crate::clipboard_wrapper::ClipboardContextWrapper;

use crate::event::{kubernetes::*, Event};

use crate::action::view_id;
use crate::context::Namespace;

use crate::tui_wrapper::{
    event::EventResult,
    tab::WidgetData,
    tui::layout::{Constraint, Direction, Layout},
    widget::{config::WidgetConfig, Table, Text, WidgetTrait},
    Tab, WindowEvent,
};

pub struct ConfigsTabBuilder<'a> {
    title: &'static str,
    tx: &'a Sender<Event>,
    namespaces: &'a Rc<RefCell<Namespace>>,
    clipboard: &'a Option<Rc<RefCell<ClipboardContextWrapper>>>,
    split_mode: Direction,
}

pub struct ConfigsTab {
    pub tab: Tab<'static>,
}

impl<'a> ConfigsTabBuilder<'a> {
    pub fn new(
        title: &'static str,
        tx: &'a Sender<Event>,
        namespaces: &'a Rc<RefCell<Namespace>>,
        clipboard: &'a Option<Rc<RefCell<ClipboardContextWrapper>>>,
        split_mode: Direction,
    ) -> Self {
        ConfigsTabBuilder {
            title,
            tx,
            namespaces,
            clipboard,
            split_mode,
        }
    }

    pub fn build(self) -> ConfigsTab {
        let configs = self.configs();
        let raw_data = self.raw_data();

        ConfigsTab {
            tab: Tab::new(
                view_id::tab_configs,
                self.title,
                [
                    WidgetData::new(configs).chunk_index(0),
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

    fn configs(&self) -> Table<'static> {
        let tx = self.tx.clone();
        let namespaces = self.namespaces.clone();
        Table::builder()
            .id(view_id::tab_configs_widget_configs)
            .widget_config(&WidgetConfig::builder().title("Configs").build())
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
                w.widget_clear(view_id::tab_configs_widget_raw_data);

                let (ns, kind, name) = config_request_param(v, &namespaces.borrow().selected);

                *(w.find_widget_mut(view_id::tab_configs_widget_raw_data)
                    .widget_config_mut()
                    .append_title_mut()) = Some((format!(" : {}", name)).into());

                tx.send(Event::Kube(Kube::ConfigRequest(ns, kind, name)))
                    .unwrap();

                EventResult::Window(WindowEvent::Continue)
            })
            .build()
    }

    fn raw_data(&self) -> Text<'static> {
        let builder = Text::builder()
            .id(view_id::tab_configs_widget_raw_data)
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

fn config_request_param(value: &[String], namespace: &[String]) -> (String, String, String) {
    if namespace.len() == 1 {
        if 2 <= value.len() {
            (
                namespace[0].to_string(),
                value[0].to_string(),
                value[1].to_string(),
            )
        } else {
            (
                "Error".to_string(),
                "Error".to_string(),
                "Error".to_string(),
            )
        }
    } else if 3 <= value.len() {
        (
            value[0].to_string(),
            value[1].to_string(),
            value[2].to_string(),
        )
    } else {
        (
            "Error".to_string(),
            "Error".to_string(),
            "Error".to_string(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_request_param_single_namespace() {
        let value = vec![
            "kind".to_string(),
            "name".to_string(),
            "data".to_string(),
            "age".to_string(),
        ];

        let namespace = vec!["ns".to_string()];

        let actual = config_request_param(&value, &namespace);

        assert_eq!(
            ("ns".to_string(), "kind".to_string(), "name".to_string()),
            actual
        )
    }

    #[test]
    fn config_request_param_multiple_namespaces() {
        let value = vec![
            "ns-1".to_string(),
            "kind".to_string(),
            "name".to_string(),
            "data".to_string(),
            "age".to_string(),
        ];
        let namespace = vec!["ns-0".to_string(), "ns-1".to_string()];
        let actual = config_request_param(&value, &namespace);

        assert_eq!(
            ("ns-1".to_string(), "kind".to_string(), "name".to_string()),
            actual
        )
    }
}
