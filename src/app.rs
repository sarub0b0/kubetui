use std::{collections::HashMap, str::FromStr as _, thread, time};

use anyhow::Result;
use crossbeam::channel::{bounded, Receiver, Sender};

use crate::{
    cmd::Command,
    config::{
        theme::{LabelColumnConfig, PodColumnConfig, PodHighlightConfig},
        Config,
    },
    features::{
        api_resources::kube::ApiConfig,
        event::kube::EventConfig,
        node::{NodeColumn, NodeColumnSpec, NodeColumns, NodeLabelColumn},
        pod::{kube::PodHighlightRule, PodColumns},
    },
    logger,
    message::Message,
    workers::{kube::YamlConfig, ApisConfig, KubeWorker, Render, Tick, UserInput},
};

pub struct App;

impl App {
    pub fn run(cmd: Command, config: Config) -> Result<()> {
        let split_direction = cmd.split_direction();
        let mut kube_worker_config = cmd.kube_worker_config();

        let (tx_input, rx_main): (Sender<Message>, Receiver<Message>) = bounded(128);
        let (tx_main, rx_kube): (Sender<Message>, Receiver<Message>) = bounded(256);
        let tx_kube = tx_input.clone();
        let tx_tick = tx_input.clone();

        let (tx_shutdown, rx_shutdown) = bounded::<Result<()>>(1);

        let user_input = UserInput::new(tx_input.clone(), tx_shutdown.clone());

        kube_worker_config.pod_config.pod_highlight_rules =
            build_pod_highlight_rules(&config.theme.pod.highlights);

        kube_worker_config.pod_config.default_columns = build_pod_columns(
            cmd.pod_columns,
            cmd.pod_columns_preset,
            &config.theme.pod.default_preset,
            &config.theme.pod.column_presets,
        )?;

        kube_worker_config.node_config.default_columns = build_node_columns(
            cmd.node_columns,
            cmd.node_columns_preset,
            &config.theme.node.default_preset,
            &config.theme.node.column_presets,
            &config.theme.node.label_columns,
        )?;

        kube_worker_config.event_config = EventConfig::from(config.theme.clone());
        kube_worker_config.api_config = ApiConfig::from(config.theme.clone());
        kube_worker_config.apis_config = ApisConfig::from(config.theme.clone());
        kube_worker_config.yaml_config = YamlConfig::from(config.theme.clone());

        kube_worker_config.fallback_namespaces =
            config.fallback_namespaces.and_then(|namespaces| {
                let mut seen = std::collections::HashSet::new();
                let deduped: Vec<String> = namespaces
                    .into_iter()
                    .filter(|ns| seen.insert(ns.clone()))
                    .collect();
                if deduped.is_empty() {
                    None
                } else {
                    Some(deduped)
                }
            });

        let default_pod_columns = kube_worker_config.pod_config.default_columns.clone();
        let default_node_columns = kube_worker_config.node_config.default_columns.clone();
        let node_label_columns = build_node_label_registry(&config.theme.node.label_columns)?;

        let kube = KubeWorker::new(
            tx_kube.clone(),
            rx_kube.clone(),
            tx_shutdown.clone(),
            kube_worker_config,
        );

        let tick = Tick::new(
            tx_tick.clone(),
            time::Duration::from_millis(200),
            tx_shutdown.clone(),
        );

        let render = Render::new(
            tx_main.clone(),
            rx_main.clone(),
            tx_shutdown.clone(),
            split_direction,
            default_pod_columns,
            default_node_columns,
            node_label_columns,
            config.theme.clone(),
            cmd.clipboard,
            config.logging.max_lines,
        );

        logger!(info, "app start");

        thread::spawn(|| {
            kube.set_panic_hook();
            kube.start();
        });

        thread::spawn(move || {
            tick.set_panic_hook();
            tick.start();
        });

        thread::spawn(move || {
            user_input.set_panic_hook();
            user_input.start();
        });

        thread::spawn(move || {
            render.set_panic_hook();
            render.start();
        });

        let result = rx_shutdown.recv()?;

        logger!(info, "app end");

        result
    }
}

fn build_pod_highlight_rules(highlights: &[PodHighlightConfig]) -> Vec<PodHighlightRule> {
    highlights
        .iter()
        .map(|hi| {
            PodHighlightRule {
                status_regex: hi.status.clone(),
                style: hi.style.into(),
            }
        })
        .collect()
}

