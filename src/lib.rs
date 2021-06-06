use crossbeam::channel::Receiver;

use event::Event;

use tui_wrapper::{
    event::{exec_to_window_event, EventResult},
    widget::{WidgetItem, WidgetTrait},
    Window, WindowEvent,
};

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

pub fn window_action(window: &mut Window, rx: &Receiver<Event>) -> WindowEvent {
    match rx.recv().unwrap() {
        Event::User(ev) => match window.on_event(ev) {
            EventResult::Nop => {}

            EventResult::Ignore => {
                if let Some(cb) = window.match_callback(ev) {
                    if let EventResult::Window(ev) = (cb)(window) {
                        return ev;
                    }
                }
            }
            ev @ EventResult::Callback(_) => {
                return exec_to_window_event(ev, window);
            }
            EventResult::Window(ev) => {
                return ev;
            }
        },

        Event::Tick => {}
        Event::Kube(k) => return WindowEvent::UpdateContents(k),
    }
    WindowEvent::Continue
}
