use std::collections::BTreeMap;

use crossbeam::channel::Receiver;

use crate::{
    context::{Context, Namespace},
    error::Result,
    event::{
        kubernetes::{
            api_resources::{ApiMessage, ApiResponse},
            config::ConfigMessage,
            network::{NetworkMessage, NetworkResponse},
            yaml::{YamlMessage, YamlResourceListItem, YamlResponse},
            Kube, KubeTable, KubeTableRow,
        },
        Event,
    },
    tui_wrapper::{
        event::{exec_to_window_event, EventResult},
        widget::{Item, LiteralItem, TableItem, WidgetTrait},
        Window, WindowEvent,
    },
};

pub mod view_id {

    #![allow(non_upper_case_globals)]
    macro_rules! generate_id {
        ($id:ident) => {
            pub const $id: &str = stringify!($id);
        };
    }

    generate_id!(tab_pod);
    generate_id!(tab_pod_widget_pod);
    generate_id!(tab_pod_widget_log);
    generate_id!(tab_config);
    generate_id!(tab_config_widget_config);
    generate_id!(tab_config_widget_raw_data);
    generate_id!(tab_network);
    generate_id!(tab_network_widget_network);
    generate_id!(tab_network_widget_description);
    generate_id!(tab_event);
    generate_id!(tab_event_widget_event);
    generate_id!(tab_api);
    generate_id!(tab_api_widget_api);
    generate_id!(tab_yaml);
    generate_id!(tab_yaml_widget_yaml);

    generate_id!(popup_ctx);
    generate_id!(popup_ns);
    generate_id!(popup_api);
    generate_id!(popup_single_ns);

    generate_id!(popup_yaml_name);
    generate_id!(popup_yaml_kind);
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
        Event::Error(_) => {}
    }
    WindowEvent::Continue
}

fn update_widget_item_for_table(window: &mut Window, id: &str, table: Result<KubeTable>) {
    let widget = window.find_widget_mut(id);
    let w = widget.as_mut_table();

    match table {
        Ok(table) => {
            if w.equal_header(table.header()) {
                w.update_widget_item(Item::Table(
                    table
                        .rows
                        .into_iter()
                        .map(
                            |KubeTableRow {
                                 namespace,
                                 name,
                                 metadata,
                                 row,
                             }| {
                                let mut item_metadata = BTreeMap::from([
                                    ("namespace".to_string(), namespace),
                                    ("name".to_string(), name),
                                ]);

                                if let Some(metadata) = metadata {
                                    item_metadata.extend(metadata);
                                }

                                TableItem {
                                    metadata: Some(item_metadata),
                                    item: row,
                                }
                            },
                        )
                        .collect(),
                ));
            } else {
                let rows: Vec<TableItem> = table
                    .rows
                    .into_iter()
                    .map(
                        |KubeTableRow {
                             namespace,
                             name,
                             metadata,
                             row,
                         }| {
                            let mut item_metadata = BTreeMap::from([
                                ("namespace".to_string(), namespace),
                                ("name".to_string(), name),
                            ]);

                            if let Some(metadata) = metadata {
                                item_metadata.extend(metadata);
                            }

                            TableItem {
                                metadata: Some(item_metadata),
                                item: row,
                            }
                        },
                    )
                    .collect();

                w.update_header_and_rows(&table.header, &rows);
            }
        }
        Err(e) => {
            let rows: Vec<TableItem> = vec![vec![error_format!("{}", e)].into()];
            w.update_header_and_rows(&["ERROR".to_string()], &rows);
        }
    }
}

fn update_widget_item_for_vec(window: &mut Window, id: &str, vec: Result<Vec<String>>) {
    let widget = window.find_widget_mut(id);
    match vec {
        Ok(i) => {
            widget.update_widget_item(Item::Array(i.into_iter().map(LiteralItem::from).collect()));
        }
        Err(i) => {
            widget.update_widget_item(Item::Array(vec![error_format!("{}", i).into()]));
        }
    }
}

