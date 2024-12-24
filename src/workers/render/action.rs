use std::collections::BTreeMap;

use anyhow::Result;
use crossbeam::channel::Receiver;

use crate::{
    features::{
        api_resources::message::{ApiMessage, ApiResponse},
        component_id::{
            API_DIALOG_ID, API_WIDGET_ID, CONFIG_RAW_DATA_WIDGET_ID, CONFIG_WIDGET_ID,
            CONTEXT_DIALOG_ID, EVENT_WIDGET_ID, MULTIPLE_NAMESPACES_DIALOG_ID,
            NETWORK_DESCRIPTION_WIDGET_ID, NETWORK_WIDGET_ID, POD_LOG_WIDGET_ID, POD_WIDGET_ID,
            SINGLE_NAMESPACE_DIALOG_ID, YAML_DIALOG_ID, YAML_KIND_DIALOG_ID, YAML_NAME_DIALOG_ID,
            YAML_NOT_FOUND_DIALOG_ID, YAML_WIDGET_ID,
        },
        config::message::ConfigMessage,
        context::message::{ContextMessage, ContextResponse},
        get::message::{GetMessage, GetResponse},
        namespace::message::{NamespaceMessage, NamespaceResponse},
        network::message::{NetworkMessage, NetworkResponse},
        pod::message::LogMessage,
        yaml::message::{YamlMessage, YamlResourceListItem, YamlResponse},
    },
    kube::{
        context::{Context, Namespace},
        table::{KubeTable, KubeTableRow},
    },
    logger,
    message::Message,
    ui::{
        event::{Callback, EventResult},
        util::chars::convert_tabs_to_spaces,
        widget::{Item, LiteralItem, TableItem, WidgetTrait},
        Window, WindowAction,
    },
    workers::kube::message::Kube,
};

macro_rules! error_format {
    ($fmt:literal, $($arg:tt)*) => {
        format!(concat!("\x1b[31m[kubetui] ", $fmt,"\x1b[39m"), $($arg)*)
    };
}

macro_rules! error_lines {
    ($err:ident) => {
        format!("{:?}", $err)
            .lines()
            .map(|line| LiteralItem {
                item: error_format!("{}", line),
                metadata: None,
            })
            .collect::<Vec<_>>()
    };
}

/// 各ウィジェットのコールバックを実行する
/// コールバックがコールバックを返す場合は再帰的に実行する
fn exec_callback(cb: Callback, w: &mut Window) -> WindowAction {
    let mut result = cb(w);

    while let EventResult::Callback(next_cb) = result {
        result = next_cb(w);
    }

    WindowAction::Continue
}

