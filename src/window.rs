mod api;
mod config;
mod context;
mod event;
mod pod;
mod yaml;

use self::event::*;
use api::*;
use config::*;
use context::*;
use pod::*;
use yaml::*;

use crossbeam::channel::Sender;

use std::{cell::RefCell, rc::Rc};

use crate::clipboard_wrapper::{ClipboardContextWrapper, ClipboardProvider};

use crate::event::{kubernetes::*, Event, UserEvent};

use crate::action::view_id;
use crate::context::{Context, Namespace};

use crate::tui_wrapper::{event::EventResult, widget::Widget, Tab, Window, WindowEvent};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use tui::{layout::Direction, text::Spans, widgets::Paragraph};

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
            tx.send(Event::Kube(Kube::GetContextsRequest)).unwrap();
            w.open_popup(view_id::popup_ctx);
            EventResult::Nop
        });

        let builder = builder.action('q', fn_close).action(KeyCode::Esc, fn_close);

        let context = self.context.clone();
        let namespaces = self.namespaces.clone();

        let builder = builder.header(2, move || {
            let ns = namespaces.borrow();
            let ctx = context.borrow();

            Paragraph::new(vec![
                Spans::from(format!(" ctx: {}", ctx)),
                Spans::from(format!(" ns: {}", ns)),
            ])
        });

        builder.build()
    }

    fn tabs_popups(&self) -> (Vec<Tab<'static>>, Vec<Widget<'static>>) {
        let clipboard = match ClipboardContextWrapper::new() {
            Ok(cb) => Some(Rc::new(RefCell::new(cb))),
            Err(_) => None,
        };

        let PodsTab { tab: tab_pods } = PodTabBuilder::new(
            "1:Pods",
            &self.tx,
            &self.namespaces,
            &clipboard,
            self.split_mode.clone(),
        )
        .build();

        let ConfigsTab { tab: tab_configs } = ConfigsTabBuilder::new(
            "2:Configs",
            &self.tx,
            &self.namespaces,
            &clipboard,
            self.split_mode.clone(),
        )
        .build();

        let EventsTab { tab: tab_events } = EventsTabBuilder::new("3:Event", &clipboard).build();

        let APIsTab {
            tab: tab_apis,
            popup: popup_apis,
        } = APIsTabBuilder::new("4:APIs", &self.tx, &clipboard).build();

        let YamlTab {
            tab: tab_yaml,
            popup_kind: popup_yaml_kind,
            popup_name: popup_yaml_name,
        } = YamlTabBuilder::new("5:Yaml", &self.tx, &self.namespaces, &clipboard).build();

        let ContextPopup {
            context: popup_context,
            single_namespace: popup_single_namespace,
            multiple_namespaces: popup_multiple_namespaces,
        } = ContextPopupBuilder::new(&self.tx, &self.context, &self.namespaces).build();

        // Init Window
        let tabs = vec![tab_pods, tab_configs, tab_events, tab_apis, tab_yaml];

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
