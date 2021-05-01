// use std::sync::mpsc::{self, Receiver, Sender};
use crossbeam::channel::{Receiver, Sender};

use crossterm::event::{KeyCode, KeyModifiers};

use tui::layout::Rect;

use event::{kubernetes::*, Event};
use tui_wrapper::{widget::WidgetItem, *};

pub mod view_id {

    #![allow(non_upper_case_globals)]
    macro_rules! generate_id {
        ($id:ident) => {
            pub const $id: &str = stringify!($id);
        };
    }

    generate_id!(tab_pods);
    generate_id!(tab_pods_pane_pods);
    generate_id!(tab_pods_pane_logs);
    generate_id!(tab_configs);
    generate_id!(tab_configs_pane_configs);
    generate_id!(tab_configs_pane_raw_data);
    generate_id!(tab_event);
    generate_id!(tab_event_pane_event);
    generate_id!(tab_apis);
    generate_id!(tab_apis_pane_apis);

    generate_id!(subwin_ns);
    generate_id!(subwin_ns_pane_ns);
    generate_id!(subwin_apis);
    generate_id!(subwin_apis_pane_apis);
}

pub enum WindowEvent {
    CloseWindow,
    Continue,
    OpenSubWindow(&'static str),
    CloseSubWindow,
}

fn update_event(window: &mut Window, ev: Vec<String>) {
    let pane = window.pane_mut(view_id::tab_event_pane_event);
    if let Some(p) = pane {
        let widget = p.widget_mut().text_mut().unwrap();

        let old_select = widget.selected();
        let is_bottom = widget.is_bottom();

        widget.set_items(WidgetItem::Array(ev));

        let new_len = widget.spans().len();

        if is_bottom || (new_len < old_select as usize) {
            widget.select_last();
        } else {
            widget.select(old_select);
        }
    }
}

fn update_pod_logs(window: &mut Window, logs: Vec<String>) {
    let pane = window.pane_mut(view_id::tab_pods_pane_logs);
    if let Some(p) = pane {
        let widget = p.widget_mut().text_mut().unwrap();

        let is_bottom = widget.is_bottom();

        widget.append_items(&logs);

        if is_bottom {
            widget.select_last();
        }
    }
}

fn selected_pod(window: &Window) -> String {
    match window.pane(view_id::tab_pods_pane_pods) {
        Some(pane) => {
            let w = pane.widget().table().unwrap();
            let index = w.state().borrow().selected();

            w.items()[index.unwrap()][0].to_string()
        }
        None => String::new(),
    }
}

fn selected_config(window: &Window) -> String {
    let pane = window.pane(view_id::tab_configs_pane_configs).unwrap();
    let selected_index = pane
        .widget()
        .list()
        .unwrap()
        .state()
        .borrow()
        .selected()
        .unwrap();
    pane.widget().list().unwrap().items()[selected_index].clone()
}

fn update_window_pane_items(window: &mut Window, id: &str, items: WidgetItem) {
    let pane = window.pane_mut(id);
    if let Some(p) = pane {
        let pod = p.widget_mut();
        pod.set_items(items);
    }
}

pub fn apis_subwin_action(
    window: &mut Window,
    subwin: &mut SubWindow,
    _tx: &Sender<Event>,
    rx: &Receiver<Event>,
) -> WindowEvent {
    match rx.recv().unwrap() {
        Event::Input(ev) => match ev.code {
            KeyCode::Char('q') if ev.modifiers == KeyModifiers::CONTROL => {
                return WindowEvent::CloseSubWindow
            }
            KeyCode::Char('n') if ev.modifiers == KeyModifiers::CONTROL => {
                subwin.select_next_item();
            }
            KeyCode::Char('p') if ev.modifiers == KeyModifiers::CONTROL => {
                subwin.select_prev_item();
            }
            KeyCode::Char('u') if ev.modifiers == KeyModifiers::CONTROL => {
                subwin.select_next_item();
            }
            KeyCode::Char('d') if ev.modifiers == KeyModifiers::CONTROL => {
                subwin.select_prev_item();
            }

            KeyCode::Enter | KeyCode::Char(' ') => {}
            _ => {}
        },
        Event::Kube(k) => match k {
            Kube::GetNamespacesResponse(ns) => {
                subwin.set_items(view_id::subwin_ns_pane_ns, WidgetItem::Array(ns))
            }
            _ => {}
        },
        Event::Resize(w, h) => {
            window.update_chunks(Rect::new(0, 0, w, h));
            subwin.update_chunks(Rect::new(0, 0, w, h));
        }
        _ => {}
    }

    WindowEvent::Continue
}

pub fn namespace_subwin_action(
    window: &mut Window,
    subwin: &mut SubWindow,
    tx: &Sender<Event>,
    rx: &Receiver<Event>,
    current_namespace: &mut String,
) -> WindowEvent {
    match rx.recv().unwrap() {
        Event::Input(ev) => match ev.code {
            KeyCode::Char('q') => return WindowEvent::CloseSubWindow,
            KeyCode::Char('j') | KeyCode::Down => {
                subwin.select_next_item();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                subwin.select_prev_item();
            }
            KeyCode::Char('n') if ev.modifiers == KeyModifiers::CONTROL => {
                subwin.select_next_item();
            }
            KeyCode::Char('p') if ev.modifiers == KeyModifiers::CONTROL => {
                subwin.select_prev_item();
            }
            KeyCode::Char('u') if ev.modifiers == KeyModifiers::CONTROL => {
                subwin.select_next_item();
            }
            KeyCode::Char('d') if ev.modifiers == KeyModifiers::CONTROL => {
                subwin.select_prev_item();
            }

            KeyCode::Char('G') => {
                subwin.select_last_item();
            }
            KeyCode::Char('g') => {
                subwin.select_first_item();
            }

            KeyCode::Enter => {
                let list = subwin.selected_pane().widget().list().unwrap();
                let index = list.state().borrow().selected();

                let selected_item = &list.items()[index.unwrap()];

                tx.send(Event::Kube(Kube::SetNamespace(selected_item.to_string())))
                    .unwrap();

                *current_namespace = selected_item.to_string();

                if let Some(p) = window.pane_mut(view_id::tab_event_pane_event) {
                    let w = p.widget_mut().text_mut().unwrap();
                    w.clear();
                }

                if let Some(p) = window.pane_mut(view_id::tab_pods_pane_logs) {
                    let w = p.widget_mut().text_mut().unwrap();
                    w.clear();
                }

                if let Some(p) = window.pane_mut(view_id::tab_configs_pane_raw_data) {
                    let w = p.widget_mut().text_mut().unwrap();
                    w.clear();
                }
                return WindowEvent::CloseSubWindow;
            }
            _ => {}
        },
        Event::Kube(k) => match k {
            Kube::GetNamespacesResponse(ns) => {
                subwin.set_items(view_id::subwin_ns_pane_ns, WidgetItem::Array(ns))
            }
            _ => {}
        },
        Event::Resize(w, h) => {
            window.update_chunks(Rect::new(0, 0, w, h));
            subwin.update_chunks(Rect::new(0, 0, w, h));
        }
        _ => {}
    }

    WindowEvent::Continue
}

pub fn window_action(
    window: &mut Window,
    subwin: &mut SubWindow,
    tx: &Sender<Event>,
    rx: &Receiver<Event>,
    current_namespace: &mut String,
    current_context: &mut String,
) -> WindowEvent {
    match rx.recv().unwrap() {
        Event::Input(ev) => match ev.code {
            KeyCode::Char('q') => {
                return WindowEvent::CloseWindow;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                window.select_next_item();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                window.select_prev_item();
            }
            KeyCode::Char('n') if ev.modifiers == KeyModifiers::CONTROL => {
                window.select_next_item();
            }
            KeyCode::Char('p') if ev.modifiers == KeyModifiers::CONTROL => {
                window.select_prev_item();
            }
            KeyCode::Char('u') if ev.modifiers == KeyModifiers::CONTROL => {
                window.scroll_up();
            }
            KeyCode::Char('d') if ev.modifiers == KeyModifiers::CONTROL => {
                window.scroll_down();
            }
            KeyCode::Tab if ev.modifiers == KeyModifiers::NONE => {
                window.select_next_pane();
            }
            KeyCode::BackTab | KeyCode::Tab if ev.modifiers == KeyModifiers::SHIFT => {
                window.select_prev_pane();
            }
            KeyCode::Char(n @ '1'..='9') => {
                window.select_tab(n as usize - b'0' as usize);
            }
            KeyCode::Char('n') => {
                tx.send(Event::Kube(Kube::GetNamespacesRequest)).unwrap();
                return WindowEvent::OpenSubWindow(view_id::subwin_ns);
            }
            KeyCode::Char('G') => {
                window.select_last_item();
            }
            KeyCode::Char('g') => {
                window.select_first_item();
            }

            KeyCode::Char('/') | KeyCode::Char('f') => {
                if window.selected_tab_id() == view_id::tab_apis {
                    return WindowEvent::OpenSubWindow(view_id::subwin_apis);
                }
            }
            KeyCode::Enter => match window.selected_pane_id() {
                view_id::tab_pods_pane_pods => {
                    window.pane_clear(view_id::tab_pods_pane_logs);
                    tx.send(Event::Kube(Kube::LogStreamRequest(selected_pod(&window))))
                        .unwrap();
                }
                view_id::tab_configs_pane_configs => {
                    window.pane_clear(view_id::tab_configs_pane_configs);
                    tx.send(Event::Kube(Kube::ConfigRequest(selected_config(&window))))
                        .unwrap();
                }
                _ => {}
            },
            _ => {}
        },

        Event::Resize(w, h) => {
            window.update_chunks(Rect::new(0, 0, w, h));
            subwin.update_chunks(Rect::new(0, 0, w, h));
        }
        Event::Tick => {}
        Event::Mouse => {}
        Event::Kube(k) => match k {
            Kube::Pod(info) => {
                update_window_pane_items(
                    window,
                    view_id::tab_pods_pane_pods,
                    WidgetItem::DoubleArray(info),
                );
            }

            Kube::Configs(configs) => {
                update_window_pane_items(
                    window,
                    view_id::tab_configs_pane_configs,
                    WidgetItem::Array(configs),
                );
            }
            Kube::LogStreamResponse(logs) => {
                update_pod_logs(window, logs);
            }

            Kube::ConfigResponse(raw) => {
                update_window_pane_items(
                    window,
                    view_id::tab_configs_pane_raw_data,
                    WidgetItem::Array(raw),
                );
            }

            Kube::GetCurrentContextResponse(ctx, ns) => {
                *current_context = ctx;
                *current_namespace = ns;
            }
            Kube::Event(ev) => {
                update_event(window, ev);
            }
            _ => unreachable!(),
        },
    }
    WindowEvent::Continue
}