pub fn update_contents(
    window: &mut Window,
    ev: Kube,
    context: &mut Context,
    namespace: &mut Namespace,
) {
    match ev {
        Kube::Pod(pods_table) => {
            update_widget_item_for_table(window, view_id::tab_pod_widget_pod, pods_table);
        }

        Kube::Config(ConfigMessage::List(configs_table)) => {
            update_widget_item_for_table(window, view_id::tab_config_widget_config, configs_table);
        }

        Kube::LogStreamResponse(logs) => {
            let widget = window.find_widget_mut(view_id::tab_pod_widget_log);

            match logs {
                Ok(i) => {
                    let array = i
                        .into_iter()
                        .map(|i| LiteralItem {
                            metadata: None,
                            item: i,
                        })
                        .collect();
                    widget.append_widget_item(Item::Array(array));
                }
                Err(i) => {
                    widget.append_widget_item(Item::Array(vec![error_format!("{:?}", i).into()]));
                }
            }
        }

        Kube::Config(ConfigMessage::DataResponse(raw)) => {
            update_widget_item_for_vec(window, view_id::tab_config_widget_raw_data, raw);
        }

        Kube::GetCurrentContextResponse {
            current_context,
            current_namespace,
        } => {
            context.update(current_context);
            namespace.update(vec![current_namespace.to_string()]);

            let widget = window
                .find_widget_mut(view_id::popup_ns)
                .as_mut_multiple_select();

            widget.update_widget_item(Item::Array(vec![current_namespace.to_string().into()]));
            widget.select_item(&LiteralItem::from(current_namespace));
        }

        Kube::Event(ev) => {
            update_widget_item_for_vec(window, view_id::tab_event_widget_event, ev);
        }

        Kube::GetNamespacesResponse(ns) => {
            window
                .find_widget_mut(view_id::popup_ns)
                .update_widget_item(Item::Array(
                    ns.iter().cloned().map(LiteralItem::from).collect(),
                ));
            window
                .find_widget_mut(view_id::popup_single_ns)
                .update_widget_item(Item::Array(
                    ns.iter().cloned().map(LiteralItem::from).collect(),
                ));
        }

        Kube::SetNamespacesResponse(ns) => {
            namespace.update(ns);
        }

        Kube::GetContextsResponse(ctxs) => {
            update_widget_item_for_vec(window, view_id::popup_ctx, ctxs);
        }

        Kube::RestoreContext {
            context: ctx,
            namespaces: ns,
        } => {
            context.update(ctx);
            namespace.update(ns.clone());

            window
                .find_widget_mut(view_id::popup_ns)
                .update_widget_item(Item::Array(
                    ns.iter().cloned().map(LiteralItem::from).collect(),
                ));
            window
                .find_widget_mut(view_id::popup_ns)
                .as_mut_multiple_select()
                .select_all();
        }

        Kube::RestoreAPIs(apis) => {
            let w = window
                .find_widget_mut(view_id::popup_api)
                .as_mut_multiple_select();

            for api in apis {
                w.select_item(&api.into());
            }
        }

        Kube::API(ApiMessage::Response(res)) => {
            use ApiResponse::*;
            match res {
                Get(api) => {
                    update_widget_item_for_vec(window, view_id::popup_api, api);
                }
                Set(_) => {}
                Poll(api) => {
                    update_widget_item_for_vec(window, view_id::tab_api_widget_api, api);
                }
            }
        }

        Kube::Yaml(YamlMessage::Response(ev)) => {
            use YamlResponse::*;
            match ev {
                APIs(res) => {
                    update_widget_item_for_vec(window, view_id::popup_yaml_kind, res);
                }

                Resource(res) => {
                    let widget = window.find_widget_mut(view_id::popup_yaml_name);
                    match res {
                        Ok(i) => {
                            widget.update_widget_item(Item::Array(
                                i.items
                                    .into_iter()
                                    .map(
                                        |YamlResourceListItem {
                                             namespace,
                                             name,
                                             kind,
                                             value,
                                         }| LiteralItem {
                                            metadata: Some(BTreeMap::from([
                                                ("namespace".to_string(), namespace),
                                                ("name".to_string(), name),
                                                ("kind".to_string(), kind),
                                            ])),
                                            item: value,
                                        },
                                    )
                                    .collect(),
                            ));
                        }
                        Err(i) => {
                            widget.update_widget_item(Item::Array(vec![
                                error_format!("{}", i).into()
                            ]));
                        }
                    }
                }
                Yaml(res) => {
                    update_widget_item_for_vec(window, view_id::tab_yaml_widget_yaml, res);
                }
            }
        }

        Kube::Network(NetworkMessage::Response(ev)) => {
            use NetworkResponse::*;

            match ev {
                List(res) => {
                    update_widget_item_for_table(window, view_id::tab_network_widget_network, res)
                }
                Yaml(res) => {
                    update_widget_item_for_vec(
                        window,
                        view_id::tab_network_widget_description,
                        res,
                    );
                }
            }
        }

        _ => unreachable!(),
    }
}
