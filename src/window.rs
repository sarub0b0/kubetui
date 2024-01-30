mod context;
mod event;
mod help;
mod list;
mod network;
mod yaml;
mod yaml_popup;

use std::{cell::RefCell, rc::Rc};

use crossbeam::channel::Sender;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{layout::Direction, text::Line, widgets::Paragraph};

use crate::{
    action::view_id,
    clipboard::Clipboard,
    context::{Context, Namespace},
    features::{config::view::ConfigTab, pod::view::PodTab},
    message::{Message, UserEvent},
    ui::{
        event::EventResult,
        popup::Popup,
        widget::{SelectedItem, WidgetTrait},
        Header, Tab, Window, WindowEvent,
    },
    workers::kube::{
        context_message::ContextRequest,
        namespace_message::NamespaceRequest,
        yaml::{
            direct::{DirectedYaml, DirectedYamlKind},
            YamlRequest,
        },
    },
};

use self::{
    context::{ContextPopup, ContextPopupBuilder},
    event::{EventsTab, EventsTabBuilder},
    help::HelpPopup,
    list::{ListTab, ListTabBuilder},
    network::{NetworkTab, NetworkTabBuilder},
    yaml::{YamlTab, YamlTabBuilder},
    yaml_popup::{YamlPopup, YamlPopupBuilder},
};

pub struct WindowInit {
    split_mode: Direction,
    tx: Sender<Message>,
    context: Rc<RefCell<Context>>,
    namespaces: Rc<RefCell<Namespace>>,
}

impl WindowInit {
    pub fn new(
        split_mode: Direction,
        tx: Sender<Message>,
        context: Rc<RefCell<Context>>,
        namespaces: Rc<RefCell<Namespace>>,
    ) -> Self {
        Self {
            split_mode,
            tx,
            context,
            namespaces,
        }
    }

    pub fn build(self) -> Window<'static> {
        let (tabs, popups) = self.tabs_popups();

        let builder = Window::builder().tabs(tabs).popup(popups);

        // Configure Action
        let tx = self.tx.clone();
        let builder = builder.action(
            UserEvent::Key(KeyEvent::new(KeyCode::Char('N'), KeyModifiers::SHIFT)),
            move |w| {
                tx.send(NamespaceRequest::Get.into())
                    .expect("Failed to send NamespaceRequest::Get");
                w.open_popup(view_id::popup_ns);
                EventResult::Nop
            },
        );

        let tx = self.tx.clone();
        let builder = builder.action('n', move |w| {
            tx.send(NamespaceRequest::Get.into())
                .expect("Failed to send NamespaceRequest::Get");
            w.open_popup(view_id::popup_single_ns);
            EventResult::Nop
        });

        let fn_close = |w: &mut Window| {
            if w.opening_popup() {
                w.close_popup();
                EventResult::Nop
            } else {
                EventResult::Window(WindowEvent::CloseWindow)
            }
        };

        let tx = self.tx.clone();
        let builder = builder.action('c', move |w| {
            tx.send(ContextRequest::Get.into())
                .expect("Failed to send ContextRequest::Get");
            w.open_popup(view_id::popup_ctx);
            EventResult::Nop
        });

        let open_help = move |w: &mut Window| {
            w.open_popup(view_id::popup_help);
            EventResult::Nop
        };

        let open_yaml = open_yaml(self.tx.clone());

        let builder = builder.action('h', open_help).action('?', open_help);
        let builder = builder.action('y', open_yaml);

        let builder = builder.action('q', fn_close).action(KeyCode::Esc, fn_close);

        let context = self.context.clone();
        let namespaces = self.namespaces.clone();

        let header = Header::new_callback(2, move || {
            let context = context.borrow();
            let namespaces = namespaces.borrow();
            Paragraph::new(vec![
                Line::from(format!(" ctx: {}", context)),
                Line::from(format!(" ns: {}", namespaces)),
            ])
        });

        let builder = builder.header(header);

