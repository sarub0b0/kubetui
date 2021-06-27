use crossbeam::channel::Receiver;

use event::{
    error::Result,
    kubernetes::{Kube, KubeTable},
    Event,
};

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
    generate_id!(subwin_single_ns);
}

macro_rules! error_format {
    ($fmt:literal, $($arg:tt)*) => {
        format!(concat!("\x1b[31m", $fmt,"\x1b[39m"), $($arg)*)

    };
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

fn update_widget_item_for_table(window: &mut Window, id: &str, table: Result<KubeTable>) {
    let widget = window.find_widget_mut(id);
    let w = widget.as_mut_table();

    match table {
        Ok(table) => {
            if w.equal_header(table.header()) {
                w.update_widget_item(WidgetItem::DoubleArray(table.rows().to_owned()));
            } else {
                w.update_header_and_rows(table.header(), table.rows());
            }
        }
        Err(e) => {
            w.update_header_and_rows(
                &["ERROR".to_string()],
                &[vec![error_format!("{}", e.to_string())]],
            );
        }
    }
}

fn update_widget_item_for_vec(window: &mut Window, id: &str, vec: Result<Vec<String>>) {
    let widget = window.find_widget_mut(id);
    match vec {
        Ok(i) => {
            widget.update_widget_item(WidgetItem::Array(i));
        }
        Err(i) => {
            widget.update_widget_item(WidgetItem::Array(vec![error_format!("{}", i)]));
        }
    }
}

// fn update_widget_item_for_table_for_error(window: &mut Window, id: &str, content: String) {
//     let widget = window.find_widget_mut(id);
//     let w = widget.as_mut_table();

//     w.update_header_and_rows(
//         &["ERROR".to_string()],
//         &[vec![error_format!("{}", content)]],
//     );
// }

pub fn update_contents(
    window: &mut Window,
    ev: Kube,
    current_context: &mut String,
    current_namespace: &mut String,
    selected_namespace: &mut Vec<String>,
) {
    match ev {
        Kube::Pod(pods_table) => {
            update_widget_item_for_table(window, view_id::tab_pods_widget_pods, pods_table);
        }

        Kube::Configs(configs_table) => {
            update_widget_item_for_table(
                window,
                view_id::tab_configs_widget_configs,
                configs_table,
            );
        }

        Kube::LogStreamResponse(logs) => {
            let widget = window.find_widget_mut(view_id::tab_pods_widget_logs);

            match logs {
                Ok(i) => {
                    widget.append_widget_item(WidgetItem::Array(i));
                }
                Err(i) => {
                    widget.append_widget_item(WidgetItem::Array(vec![error_format!("{}", i)]));
                }
            }
        }

        Kube::ConfigResponse(raw) => {
            update_widget_item_for_vec(window, view_id::tab_configs_widget_raw_data, raw);
        }

        Kube::GetCurrentContextResponse(ctx, ns) => {
            *current_context = ctx;
            *current_namespace = ns.to_string();

            selected_namespace.clear();
            selected_namespace.push(ns);
        }

        Kube::Event(ev) => {
            update_widget_item_for_vec(window, view_id::tab_event_widget_event, ev);
        }

        Kube::APIsResults(apis) => {
            update_widget_item_for_vec(window, view_id::tab_apis_widget_apis, apis);
        }

        Kube::GetNamespacesResponse(ns) => {
            window
                .find_widget_mut(view_id::subwin_ns)
                .update_widget_item(WidgetItem::Array(ns.to_vec()));
            window
                .find_widget_mut(view_id::subwin_single_ns)
                .update_widget_item(WidgetItem::Array(ns));

            let widget = window
                .find_widget_mut(view_id::subwin_ns)
                .as_mut_multiple_select();

            if widget.selected_items().is_empty() {
                widget.select_item(&current_namespace)
            }
        }

        Kube::GetAPIsResponse(apis) => {
            update_widget_item_for_vec(window, view_id::subwin_apis, apis);
        }
        _ => unreachable!(),
    }
}
