use crossbeam::channel::{Receiver, Sender};

use crossterm::event::{KeyCode, KeyModifiers};

use event::{kubernetes::*, Event};
use tui_wrapper::widget::*;

use component::{multiple_select::MultipleSelect, single_select::SingleSelect};

mod sub_window;
mod window;
pub use sub_window::*;
pub use window::*;

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

// Main Logic
pub enum WindowEvent {
    CloseWindow,
    Continue,
    OpenSubWindow(&'static str),
    CloseSubWindow,
    ResizeWindow,
    UpdateContents(Kube),
}

pub fn update_window_text_pane_items_and_keep_scroll(
    window: &mut Window,
    id: &str,
    items: WidgetItem,
) {
    let pane = window.pane_mut(id);
    if let Some(p) = pane {
        let widget = p.widget_mut().as_mut_text();

        let old_select = widget.state().selected_vertical();
        let is_bottom = widget.is_bottom();

        widget.set_items(items);

        let new_len = widget.spans().len();

        if old_select == 0 {
            widget.select_first();
        } else if is_bottom || (new_len < old_select as usize) {
            widget.select_last();
        } else {
            widget.select_vertical(old_select);
        }
    }
}

pub fn update_event(window: &mut Window, ev: Vec<String>) {
    let pane = window.pane_mut(view_id::tab_event_pane_event);
    if let Some(p) = pane {
        let widget = p.widget_mut().as_mut_text();

        let old_select = widget.state().selected_vertical();
        let is_bottom = widget.is_bottom();

        widget.set_items(WidgetItem::Array(ev));

        let new_len = widget.spans().len();

        if is_bottom || (new_len < old_select as usize) {
            widget.select_last();
        } else {
            widget.select_vertical(old_select);
        }
    }
}

pub fn append_items_window_pane(window: &mut Window, id: &str, items: WidgetItem) {
    let pane = window.pane_mut(id);
    if let Some(p) = pane {
        p.widget_mut().append_items(items)
    }
}

fn selected_pod(window: &Window) -> String {
    match window.pane(view_id::tab_pods_pane_pods) {
        Some(pane) => {
            let w = pane.widget().as_table();
            let index = w.state().selected();

            w.items()[index.unwrap()][0].to_string()
        }
        None => String::new(),
    }
}

fn selected_config(window: &Window) -> String {
    let pane = window.pane(view_id::tab_configs_pane_configs).unwrap();
    let widget = pane.widget().as_list();
    let selected_index = widget.state().selected().unwrap();

    widget.items()[selected_index].clone()
}

pub fn update_window_pane_items(window: &mut Window, id: &str, items: WidgetItem) {
    let pane = window.pane_mut(id);
    if let Some(p) = pane {
        p.set_items(items);
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
        Event::Input(key) => match key.code {
            KeyCode::Char('q') if key.modifiers == KeyModifiers::CONTROL => {
                return WindowEvent::CloseSubWindow
            }

            KeyCode::Char('n') if key.modifiers == KeyModifiers::CONTROL => {
                pane.select_next_item();
            }

            KeyCode::Char('p') if key.modifiers == KeyModifiers::CONTROL => {
                pane.select_prev_item();
            }

            KeyCode::Char('u') if key.modifiers == KeyModifiers::CONTROL => {
                pane.select_next_item();
            }

            KeyCode::Char('d') if key.modifiers == KeyModifiers::CONTROL => {
                pane.select_prev_item();
            }

            KeyCode::Char('h') if key.modifiers == KeyModifiers::CONTROL => {
                pane.remove_char();
            }

            KeyCode::Tab => {
                pane.select_next_pane();
            }

            KeyCode::Enter | KeyCode::Char(' ') => {
                pane.toggle_select_unselect();
                pane.clear_filter();

                tx.send(Event::Kube(Kube::SetAPIsRequest(
                    pane.to_vec_selected_items(),
                )))
                .unwrap();

                if pane.selected_items().is_empty() {
                    window.pane_clear(view_id::tab_apis_pane_apis)
                }
            }

            KeyCode::Delete | KeyCode::Backspace => {
                pane.remove_char();
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

            _ => {}
        },
        Event::Kube(k) => return WindowEvent::UpdateContents(k),
        Event::Resize(_w, _h) => {
            return WindowEvent::ResizeWindow;
        }
        _ => {}
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
        Event::Input(key) => match key.code {
            KeyCode::Char('q') if key.modifiers == KeyModifiers::CONTROL => {
                return WindowEvent::CloseSubWindow
            }

            KeyCode::Char('n') if key.modifiers == KeyModifiers::CONTROL => {
                pane.select_next_item();
            }

            KeyCode::Char('p') if key.modifiers == KeyModifiers::CONTROL => {
                pane.select_prev_item();
            }

            KeyCode::Char('u') if key.modifiers == KeyModifiers::CONTROL => {
                pane.select_next_item();
            }

            KeyCode::Char('d') if key.modifiers == KeyModifiers::CONTROL => {
                pane.select_prev_item();
            }

            KeyCode::Char('h') if key.modifiers == KeyModifiers::CONTROL => {
                pane.remove_char();
            }

            KeyCode::Tab => {
                pane.select_next_pane();
            }

            KeyCode::Delete | KeyCode::Backspace => {
                pane.remove_char();
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
        Event::Kube(k) => return WindowEvent::UpdateContents(k),
        Event::Resize(_w, _h) => {
            return WindowEvent::ResizeWindow;
        }
        _ => {}
    }

    WindowEvent::Continue
}

pub fn window_action(window: &mut Window, tx: &Sender<Event>, rx: &Receiver<Event>) -> WindowEvent {
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
            KeyCode::Char('f') if ev.modifiers == KeyModifiers::CONTROL => {
                if window.selected_pane_id() == view_id::tab_apis_pane_apis {
                    if let Some(pane) = window.pane_mut(view_id::tab_apis_pane_apis) {
                        let w = pane.widget_mut().as_mut_text();
                        w.scroll_right(10);
                    }
                }
            }
            KeyCode::Char('b') if ev.modifiers == KeyModifiers::CONTROL => {
                if window.selected_pane_id() == view_id::tab_apis_pane_apis {
                    if let Some(pane) = window.pane_mut(view_id::tab_apis_pane_apis) {
                        let w = pane.widget_mut().as_mut_text();
                        w.scroll_left(10);
                    }
                }
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
                    tx.send(Event::Kube(Kube::GetAPIsRequest)).unwrap();
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

        Event::Resize(_w, _h) => {
            return WindowEvent::ResizeWindow;
        }
        Event::Tick => {}
        Event::Mouse => {}
        Event::Kube(k) => return WindowEvent::UpdateContents(k),
    }
    WindowEvent::Continue
}
