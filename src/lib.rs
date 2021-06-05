use crossbeam::channel::{Receiver, Sender};

use tui_wrapper::crossterm::event::{KeyCode, KeyModifiers};

use event::{kubernetes::*, Event, UserEvent};

use tui_wrapper::widget::*;
use tui_wrapper::{key_event_to_code, EventResult};

use tui_wrapper::complex_widgets::{multiple_select::MultipleSelect, single_select::SingleSelect};

pub use tui_wrapper::sub_window::*;
pub use tui_wrapper::window::*;

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
    generate_id!(subwin_apis_pane);
    generate_id!(subwin_apis_pane_filter);
    generate_id!(subwin_apis_pane_items);
    generate_id!(subwin_apis_pane_selected);
}

pub fn set_items_window_pane(window: &mut Window, id: &str, items: WidgetItem) {
    if let Some(pane) = window.pane_mut(id) {
        pane.set_items(items);
    }
}

pub fn append_items_window_pane(window: &mut Window, id: &str, items: WidgetItem) {
    if let Some(pane) = window.pane_mut(id) {
        pane.append_items(items);
    }
}

pub fn apis_subwin_action<'a, P>(
    window: &mut Window,
    subwin: &mut SubWindow<P>,
    tx: &Sender<Event>,
    rx: &Receiver<Event>,
) -> WindowEvent
where
    P: PaneTrait<Item = MultipleSelect<'a>>,
{
    let pane = subwin.pane_mut();

    match rx.recv().unwrap() {
        Event::User(ev) => match ev {
            UserEvent::Key(key) => match key_event_to_code(key) {
                KeyCode::Esc => {
                    return WindowEvent::CloseSubWindow;
                }

                KeyCode::Down => {
                    pane.select_next_item();
                }

                KeyCode::Up => {
                    pane.select_prev_item();
                }

                KeyCode::PageDown => {
                    pane.select_next_item();
                }

                KeyCode::PageUp => {
                    pane.select_prev_item();
                }

                KeyCode::Delete => {
                    pane.remove_char();
                }

                KeyCode::Char('w') if key.modifiers == KeyModifiers::CONTROL => {
                    pane.remove_chars_before_cursor();
                }

                KeyCode::Char('k') if key.modifiers == KeyModifiers::CONTROL => {
                    pane.remove_chars_after_cursor();
                }

                KeyCode::Home => {
                    pane.move_cursor_top();
                }

                KeyCode::End => {
                    pane.move_cursor_end();
                }

                KeyCode::Tab => {
                    pane.select_next_pane();
                }

                KeyCode::Right => {
                    pane.forward_cursor();
                }

                KeyCode::Left => {
                    pane.back_cursor();
                }

                KeyCode::Enter | KeyCode::Char(' ') => {
                    pane.toggle_select_unselect();

                    tx.send(Event::Kube(Kube::SetAPIsRequest(
                        pane.to_vec_selected_items(),
                    )))
                    .unwrap();

                    if pane.selected_items().is_empty() {
                        window.pane_clear(view_id::tab_apis_pane_apis)
                    }
                }

                KeyCode::Char(c) => {
                    pane.insert_char(c);
                }

                _ => {}
            },
            UserEvent::Mouse(ev) => {
                let _callback = subwin.on_mouse_event(ev);
            }
            UserEvent::Resize(w, h) => {
                return WindowEvent::ResizeWindow(w, h);
            }
        },
        Event::Kube(k) => return WindowEvent::UpdateContents(k),
        Event::Tick => {}
    }

    WindowEvent::Continue
}

pub fn namespace_subwin_action<'a, P>(
    window: &mut Window,
    subwin: &mut SubWindow<P>,
    tx: &Sender<Event>,
    rx: &Receiver<Event>,
    current_namespace: &mut String,
) -> WindowEvent
where
    P: PaneTrait<Item = SingleSelect<'a>>,
{
    let pane = subwin.pane_mut();
    match rx.recv().unwrap() {
        Event::User(ev) => match ev {
            UserEvent::Key(key) => match key_event_to_code(key) {
                KeyCode::Esc => {
                    return WindowEvent::CloseSubWindow;
                }

                KeyCode::Down => {
                    pane.select_next_item();
                }

                KeyCode::Up => {
                    pane.select_prev_item();
                }

                KeyCode::PageUp => {
                    pane.select_prev_item();
                }

                KeyCode::PageDown => {
                    pane.select_next_item();
                }

                KeyCode::Delete => {
                    pane.remove_char();
                }

                KeyCode::Char('w') if key.modifiers == KeyModifiers::CONTROL => {
                    pane.remove_chars_before_cursor();
                }

                KeyCode::Char('k') if key.modifiers == KeyModifiers::CONTROL => {
                    pane.remove_chars_after_cursor();
                }

                KeyCode::Home => {
                    pane.move_cursor_top();
                }

                KeyCode::End => {
                    pane.move_cursor_end();
                }

                KeyCode::Tab => {
                    pane.select_next_pane();
                }

                KeyCode::Right => {
                    pane.forward_cursor();
                }

                KeyCode::Left => {
                    pane.back_cursor();
                }

                KeyCode::Char(c) => {
                    pane.insert_char(c);
                }

                KeyCode::Enter => {
                    if let Some(item) = pane.get_item() {
                        let item = item.single();

                        tx.send(Event::Kube(Kube::SetNamespace(item.to_string())))
                            .unwrap();

                        *current_namespace = item;

                        if let Some(p) = window.pane_mut(view_id::tab_event_pane_event) {
                            p.clear();
                        }

                        if let Some(p) = window.pane_mut(view_id::tab_pods_pane_logs) {
                            p.clear();
                            window.select_pane(view_id::tab_pods_pane_pods);
                        }

                        if let Some(p) = window.pane_mut(view_id::tab_configs_pane_raw_data) {
                            p.clear();
                            window.select_pane(view_id::tab_configs_pane_configs);
                        }
                    }
                    return WindowEvent::CloseSubWindow;
                }
                _ => {}
            },
            UserEvent::Mouse(ev) => {
                let _callback = subwin.on_mouse_event(ev);
            }
            UserEvent::Resize(w, h) => {
                return WindowEvent::ResizeWindow(w, h);
            }
        },

        Event::Kube(k) => return WindowEvent::UpdateContents(k),
        Event::Tick => {}
    }

    WindowEvent::Continue
}

pub fn window_action(window: &mut Window, rx: &Receiver<Event>) -> WindowEvent {
    match rx.recv().unwrap() {
        Event::User(ev) => match ev {
            UserEvent::Key(_) | UserEvent::Mouse(_) => match window.on_event(ev) {
                EventResult::Nop => {}

                EventResult::Ignore => {
                    if let Some(cb) = window.match_callback(ev) {
                        match (cb)(window) {
                            EventResult::WindowEvent(ev) => {
                                return ev;
                            }
                            _ => {}
                        }
                    }
                }
                ev @ EventResult::Callback(_) => {
                    ev.exec(window);
                }
                EventResult::WindowEvent(ev) => {
                    return ev;
                }
            },

            UserEvent::Resize(w, h) => {
                return WindowEvent::ResizeWindow(w, h);
            }
        },

        Event::Tick => {}
        Event::Kube(k) => return WindowEvent::UpdateContents(k),
    }
    WindowEvent::Continue
}