/// ### Pod カラムの設定決定フロー（優先順位つき）
///
/// CLI > Preset > Config Default > Built-in Default という優先順位。
///
/// 1. **CLI引数 `--pod-columns` が指定されている場合**
///
///    * `parse_pod_columns(...)` を呼び出して解析。
///    * 結果を `PodColumns` に変換して使用。
///    * 他の設定（preset/config）は無視される。
///
/// 2. **CLI引数 `--pod-columns-preset` が指定されている場合**
///
///    * 設定ファイルの `pod.column_presets` から該当プリセット名を検索。
///    * 見つからなければエラー（stderr に表示して終了）。
///    * プリセットに定義されたカラムリストを `PodColumns` に変換して使用。
///
/// 3. **設定ファイルに `pod.default_preset` が定義されている場合**
///
///    * `pod.column_presets` に該当プリセットが存在するか確認。
///    * 存在しない場合はエラー（stderr に表示して終了）。
///    * 定義されていればそのプリセットを使って `PodColumns` を構築。
///
/// 4. **上記いずれも指定されていない場合**
///
///    * None を返す。
///
fn build_pod_columns(
    cmd_pod_columns: Option<PodColumns>,
    cmd_pod_columns_preset: Option<String>,
    default_preset: &Option<String>,
    column_presets: &Option<HashMap<String, Vec<PodColumnConfig>>>,
) -> Result<Option<PodColumns>> {
    if let Some(columns) = cmd_pod_columns {
        return Ok(Some(columns));
    }

    if let Some(preset) = cmd_pod_columns_preset {
        let Some(presets) = column_presets else {
            anyhow::bail!("No pod column presets defined in config file, but '--pod-columns-preset' flag was used");
        };

        let Some(columns) = presets.get(&preset) else {
            anyhow::bail!("Pod column preset '{}' was specified via '--pod-columns-preset' but is not defined in config file", preset);
        };

        return Ok(Some(convert_columns(columns)));
    }

    if let Some(default_preset) = default_preset {
        let Some(presets) = column_presets else {
            anyhow::bail!(
                "No pod column presets defined in config file, but 'default_preset' is set"
            );
        };

        let Some(columns) = presets.get(default_preset) else {
            anyhow::bail!(
                "Default pod columns preset '{}' is set in config file but not defined in column_presets",
                default_preset
            );
        };

        return Ok(Some(convert_columns(columns)));
    }

    Ok(None)
}

fn convert_columns(columns: &[PodColumnConfig]) -> PodColumns {
    PodColumns::from(columns)
        .ensure_name_column()
        .dedup_columns()
}

fn build_node_columns(
    cmd_node_columns: Option<Vec<String>>,
    cmd_node_columns_preset: Option<String>,
    default_preset: &Option<String>,
    column_presets: &Option<HashMap<String, Vec<String>>>,
    label_columns: &Option<Vec<LabelColumnConfig>>,
) -> Result<Option<NodeColumns>> {
    let registry = build_node_label_registry(label_columns)?;

    if let Some(names) = cmd_node_columns {
        return Ok(Some(resolve_columns(&names, &registry)?));
    }

    let Some(preset_name) = cmd_node_columns_preset.as_ref().or(default_preset.as_ref()) else {
        return Ok(None);
    };

    let Some(presets) = column_presets else {
        anyhow::bail!(
            "No node column presets defined in config file, but preset '{}' was requested",
            preset_name
        );
    };
    let Some(entries) = presets.get(preset_name) else {
        anyhow::bail!(
            "Node column preset '{}' is not defined in column_presets",
            preset_name
        );
    };

    Ok(Some(resolve_columns(entries, &registry)?))
}

/// Build the label-column registry from config, erroring on builtin name collisions.
fn build_node_label_registry(
    label_columns: &Option<Vec<LabelColumnConfig>>,
) -> Result<Vec<NodeLabelColumn>> {
    let mut out = Vec::new();
    if let Some(defs) = label_columns {
        for def in defs {
            let norm = NodeColumn::normalize_column(&def.name);
            if NodeColumn::from_str(&norm).is_ok() {
                anyhow::bail!(
                    "label_columns name '{}' collides with a builtin column name",
                    def.name
                );
            }
            out.push(NodeLabelColumn {
                name: def.name.clone(),
                key: def.label.clone(),
                header: def.name.to_uppercase(),
            });
        }
    }
    Ok(out)
}

