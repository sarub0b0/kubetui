mod api;
mod config;
mod context;
mod event;
mod network;
mod pod;
mod yaml;

use self::network::{NetworkTab, NetworkTabBuilder};
use api::*;
use config::*;
use context::*;
use event::*;
use pod::*;
use yaml::*;

use std::{cell::RefCell, rc::Rc};

use crossbeam::channel::Sender;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tui::{layout::Direction, text::Spans, widgets::Paragraph};

use crate::{
    action::view_id,
    clipboard_wrapper::{ClipboardContextWrapper, ClipboardProvider},
    context::{Context, Namespace},
    event::{
        kubernetes::{context_message::ContextRequest, *},
        Event, UserEvent,
    },
    tui_wrapper::{event::EventResult, widget::Widget, Header, Tab, Window, WindowEvent},
};

pub struct WindowInit {
    split_mode: Direction,
    tx: Sender<Event>,
    context: Rc<RefCell<Context>>,
    namespaces: Rc<RefCell<Namespace>>,
}

impl WindowInit {
    pub fn new(
        split_mode: Direction,
        tx: Sender<Event>,
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
            UserEvent::Key(KeyEvent {
                code: KeyCode::Char('N'),
                modifiers: KeyModifiers::SHIFT,
            }),
            move |w| {
                tx.send(Event::Kube(Kube::GetNamespacesRequest)).unwrap();
                w.open_popup(view_id::popup_ns);
                EventResult::Nop
            },
        );

        let tx = self.tx.clone();
        let builder = builder.action('n', move |w| {
            tx.send(Event::Kube(Kube::GetNamespacesRequest)).unwrap();
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
            tx.send(ContextRequest::Get.into()).unwrap();
            w.open_popup(view_id::popup_ctx);
            EventResult::Nop
        });

        let builder = builder.action('q', fn_close).action(KeyCode::Esc, fn_close);

        let context = self.context.clone();
        let namespaces = self.namespaces.clone();

        let header = Header::new_callback(2, move || {
            let context = context.borrow();
            let namespaces = namespaces.borrow();
            Paragraph::new(vec![
                Spans::from(format!(" ctx: {}", context)),
                Spans::from(format!(" ns: {}", namespaces)),
            ])
        });

        let builder = builder.header(header);

        builder.build()
    }

    fn tabs_popups(&self) -> (Vec<Tab<'static>>, Vec<Widget<'static>>) {
        let clipboard = match ClipboardContextWrapper::new() {
            Ok(cb) => Some(Rc::new(RefCell::new(cb))),
            Err(_) => None,
        };

        let PodsTab { tab: tab_pods } =
            PodTabBuilder::new("Pod", &self.tx, &clipboard, self.split_mode.clone()).build();

        let ConfigTab { tab: tab_configs } =
            ConfigTabBuilder::new("Config", &self.tx, &clipboard, self.split_mode.clone()).build();

        let NetworkTab { tab: tab_network } =
            NetworkTabBuilder::new("Network", &self.tx, &clipboard, self.split_mode.clone())
                .build();

        let EventsTab { tab: tab_events } = EventsTabBuilder::new("Event", &clipboard).build();

        let ApiTab {
            tab: tab_apis,
            popup: popup_apis,
        } = ApiTabBuilder::new("API", &self.tx, &clipboard).build();

        let YamlTab {
            tab: tab_yaml,
            popup_kind: popup_yaml_kind,
            popup_name: popup_yaml_name,
        } = YamlTabBuilder::new("Yaml", &self.tx, &clipboard).build();

        let ContextPopup {
            context: popup_context,
            single_namespace: popup_single_namespace,
            multiple_namespaces: popup_multiple_namespaces,
        } = ContextPopupBuilder::new(&self.tx).build();

        // Init Window
        let tabs = vec![
            tab_pods,
            tab_configs,
            tab_network,
            tab_events,
            tab_apis,
            tab_yaml,
        ];

        let popups = vec![
            popup_context,
            popup_single_namespace,
            popup_multiple_namespaces,
            popup_apis,
            popup_yaml_kind,
            popup_yaml_name,
        ];

        (tabs, popups)
    }
}
