use std::{cell::RefCell, rc::Rc};

use crossbeam::channel::Sender;
use crossterm::event::KeyCode;
use indoc::indoc;
use ratatui::layout::{Constraint, Direction};

use crate::{
    action::view_id,
    clipboard_wrapper::Clipboard,
    context::Namespace,
    event::{Event, UserEvent},
    ui::{
        event::EventResult,
        tab::{LayoutElement, NestedLayoutElement, NestedWidgetLayout},
        widget::{
            config::WidgetConfig,
            input::{InputForm, InputFormBuilder},
            Item, SelectedItem, Table, Text, Widget, WidgetTrait,
        },
        Tab, Window, WindowEvent,
    },
    workers::kubernetes::pod::{LogConfig, LogMessage, LogPrefixType},
};

pub struct PodTabBuilder<'a> {
    title: &'a str,
    tx: &'a Sender<Event>,
    clipboard: &'a Option<Rc<RefCell<Clipboard>>>,
    split_mode: Direction,
    namespaces: Rc<RefCell<Namespace>>,
}

pub struct PodsTab {
    pub tab: Tab<'static>,
    pub popup_log_query_help: Widget<'static>,
}

impl<'a> PodTabBuilder<'a> {
    pub fn new(
        title: &'static str,
        tx: &'a Sender<Event>,
        clipboard: &'a Option<Rc<RefCell<Clipboard>>>,
        split_mode: Direction,
        namespaces: Rc<RefCell<Namespace>>,
    ) -> Self {
        PodTabBuilder {
            title,
            tx,
            clipboard,
            split_mode,
            namespaces,
        }
    }

    pub fn build(self) -> PodsTab {
        let pod = self.pod();
        let log_query = self.log_query();
        let log = self.log();
        let log_query_help = self.log_query_help().into();

        let pod_layout = {
            let constraint = match self.split_mode {
                Direction::Horizontal => Constraint::Percentage(50),
                Direction::Vertical => Constraint::Percentage(45), // log_query領域分小さくする
            };

            NestedLayoutElement(constraint, LayoutElement::WidgetIndex(0))
        };

        let log_layout = NestedLayoutElement(
            Constraint::Percentage(50),
            LayoutElement::NestedElement(
                NestedWidgetLayout::default()
                    .direction(Direction::Vertical)
                    .nested_widget_layout([
                        NestedLayoutElement(Constraint::Length(3), LayoutElement::WidgetIndex(1)),
                        NestedLayoutElement(Constraint::Min(3), LayoutElement::WidgetIndex(2)),
                    ]),
            ),
        );

        let layout = NestedWidgetLayout::default()
            .direction(self.split_mode)
            .nested_widget_layout([pod_layout, log_layout]);

        let mut tab = Tab::new(
            view_id::tab_pod,
            self.title,
            [pod.into(), log_query.into(), log.into()],
            layout,
        );

        tab.activate_widget_by_id(view_id::tab_pod_widget_pod);

        PodsTab {
            tab,
            popup_log_query_help: log_query_help,
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

                let Some(ref metadata) = v.metadata else {
                    return EventResult::Ignore;
                };

                let Some(ref namespace) = metadata.get("namespace") else {
                    return EventResult::Ignore;
                };

                let Some(ref name) = metadata.get("name") else {
                    return EventResult::Ignore;
                };

                let query_form = w.find_widget_mut(view_id::tab_pod_widget_log_query);

                query_form.update_widget_item(Item::Single(format!("pod/{}", name).into()));

                let namespaces = Namespace(vec![namespace.to_string()]);

                let config = LogConfig::new(
                    format!("pod/{}", name),
                    namespaces.to_owned(),
                    LogPrefixType::OnlyContainer,
                );

                tx.send(LogMessage::Request(config).into())
                    .expect("Failed to send LogMessage::Request");

                EventResult::Window(WindowEvent::Continue)
            })
            .build()
    }

    fn log_query(&self) -> InputForm {
        let tx = self.tx.clone();

        let namespaces = self.namespaces.clone();

        let execute = move |w: &mut Window| {
            let widget = w.find_widget_mut(view_id::tab_pod_widget_log_query);

            let Some(SelectedItem::Literal { metadata: _, item }) = widget.widget_item() else {
                return EventResult::Ignore;
            };

            if item == "?" || item == "help" {
                widget.clear();
                w.open_popup(view_id::tab_pod_widget_log_query_help);
                return EventResult::Nop;
            }

            w.widget_clear(view_id::tab_pod_widget_log);

            let namespaces = namespaces.borrow();

            let prefix_type = if 1 < namespaces.len() {
                LogPrefixType::All
            } else {
                LogPrefixType::PodAndContainer
            };

            let config = LogConfig::new(item, namespaces.to_owned(), prefix_type);

            tx.send(LogMessage::Request(config).into())
                .expect("Failed to send LogMessage::Request");

            EventResult::Ignore
        };

        InputFormBuilder::default()
            .id(view_id::tab_pod_widget_log_query)
            .widget_config(WidgetConfig::builder().title("Log Query").build())
            .actions(UserEvent::from(KeyCode::Enter), execute)
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
            .block_injection(|text: &Text, is_active: bool, is_mouse_over: bool| {
                let (index, size) = text.state();

                let mut config = text.widget_config().clone();

                *config.title_mut() = format!("Log [{}/{}]", index, size).into();

                config.render_block(text.can_activate() && is_active, is_mouse_over)
            })
            .action(UserEvent::from(KeyCode::Enter), add_newline);

        if let Some(cb) = self.clipboard {
            builder.clipboard(cb.clone())
        } else {
            builder
        }
        .build()
    }

    fn log_query_help(&self) -> Text {
        let content: Vec<String> = indoc! {r#"
            Usage: QUERY [ QUERY ]...

            Queries:
               pod:<regex>           (alias: pods, po, p)
               !pod:<regex>          (alias: !pods, !po, p)
               container:<regex>     (alias: containers, co, c)
               !container:<regex>    (alias: !containers, !co, !c)
               log:<regex>           (alias: logs, lo, l)
               !log:<regex>          (alias: !logs, !lo, !l)
               label:<selector>      (alias: labels)
               field:<selector>      (alias: fields)
               <resource>/<name>

            Resources:
               pod            (alias: pods, po)
               replicaset     (alias: replicasets, rs)
               deployment     (alias: deployments, deploy)
               statefulset    (alias: statefulsets, sts)
               daemonset      (alias: daemonsets, ds)
               service        (alias: services, svc)
               job            (alias: jobs)
        "# }
        .lines()
        .map(ToString::to_string)
        .collect();

        Text::builder()
            .id(view_id::tab_pod_widget_log_query_help)
            .widget_config(&WidgetConfig::builder().title("Log Query Help").build())
            .items(content)
            .action(UserEvent::from(KeyCode::Enter), |w: &mut Window| {
                w.close_popup();
                EventResult::Nop
            })
            .build()
    }
}