/// Resolve column names (builtin or registry label, or "full") into NodeColumns.
fn resolve_columns(names: &[String], registry: &[NodeLabelColumn]) -> Result<NodeColumns> {
    if names.len() == 1 && NodeColumn::normalize_column(&names[0]) == "full" {
        return Ok(NodeColumns::from_builtins(NodeColumn::all()));
    }

    let mut specs = Vec::new();
    for name in names {
        let norm = NodeColumn::normalize_column(name);
        if let Ok(builtin) = NodeColumn::from_str(&norm) {
            specs.push(NodeColumnSpec::Builtin(builtin));
        } else if let Some(lc) = registry
            .iter()
            .find(|lc| NodeColumn::normalize_column(&lc.name) == norm)
        {
            specs.push(NodeColumnSpec::Label {
                key: lc.key.clone(),
                header: lc.header.clone(),
            });
        } else {
            anyhow::bail!(
                "Column '{}' is neither a builtin column nor a defined label column",
                name
            );
        }
    }

    Ok(NodeColumns::new(specs).ensure_name_column().dedup_columns())
}

#[cfg(test)]
mod node_columns_tests {
    use super::*;
    use crate::features::node::{NodeColumn, NodeColumnSpec};

    fn presets() -> HashMap<String, Vec<String>> {
        HashMap::from([(
            "gpu".to_string(),
            vec!["name".to_string(), "mig".to_string(), "status".to_string()],
        )])
    }

    fn labels() -> Vec<LabelColumnConfig> {
        vec![LabelColumnConfig {
            name: "mig".to_string(),
            label: "nvidia.com/mig.config.state".to_string(),
        }]
    }

    #[test]
    fn resolves_preset_with_builtin_and_label_interleaved() {
        let cols = build_node_columns(
            None,
            None,
            &Some("gpu".to_string()),
            &Some(presets()),
            &Some(labels()),
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            cols.specs(),
            &[
                NodeColumnSpec::Builtin(NodeColumn::Name),
                NodeColumnSpec::Label {
                    key: "nvidia.com/mig.config.state".to_string(),
                    header: "MIG".to_string()
                },
                NodeColumnSpec::Builtin(NodeColumn::Status),
            ]
        );
    }

    #[test]
    fn cli_names_resolve_labels_and_take_precedence() {
        let cols = build_node_columns(
            Some(vec!["name".to_string(), "mig".to_string()]),
            Some("gpu".to_string()),
            &Some("gpu".to_string()),
            &Some(presets()),
            &Some(labels()),
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            cols.specs(),
            &[
                NodeColumnSpec::Builtin(NodeColumn::Name),
                NodeColumnSpec::Label {
                    key: "nvidia.com/mig.config.state".to_string(),
                    header: "MIG".to_string()
                },
            ]
        );
    }

    #[test]
    fn none_when_no_preset() {
        let actual = build_node_columns(None, None, &None, &None, &None).unwrap();
        assert!(actual.is_none());
    }

    #[test]
    fn error_on_unknown_name() {
        let presets = HashMap::from([(
            "p".to_string(),
            vec!["name".to_string(), "bogus".to_string()],
        )]);
        assert!(
            build_node_columns(None, None, &Some("p".to_string()), &Some(presets), &None).is_err()
        );
    }

    #[test]
    fn error_on_label_name_colliding_with_builtin() {
        let labels = vec![LabelColumnConfig {
            name: "status".to_string(),
            label: "x".to_string(),
        }];
        let presets = HashMap::from([("p".to_string(), vec!["name".to_string()])]);
        assert!(build_node_columns(
            None,
            None,
            &Some("p".to_string()),
            &Some(presets),
            &Some(labels)
        )
        .is_err());
    }

    #[test]
    fn full_returns_all_builtins() {
        let cols = build_node_columns(Some(vec!["full".to_string()]), None, &None, &None, &None)
            .unwrap()
            .unwrap();
        assert_eq!(cols.specs().len(), NodeColumn::all().count());
    }
}
