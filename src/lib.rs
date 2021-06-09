use crossbeam::channel::Receiver;

use event::{kubernetes::Kube, Event};

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
    generate_id!(tab_pods_widget_pods);
    generate_id!(tab_pods_widget_logs);
    generate_id!(tab_configs);
    generate_id!(tab_configs_widget_configs);
    generate_id!(tab_configs_widget_raw_data);
    generate_id!(tab_event);
    generate_id!(tab_event_widget_event);
    generate_id!(tab_apis);
    generate_id!(tab_apis_widget_apis);

    generate_id!(subwin_ns);
    generate_id!(subwin_apis);
}

#[inline]
fn update_widget_items(window: &mut Window, id: &str, items: WidgetItem) {
    if let Some(w) = window.find_widget_mut(id) {
        w.update_widget_item(items);
    }
}

#[inline]
fn append_widget_items(window: &mut Window, id: &str, items: WidgetItem) {
    if let Some(w) = window.find_widget_mut(id) {
        w.append_widget_item(items);
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

pub fn update_contents(
    window: &mut Window,
    ev: Kube,
    kube_context: &mut String,
    kube_namespace: &mut String,
) {
    match ev {
        Kube::Pod(info) => {
            update_widget_items(
                window,
                view_id::tab_pods_widget_pods,
                WidgetItem::DoubleArray(info),
            );
        }

        Kube::Configs(configs) => {
            update_widget_items(
                window,
                view_id::tab_configs_widget_configs,
                WidgetItem::Array(configs),
            );
        }
        Kube::LogStreamResponse(logs) => {
            append_widget_items(
                window,
                view_id::tab_pods_widget_logs,
                WidgetItem::Array(logs),
            );
        }

        Kube::ConfigResponse(raw) => {
            update_widget_items(
                window,
                view_id::tab_configs_widget_raw_data,
                WidgetItem::Array(raw),
            );
        }

        Kube::GetCurrentContextResponse(ctx, ns) => {
            *kube_context = ctx;
            // let mut cn = current_namespace.borrow_mut();
            *kube_namespace = ns;
        }
        Kube::Event(ev) => {
            update_widget_items(
                window,
                view_id::tab_event_widget_event,
                WidgetItem::Array(ev),
            );
        }
        Kube::APIsResults(apis) => {
            update_widget_items(
                window,
                view_id::tab_apis_widget_apis,
                WidgetItem::Array(apis),
            );
        }
        Kube::GetNamespacesResponse(ns) => {
            update_widget_items(window, view_id::subwin_ns, WidgetItem::Array(ns));
        }

        Kube::GetAPIsResponse(apis) => {
            update_widget_items(window, view_id::subwin_apis, WidgetItem::Array(apis));
        }
        _ => unreachable!(),
    }
}