pub fn window_action(window: &mut Window, rx: &Receiver<Message>) -> WindowAction {
    match rx.recv().expect("Failed to recv") {
        Message::User(ev) => match window.on_event(ev) {
            EventResult::Nop => {}

            EventResult::Ignore => {
                if let Some(cb) = window.match_callback(ev) {
                    if let EventResult::WindowAction(action) = (cb)(window) {
                        return action;
                    }
                }
            }
            EventResult::Callback(cb) => {
                return exec_callback(cb, window);
            }
            EventResult::WindowAction(action) => {
                return action;
            }
        },

        Message::Tick => {}
        Message::Kube(k) => return WindowAction::UpdateContents(k),
        Message::Error(err) => {
            logger!(error, "Error: {:?}", err);
        }
    }
    WindowAction::Continue
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
            let rows: Vec<TableItem> = vec![vec![error_format!("{:?}", e)].into()];
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
        Err(e) => {
            widget.update_widget_item(Item::Array(error_lines!(e)));
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
            update_widget_item_for_table(window, POD_WIDGET_ID, pods_table);
        }

        Kube::Log(LogMessage::Response(res)) => {
            let widget = window.find_widget_mut(POD_LOG_WIDGET_ID);

            match res {
                Ok(i) => {
                    let array = i
                        .into_iter()
                        .map(|i| LiteralItem {
                            metadata: None,
                            item: convert_tabs_to_spaces(i),
                        })
                        .collect();

                    widget.append_widget_item(Item::Array(array));
                }
                Err(e) => {
                    widget.append_widget_item(Item::Array(error_lines!(e)));
                }
            }
        }

        Kube::Config(ConfigMessage::Response(res)) => {
            use crate::features::config::message::ConfigResponse::*;

            match res {
                Table(list) => {
                    update_widget_item_for_table(window, CONFIG_WIDGET_ID, list);
                }
                Data(data) => {
                    update_widget_item_for_vec(window, CONFIG_RAW_DATA_WIDGET_ID, data);
                }
            }
        }

        Kube::Event(ev) => {
            update_widget_item_for_vec(window, EVENT_WIDGET_ID, ev);
        }

        Kube::Namespace(NamespaceMessage::Response(res)) => match res {
            NamespaceResponse::Get(res) => match res {
                Ok(namespaces) => {
                    window
                        .find_widget_mut(MULTIPLE_NAMESPACES_DIALOG_ID)
                        .update_widget_item(Item::Array(
                            namespaces.iter().cloned().map(LiteralItem::from).collect(),
                        ));
                    window
                        .find_widget_mut(SINGLE_NAMESPACE_DIALOG_ID)
                        .update_widget_item(Item::Array(
                            namespaces.iter().cloned().map(LiteralItem::from).collect(),
                        ));
                }
                Err(err) => {
                    let err = error_lines!(err);
                    window
                        .find_widget_mut(MULTIPLE_NAMESPACES_DIALOG_ID)
                        .update_widget_item(Item::Array(err.to_vec()));

                    window
                        .find_widget_mut(SINGLE_NAMESPACE_DIALOG_ID)
                        .update_widget_item(Item::Array(err));
                }
            },
            NamespaceResponse::Set(res) => {
                namespace.update(res);
            }
        },

        Kube::Context(ContextMessage::Response(res)) => match res {
            ContextResponse::Get(res) => {
                update_widget_item_for_vec(window, CONTEXT_DIALOG_ID, Ok(res));
            }
        },

        Kube::RestoreContext {
            context: ctx,
            namespaces: ns,
        } => {
            context.update(ctx);
            namespace.update(ns.clone());

            window
                .find_widget_mut(MULTIPLE_NAMESPACES_DIALOG_ID)
                .update_widget_item(Item::Array(
                    ns.iter().cloned().map(LiteralItem::from).collect(),
                ));
            window
                .find_widget_mut(MULTIPLE_NAMESPACES_DIALOG_ID)
                .as_mut_multiple_select()
                .select_all();
        }

        Kube::RestoreAPIs(apis) => {
            let w = window
                .find_widget_mut(API_DIALOG_ID)
                .as_mut_multiple_select();

            for api in apis {
                let Ok(json) = serde_json::to_string(&api.resource) else {
                    unreachable!()
                };

                let metadata = BTreeMap::from([("key".into(), json)]);

                let literal_item = LiteralItem::new(api.to_string(), Some(metadata));

                w.select_item(&literal_item);
            }
        }

        Kube::Api(ApiMessage::Response(res)) => {
            use ApiResponse::*;
            match res {
                Get(apis) => {
                    let widget = window.find_widget_mut(API_DIALOG_ID);
                    match apis {
                        Ok(i) => {
                            let items = i
                                .into_iter()
                                .map(|api_resource| {
                                    let Ok(json) = serde_json::to_string(&api_resource.resource)
                                    else {
                                        unreachable!()
                                    };
                                    let metadata = BTreeMap::from([("key".into(), json)]);

                                    LiteralItem::new(api_resource.to_string(), Some(metadata))
                                })
                                .collect();

                            widget.update_widget_item(Item::Array(items));
                        }
                        Err(e) => {
                            widget.update_widget_item(Item::Array(error_lines!(e)));
                        }
                    }
                }
                Poll(apis) => {
                    update_widget_item_for_vec(window, API_WIDGET_ID, apis);
                }
            }
        }

        Kube::Yaml(YamlMessage::Response(ev)) => {
            use YamlResponse::*;
            match ev {
                APIs(res) => {
                    let widget = window.find_widget_mut(YAML_KIND_DIALOG_ID);
                    match res {
                        Ok(vec) => {
                            let items = vec
                                .into_iter()
                                .map(|api_resource| {
                                    let Ok(json) = serde_json::to_string(&api_resource.resource)
                                    else {
                                        unreachable!()
                                    };

                                    let metadata = BTreeMap::from([("key".into(), json)]);

                                    LiteralItem::new(api_resource.to_string(), Some(metadata))
                                })
                                .collect();

                            widget.update_widget_item(Item::Array(items));
                        }
                        Err(e) => {
                            widget.update_widget_item(Item::Array(error_lines!(e)));
                        }
                    }
                }

                Resource(res) => match res {
                    Ok(list) => {
                        if list.items.is_empty() {
                            window.open_dialog(YAML_NOT_FOUND_DIALOG_ID);
                        } else {
                            window.open_dialog(YAML_NAME_DIALOG_ID);

                            let widget = window.find_widget_mut(YAML_NAME_DIALOG_ID);

                            let items = list
                                .items
                                .into_iter()
                                .map(
                                    |YamlResourceListItem {
                                         namespace,
                                         name,
                                         kind,
                                         value,
                                     }| {
                                        let Ok(json) = serde_json::to_string(&kind) else {
                                            unreachable!()
                                        };

                                        let metadata = BTreeMap::from([
                                            ("namespace".to_string(), namespace),
                                            ("name".to_string(), name),
                                            ("key".into(), json),
                                        ]);

                                        LiteralItem {
                                            metadata: Some(metadata),
                                            item: value,
                                        }
                                    },
                                )
                                .collect();

                            widget.update_widget_item(Item::Array(items));
                        }
                    }
                    Err(e) => {
                        let widget = window.find_widget_mut(YAML_NAME_DIALOG_ID);
                        widget.update_widget_item(Item::Array(error_lines!(e)));
                    }
                },
                Yaml(res) => {
                    update_widget_item_for_vec(window, YAML_WIDGET_ID, res);
                }
            }
        }

        Kube::Get(GetMessage::Response(GetResponse { kind, name, yaml })) => {
            let widget = window.find_widget_mut(YAML_DIALOG_ID).widget_base_mut();
            *(widget.append_title_mut()) = Some(format!(" : {}/{}", kind, name).into());

            update_widget_item_for_vec(window, YAML_DIALOG_ID, yaml);
        }

        Kube::Network(NetworkMessage::Response(ev)) => {
            use NetworkResponse::*;

            match ev {
                List(res) => update_widget_item_for_table(window, NETWORK_WIDGET_ID, res),
                Yaml(res) => {
                    update_widget_item_for_vec(window, NETWORK_DESCRIPTION_WIDGET_ID, res);
                }
            }
        }

        _ => unreachable!(),
    }
}
