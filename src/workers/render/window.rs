use std::{cell::RefCell, rc::Rc};

use crossbeam::channel::Sender;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use k8s_openapi::{
    api::{
        core::v1::{ConfigMap, Pod, Secret, Service},
        networking::v1::{Ingress, NetworkPolicy},
    },
    Resource as _,
};
use ratatui::{layout::Direction, text::Line, widgets::Paragraph};

use crate::{
    clipboard::Clipboard,
    features::{
        api_resources::view::ListTab,
        component_id::{
            CONFIG_WIDGET_ID, CONTEXT_POPUP_ID, HELP_POPUP_ID, MULTIPLE_NAMESPACES_POPUP_ID,
            NETWORK_WIDGET_ID, POD_WIDGET_ID, SINGLE_NAMESPACE_POPUP_ID, YAML_POPUP_ID,
        },
        config::view::ConfigTab,
        context::{message::ContextRequest, view::ContextPopup},
        event::view::EventTab,
        get::{
            message::{GetRequest, GetYamlKind},
            view::YamlPopup,
        },
        help::HelpPopup,
        namespace::{
            message::NamespaceRequest,
            view::{MultipleNamespacesPopup, SingleNamespacePopup},
        },
        network::view::NetworkTab,
        pod::view::PodTab,
        yaml::view::YamlTab,
    },
    kube::{
        apis::networking::gateway::v1::{Gateway, HTTPRoute},
        context::{Context, Namespace},
    },
    message::{Message, UserEvent},
    ui::{
        event::{CallbackFn, EventResult},
        popup::Popup,
        widget::{SelectedItem, WidgetTrait},
        Header, Tab, Window, WindowAction,
    },
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
            move |w: &mut Window| {
                tx.send(NamespaceRequest::Get.into())
                    .expect("Failed to send NamespaceRequest::Get");
                w.open_popup(MULTIPLE_NAMESPACES_POPUP_ID);
                EventResult::Nop
            },
        );

        let tx = self.tx.clone();
        let builder = builder.action('n', move |w: &mut Window| {
            tx.send(NamespaceRequest::Get.into())
                .expect("Failed to send NamespaceRequest::Get");
            w.open_popup(SINGLE_NAMESPACE_POPUP_ID);
            EventResult::Nop
        });

        let fn_close = |w: &mut Window| {
            if w.opening_popup() {
                w.close_popup();
                EventResult::Nop
            } else {
                EventResult::WindowAction(WindowAction::CloseWindow)
            }
        };

        let tx = self.tx.clone();
        let builder = builder.action('c', move |w: &mut Window| {
            tx.send(ContextRequest::Get.into())
                .expect("Failed to send ContextRequest::Get");
            w.open_popup(CONTEXT_POPUP_ID);
            EventResult::Nop
        });

        let open_help = move |w: &mut Window| {
            w.open_popup(HELP_POPUP_ID);
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

        let NetworkTab { tab: network_tab } =
            NetworkTab::new("Network", &self.tx, &clipboard, self.split_mode);

        let EventTab { tab: event_tab } = EventTab::new("Event", &clipboard);

        let ListTab {
            tab: list_tab,
            popup: list_popup,
        } = ListTab::new("List", &self.tx, &clipboard);

        let YamlTab {
            tab: yaml_tab,
            kind_popup: yaml_kind_popup,
            name_popup: yaml_name_popup,
            not_found_popup: yaml_not_found_popup,
        } = YamlTab::new("Yaml", &self.tx, &clipboard);

        let ContextPopup {
            popup: context_popup,
        } = ContextPopup::new(&self.tx);

        let SingleNamespacePopup {
            popup: single_namespace_popup,
        } = SingleNamespacePopup::new(&self.tx);

        let MultipleNamespacesPopup {
            popup: multiple_namespaces_popup,
        } = MultipleNamespacesPopup::new(&self.tx);

        let HelpPopup { popup: help_popup } = HelpPopup::new();

        let YamlPopup { popup: yaml_popup } = YamlPopup::new(&clipboard);

        // Init Window
        let tabs = vec![
            pod_tab,
            config_tab,
            network_tab,
            event_tab,
            list_tab,
            yaml_tab,
        ];

        let popups = vec![
            Popup::new(context_popup),
            Popup::new(single_namespace_popup),
            Popup::new(multiple_namespaces_popup),
            Popup::new(list_popup),
            Popup::new(yaml_kind_popup),
            Popup::new(yaml_name_popup),
            Popup::new(yaml_not_found_popup),
            Popup::new(help_popup),
            Popup::new(log_query_help_popup),
            Popup::new(yaml_popup),
        ];

        (tabs, popups)
    }
}

fn open_yaml(tx: Sender<Message>) -> impl CallbackFn {
    move |w: &mut Window| {
        let widget = w.active_tab().active_widget();

        match widget.id() {
            POD_WIDGET_ID | CONFIG_WIDGET_ID | NETWORK_WIDGET_ID => {}
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

        let kind = match metadata.get("kind").map(|v| v.as_str()) {
            Some(Pod::KIND) => GetYamlKind::Pod,
            Some(ConfigMap::KIND) => GetYamlKind::ConfigMap,
            Some(Secret::KIND) => GetYamlKind::Secret,
            Some(Ingress::KIND) => GetYamlKind::Ingress,
            Some(Service::KIND) => GetYamlKind::Service,
            Some(NetworkPolicy::KIND) => GetYamlKind::NetworkPolicy,
            Some(Gateway::KIND) => GetYamlKind::Gateway,
            Some(HTTPRoute::KIND) => GetYamlKind::HTTPRoute,
            _ => {
                unreachable!();
            }
        };

        tx.send(
            GetRequest {
                name: name.to_string(),
                namespace: namespace.to_string(),
                kind,
            }
            .into(),
        )
        .expect("Failed to send YamlMessage::Request");

        w.widget_clear(YAML_POPUP_ID);
        w.open_popup(YAML_POPUP_ID);

        EventResult::Nop
    }
}
