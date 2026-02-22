use std::{collections::HashMap, thread, time};

use anyhow::Result;
use crossbeam::channel::{bounded, Receiver, Sender};

use crate::{
    cmd::Command,
    config::{
        theme::{PodColumnConfig, PodHighlightConfig},
        Config,
    },
    features::{
        api_resources::kube::ApiConfig,
        event::kube::EventConfig,
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

        kube_worker_config.event_config = EventConfig::from(config.theme.clone());
        kube_worker_config.api_config = ApiConfig::from(config.theme.clone());
        kube_worker_config.apis_config = ApisConfig::from(config.theme.clone());
        kube_worker_config.yaml_config = YamlConfig::from(config.theme.clone());

        let default_pod_columns = kube_worker_config.pod_config.default_columns.clone();

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
        .map(|hi| PodHighlightRule {
            status_regex: hi.status.clone(),
            style: hi.style.into(),
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
