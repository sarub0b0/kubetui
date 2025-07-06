use std::{cell::RefCell, rc::Rc};

use crossbeam::channel::Sender;
use k8s_openapi::{
    Resource as _,
    api::{
        core::v1::{ConfigMap, Pod, Secret, Service},
        networking::v1::{Ingress, NetworkPolicy},
    },
};
use ratatui::{
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    layout::Direction,
    text::Line,
    widgets::Paragraph,
};

use crate::{
    clipboard::Clipboard,
    config::theme::ThemeConfig,
    features::{
        api_resources::view::ApiTab,
        component_id::{
            CONFIG_WIDGET_ID, CONTEXT_DIALOG_ID, HELP_DIALOG_ID, MULTIPLE_NAMESPACES_DIALOG_ID,
            NETWORK_WIDGET_ID, POD_WIDGET_ID, SINGLE_NAMESPACE_DIALOG_ID, YAML_DIALOG_ID,
        },
        config::view::ConfigTab,
        context::{message::ContextRequest, view::ContextDialog},
        event::view::EventTab,
        get::{
            message::{GetRequest, GetYamlKind},
            view::YamlDialog,
        },
        help::HelpDialog,
        namespace::{
            message::NamespaceRequest,
            view::{MultipleNamespacesDialog, SingleNamespaceDialog},
        },
        network::{
            message::{GatewayVersion, HTTPRouteVersion},
            view::NetworkTab,
        },
        pod::{PodColumns, view::PodTab},
        yaml::view::YamlTab,
    },
    kube::{
        apis::networking::gateway::v1::{Gateway, HTTPRoute},
        context::{Context, Namespace},
    },
    logger,
    message::{Message, UserEvent},
    ui::{
        Header, HeaderTheme, Tab, TabTheme, Window, WindowAction,
        dialog::{Dialog, DialogTheme},
        event::{CallbackFn, EventResult},
        widget::{SelectedItem, WidgetTrait},
    },
};

pub struct WindowInit {
    split_mode: Direction,
    tx: Sender<Message>,
    context: Rc<RefCell<Context>>,
    namespaces: Rc<RefCell<Namespace>>,
    default_pod_columns: Option<PodColumns>,
    theme: ThemeConfig,
}

impl WindowInit {
    pub fn new(
        split_mode: Direction,
        tx: Sender<Message>,
        context: Rc<RefCell<Context>>,
        namespaces: Rc<RefCell<Namespace>>,
        default_pod_columns: Option<PodColumns>,
        theme: ThemeConfig,
    ) -> Self {
        Self {
            split_mode,
            tx,
            context,
            namespaces,
            default_pod_columns,
            theme,
        }
    }

    pub fn build(self) -> Window<'static> {
        let (tabs, dialogs) = self.tabs_dialogs();

        let builder = Window::builder().tabs(tabs).dialogs(dialogs);

        // Configure Action
        let tx = self.tx.clone();
        let builder = builder.action(
            UserEvent::Key(KeyEvent::new(KeyCode::Char('N'), KeyModifiers::SHIFT)),
            move |w: &mut Window| {
                tx.send(NamespaceRequest::Get.into())
                    .expect("Failed to send NamespaceRequest::Get");
                w.open_dialog(MULTIPLE_NAMESPACES_DIALOG_ID);
                EventResult::Nop
            },
        );

        let tx = self.tx.clone();
        let builder = builder.action('n', move |w: &mut Window| {
            tx.send(NamespaceRequest::Get.into())
                .expect("Failed to send NamespaceRequest::Get");
            w.open_dialog(SINGLE_NAMESPACE_DIALOG_ID);
            EventResult::Nop
        });

        let fn_close = |w: &mut Window| {
            if w.opening_dialog() {
                w.close_dialog();
                EventResult::Nop
            } else {
                EventResult::WindowAction(WindowAction::CloseWindow)
            }
        };

        let tx = self.tx.clone();
        let builder = builder.action('c', move |w: &mut Window| {
            tx.send(ContextRequest::Get.into())
                .expect("Failed to send ContextRequest::Get");
            w.open_dialog(CONTEXT_DIALOG_ID);
            EventResult::Nop
        });

        let open_help = move |w: &mut Window| {
            w.open_dialog(HELP_DIALOG_ID);
            EventResult::Nop
        };

        let open_yaml = open_yaml(self.tx.clone());

        let builder = builder.action('h', open_help).action('?', open_help);
        let builder = builder.action('y', open_yaml);

        //　分割方向を変更する
        let toggle_split_direction = move |w: &mut Window| {
            logger!(info, "Toggle split direction");

            w.toggle_split_direction();

            EventResult::Nop
        };

        let builder = builder.action(
            KeyEvent::new(KeyCode::Char('S'), KeyModifiers::SHIFT),
            toggle_split_direction,
        );

        let builder = builder.action('q', fn_close).action(KeyCode::Esc, fn_close);

        let context = self.context.clone();
        let namespaces = self.namespaces.clone();

