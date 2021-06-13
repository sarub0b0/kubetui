use crossbeam::channel::Receiver;

use event::{kubernetes::Kube, Event};

use tui_wrapper::{
    event::{exec_to_window_event, EventResult},
    widget::{ComplexWidget, WidgetItem, WidgetTrait},
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
    generate_id!(subwin_single_ns);
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
    current_context: &mut String,
    current_namespace: &mut String,
    selected_namespace: &mut Vec<String>,
) {
    match ev {
        Kube::Pod(info) => {
            let widget = window.find_widget_mut(view_id::tab_pods_widget_pods);
            let w = widget.as_mut_table();

            if w.equal_header(info.header()) {
                w.update_widget_item(WidgetItem::DoubleArray(info.rows().to_owned()));
            } else {
                w.update_header_and_rows(info.header(), info.rows());
            }
        }

        Kube::Configs(configs) => {
            window
                .find_widget_mut(view_id::tab_configs_widget_configs)
                .update_widget_item(WidgetItem::Array(configs));
        }
        Kube::LogStreamResponse(logs) => {
            window
                .find_widget_mut(view_id::tab_pods_widget_logs)
                .append_widget_item(WidgetItem::Array(logs));
        }

        Kube::ConfigResponse(raw) => {
            window
                .find_widget_mut(view_id::tab_configs_widget_raw_data)
                .update_widget_item(WidgetItem::Array(raw));
        }

        Kube::GetCurrentContextResponse(ctx, ns) => {
            *current_context = ctx;
            *current_namespace = ns.to_string();

            selected_namespace.clear();
            selected_namespace.push(ns);
        }
        Kube::Event(ev) => {
            window
                .find_widget_mut(view_id::tab_event_widget_event)
                .update_widget_item(WidgetItem::Array(ev));
        }
        Kube::APIsResults(apis) => {
            window
                .find_widget_mut(view_id::tab_apis_widget_apis)
                .update_widget_item(WidgetItem::Array(apis));
        }
        Kube::GetNamespacesResponse(ns) => {
            window
                .find_widget_mut(view_id::subwin_ns)
                .update_widget_item(WidgetItem::Array(ns.to_vec()));
            window
                .find_widget_mut(view_id::subwin_single_ns)
                .update_widget_item(WidgetItem::Array(ns));

            let widget = window.find_widget_mut(view_id::subwin_ns);
            if let ComplexWidget::MultipleSelect(widget) = widget.as_mut_complex() {
                if widget.selected_items().is_empty() {
                    widget.select_item(&current_namespace)
                }
            }
        }

        Kube::GetAPIsResponse(apis) => {
            window
                .find_widget_mut(view_id::subwin_apis)
                .update_widget_item(WidgetItem::Array(apis));
        }
        _ => unreachable!(),
    }
}