        builder.build()
    }

    fn tabs_popups(&self) -> (Vec<Tab<'static>>, Vec<Popup<'static>>) {
        let clipboard = Some(Rc::new(RefCell::new(Clipboard::new())));

        let PodTab {
            tab: pod_tab,
            log_query_help_popup,
        } = PodTab::new(
            "Pod",
            &self.tx,
            &clipboard,
            self.split_mode,
            self.namespaces.clone(),
        );

        let ConfigTab { tab: config_tab } =
            ConfigTab::new("Config", &self.tx, &clipboard, self.split_mode);

        let NetworkTab { tab: tab_network } =
            NetworkTabBuilder::new("Network", &self.tx, &clipboard, self.split_mode).build();

        let EventsTab { tab: tab_events } = EventsTabBuilder::new("Event", &clipboard).build();

        let ListTab {
            tab: tab_list,
            popup: popup_list,
        } = ListTabBuilder::new("List", &self.tx, &clipboard).build();

        let YamlTab {
            tab: tab_yaml,
            popup_kind: popup_yaml_kind,
            popup_name: popup_yaml_name,
            popup_return: popup_yaml_return,
        } = YamlTabBuilder::new("Yaml", &self.tx, &clipboard).build();

        let ContextPopup {
            context: popup_context,
            single_namespace: popup_single_namespace,
            multiple_namespaces: popup_multiple_namespaces,
        } = ContextPopupBuilder::new(&self.tx).build();

        let HelpPopup {
            content: popup_help,
        } = HelpPopup::new();

        let YamlPopup { popup: popup_yaml } = YamlPopupBuilder::new(&clipboard).build();

        // Init Window
        let tabs = vec![
            pod_tab,
            config_tab,
            tab_network,
            tab_events,
            tab_list,
            tab_yaml,
        ];

        let popups = vec![
            Popup::new(popup_context),
            Popup::new(popup_single_namespace),
            Popup::new(popup_multiple_namespaces),
            Popup::new(popup_list),
            Popup::new(popup_yaml_kind),
            Popup::new(popup_yaml_name),
            Popup::new(popup_yaml_return),
            Popup::new(popup_help),
            Popup::new(log_query_help_popup),
            Popup::new(popup_yaml),
        ];

        (tabs, popups)
    }
}

fn open_yaml(tx: Sender<Message>) -> impl Fn(&mut Window) -> EventResult {
    move |w: &mut Window| {
        let widget = w.active_tab().active_widget();

        match widget.id() {
            view_id::tab_pod_widget_pod
            | view_id::tab_config_widget_config
            | view_id::tab_network_widget_network => {}
            _ => {
                return EventResult::Ignore;
            }
        }

        let Some(SelectedItem::TableRow { metadata, .. }) = widget.widget_item() else {
            return EventResult::Ignore;
        };

        let Some(ref metadata) = metadata else {
            return EventResult::Ignore;
        };

        let Some(ref namespace) = metadata.get("namespace") else {
            return EventResult::Ignore;
        };

        let Some(ref name) = metadata.get("name") else {
            return EventResult::Ignore;
        };

        let kind = match widget.id() {
            view_id::tab_pod_widget_pod => DirectedYamlKind::Pod,
            view_id::tab_config_widget_config => match metadata.get("kind").map(|v| v.as_str()) {
                Some("ConfigMap") => DirectedYamlKind::ConfigMap,
                Some("Secret") => DirectedYamlKind::Secret,
                _ => {
                    return EventResult::Ignore;
                }
            },
            view_id::tab_network_widget_network => match metadata.get("kind").map(|v| v.as_str()) {
                Some("Ingress") => DirectedYamlKind::Ingress,
                Some("Service") => DirectedYamlKind::Service,
                Some("Pod") => DirectedYamlKind::Pod,
                Some("NetworkPolicy") => DirectedYamlKind::NetworkPolicy,
                _ => {
                    return EventResult::Ignore;
                }
            },
            _ => return EventResult::Ignore,
        };

        tx.send(
            YamlRequest::DirectedYaml(DirectedYaml {
                name: name.to_string(),
                namespace: namespace.to_string(),
                kind,
            })
            .into(),
        )
        .expect("Failed to send YamlMessage::Request");

        w.widget_clear(view_id::popup_yaml);
        w.open_popup(view_id::popup_yaml);

        EventResult::Nop
    }
}