        let header = Header::new_callback(2, move |theme: &HeaderTheme| {
            let context = context.borrow();
            let namespaces = namespaces.borrow();

            Paragraph::new(vec![
                Line::from(format!(" ctx: {context}")).style(theme.line_styles[0]),
                Line::from(format!(" ns: {namespaces}")).style(theme.line_styles[1]),
            ])
            .style(theme.base_style)
        });

        let builder = builder.header(header);

        let tab_theme = TabTheme::from(self.theme.clone());

        let builder = builder.tab_theme(tab_theme);

        let header_theme = HeaderTheme::from(self.theme.clone());

        let builder = builder.header_theme(header_theme);

        let builder = builder.base_style(self.theme.base);

        builder.build()
    }

    fn tabs_dialogs(&self) -> (Vec<Tab<'static>>, Vec<Dialog<'static>>) {
        let clipboard = arboard::Clipboard::new()
            .inspect_err(|err| {
                logger!(error, "Failed to create clipboard. {}", err);
            })
            .ok()
            .map(|clipboard| Rc::new(RefCell::new(Clipboard::new(clipboard))));

        let PodTab {
            tab: pod_tab,
            log_query_help_dialog,
            pod_columns_dialog,
        } = PodTab::new(
            "Pod",
            &self.tx,
            &clipboard,
            self.split_mode,
            self.namespaces.clone(),
            self.default_pod_columns.clone(),
            self.theme.component.clone(),
        );

        let ConfigTab { tab: config_tab } = ConfigTab::new(
            "Config",
            &self.tx,
            &clipboard,
            self.split_mode,
            self.theme.component.clone(),
        );

        let NetworkTab { tab: network_tab } = NetworkTab::new(
            "Network",
            &self.tx,
            &clipboard,
            self.split_mode,
            self.theme.component.clone(),
        );

        let EventTab { tab: event_tab } =
            EventTab::new("Event", &clipboard, self.theme.component.clone());

        let ApiTab {
            tab: api_tab,
            dialog: api_dialog,
        } = ApiTab::new("API", &self.tx, &clipboard, self.theme.component.clone());

        let YamlTab {
            tab: yaml_tab,
            kind_dialog: yaml_kind_dialog,
            name_dialog: yaml_name_dialog,
            not_found_dialog: yaml_not_found_dialog,
        } = YamlTab::new("Yaml", &self.tx, &clipboard, self.theme.component.clone());

        let ContextDialog {
            widget: context_dialog,
        } = ContextDialog::new(&self.tx, self.theme.clone());

        let SingleNamespaceDialog {
            widget: single_namespace_dialog,
        } = SingleNamespaceDialog::new(&self.tx, self.theme.clone());

        let MultipleNamespacesDialog {
            widget: multiple_namespaces_dialog,
        } = MultipleNamespacesDialog::new(&self.tx, self.theme.clone());

        let HelpDialog {
            widget: help_dialog,
        } = HelpDialog::new(self.theme.clone());

        let YamlDialog {
            widget: yaml_dialog,
        } = YamlDialog::new(&clipboard, self.theme.clone());

        // Init Window
        let tabs = vec![
            pod_tab,
            config_tab,
            network_tab,
            event_tab,
            api_tab,
            yaml_tab,
        ];

        let dialog_theme = DialogTheme::from(self.theme.clone());

        let dialog_widgets = vec![
            context_dialog,
            single_namespace_dialog,
            multiple_namespaces_dialog,
            api_dialog,
            yaml_kind_dialog,
            yaml_name_dialog,
            yaml_not_found_dialog,
            help_dialog,
            log_query_help_dialog,
            pod_columns_dialog,
            yaml_dialog,
        ];

        let dialogs: Vec<Dialog> = dialog_widgets
            .into_iter()
            .map(|widget| {
                Dialog::builder()
                    .widget(widget)
                    .theme(dialog_theme.clone())
                    .build()
            })
            .collect();

        (tabs, dialogs)
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

        let version = metadata.get("version");

        let kind = match metadata.get("kind").map(|v| v.as_str()) {
            Some(Pod::KIND) => GetYamlKind::Pod,
            Some(ConfigMap::KIND) => GetYamlKind::ConfigMap,
            Some(Secret::KIND) => GetYamlKind::Secret,
            Some(Ingress::KIND) => GetYamlKind::Ingress,
            Some(Service::KIND) => GetYamlKind::Service,
            Some(NetworkPolicy::KIND) => GetYamlKind::NetworkPolicy,
            Some(Gateway::KIND) => match version.as_ref().map(|v| v.as_str()) {
                Some("v1") => GetYamlKind::Gateway(GatewayVersion::V1),
                Some("v1beta1") => GetYamlKind::Gateway(GatewayVersion::V1Beta1),
                _ => unreachable!(),
            },
            Some(HTTPRoute::KIND) => match version.as_ref().map(|v| v.as_str()) {
                Some("v1") => GetYamlKind::HTTPRoute(HTTPRouteVersion::V1),
                Some("v1beta1") => GetYamlKind::HTTPRoute(HTTPRouteVersion::V1Beta1),
                _ => unreachable!(),
            },
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

        w.widget_clear(YAML_DIALOG_ID);
        w.open_dialog(YAML_DIALOG_ID);

        EventResult::Nop
    }
}
