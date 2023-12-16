use crossbeam::channel::Sender;
use crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Direction};
use std::{cell::RefCell, rc::Rc};

use crate::{
    action::view_id,
    clipboard_wrapper::Clipboard,
    context::Namespace,
    event::{
        kubernetes::log::{LogStreamConfig, LogStreamMessage, LogStreamPrefixType},
        Event, UserEvent,
    },
    ui::{
        event::EventResult,
        tab::{LayoutElement, NestedLayoutElement, NestedWidgetLayout},
        widget::{
            config::WidgetConfig,
            input::{InputForm, InputFormBuilder},
            Item, SelectedItem, Table, Text, WidgetTrait,
        },
        Tab, Window, WindowEvent,
    },
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
        let input = self.input();
        let pod = self.pod();
        let log = self.log();

        let layout = NestedWidgetLayout::default()
            .direction(Direction::Vertical)
            .nested_widget_layout([
                NestedLayoutElement(Constraint::Length(3), LayoutElement::WidgetIndex(0)),
                NestedLayoutElement(
                    Constraint::Min(3),
                    LayoutElement::NestedElement(
                        NestedWidgetLayout::default()
                            .direction(self.split_mode)
                            .nested_widget_layout([
                                NestedLayoutElement(
                                    Constraint::Percentage(50),
                                    LayoutElement::WidgetIndex(1),
                                ),
                                NestedLayoutElement(
                                    Constraint::Percentage(50),
                                    LayoutElement::WidgetIndex(2),
                                ),
                            ]),
                    ),
                ),
            ]);

        let mut tab = Tab::new(
            view_id::tab_pod,
            self.title,
            [input.into(), pod.into(), log.into()],
            layout,
        );

        tab.activate_widget_by_id(view_id::tab_pod_widget_pod);

        PodsTab { tab }
    }

    fn input(&self) -> InputForm {
        let tx = self.tx.clone();

        let namespaces = self.namespaces.clone();

        let execute = move |w: &mut Window| {
            w.widget_clear(view_id::tab_pod_widget_log);

            let widget = w.find_widget_mut(view_id::tab_pod_widget_query);

            let Some(SelectedItem::Literal { metadata: _, item }) = widget.widget_item() else {
                return EventResult::Ignore;
            };

            let namespaces = namespaces.borrow();

            let prefix_type = if 1 < namespaces.len() {
                LogStreamPrefixType::All
            } else {
                LogStreamPrefixType::PodAndContainer
            };

            let config = LogStreamConfig::new(item, namespaces.to_owned(), prefix_type);

            tx.send(LogStreamMessage::Request(config).into())
                .expect("Failed to send LogStreamMessage::Request");

            EventResult::Ignore
        };

        InputFormBuilder::default()
            .id(view_id::tab_pod_widget_query)
            .widget_config(WidgetConfig::builder().title("Query").build())
            .actions(UserEvent::from(KeyCode::Enter), execute)
            .build()
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

                *(w.find_widget_mut(view_id::tab_pod_widget_log)
                    .widget_config_mut()
                    .append_title_mut()) = Some((format!(" : {}", name)).into());

                let namespaces = Namespace(vec![namespace.to_string()]);

                let config = LogStreamConfig::new(
                    format!("regex:^{}$", name),
                    namespaces.to_owned(),
                    LogStreamPrefixType::OnlyContainer,
                );

                tx.send(LogStreamMessage::Request(config).into())
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
}
