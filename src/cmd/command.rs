use anyhow::Result;
use clap::Parser;
use ratatui::layout::Direction;
use std::path::PathBuf;

use crate::{config::ConfigLoadOption, features::pod::PodColumns, workers::kube::KubeWorkerConfig};

use super::{
    args::{parse_pod_columns, AllNamespaces, ClipboardMode, SplitDirection},
    SubCommand,
};

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None, disable_help_subcommand = true)]
pub struct Command {
    /// Window split direction
    #[arg(
        short,
        long,
        value_name = "v|h",
        default_value = "v",
        display_order = 1000
    )]
    pub split_direction: SplitDirection,

    /// Namespaces (e.g. -n val1,val2,val3 | -n val1 -n val2 -n val3)
    #[arg(
        short,
        long,
        conflicts_with = "all_namespaces",
        value_delimiter = ',',
        display_order = 1000
    )]
    pub namespaces: Option<Vec<String>>,

    /// Context
    #[arg(short, long, display_order = 1000)]
    pub context: Option<String>,

    /// Select all namespaces
    //
    // bool型だと下記エラーが出てうまく行かないため、専用のenumを定義して対処する
    // boolで行ける方法が分かり次第修正する。
    //
    // ```sh
    // thread 'main' panicked at 'assertion failed: `(left == right)`
    //   left: `true`,
    //  right: `false`: Argument all_namespaces: mismatch between `num_args` (0..=1) and `takes_value`', /Users/kohashimoto/.cargo/registry/src/github.com-1ecc6299db9ec823/clap-4.0.11/src/builder/debug_asserts.rs:769:5
    // note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
    // ```
    #[arg(
        short = 'A',
        long,
        value_name = "true|false",
        num_args = 0..=1,
        require_equals = true,
        default_value_t = AllNamespaces::False,
        default_missing_value = "true",
        hide_possible_values = true,
        value_enum,
        display_order = 1000
    )]
    pub all_namespaces: AllNamespaces,

    /// kubeconfig path
    #[arg(short = 'C', long, display_order = 1000)]
    pub kubeconfig: Option<PathBuf>,

    /// Logging
    #[arg(short = 'l', long, display_order = 1000)]
    pub logging: bool,

    /// Config file path
    #[arg(long, display_order = 1000)]
    pub config_file: Option<PathBuf>,

    /// Comma-separated list of columns to show in pod table (e.g. name,status,ip). Use "full" to show all available columns.
    #[arg(
        long,
        value_parser = parse_pod_columns,
        display_order = 1000)]
    pub pod_columns: Option<PodColumns>,

    /// Preset name for pod columns (e.g. "default", "full"). If both are specified, `--pod-columns` overrides this.
    #[arg(long, display_order = 1000)]
    pub pod_columns_preset: Option<String>,

    /// Clipboard mode (auto, system, or osc52)
    #[arg(
        long,
        value_name = "auto|system|osc52",
        default_value = "auto",
        env = "KUBETUI_CLIPBOARD",
        value_enum,
        display_order = 1000
    )]
    pub clipboard: ClipboardMode,

    #[command(subcommand)]
    pub subcommand: Option<SubCommand>,
}

impl Command {
    pub fn init() -> Self {
        Self::parse()
    }

    pub fn split_direction(&self) -> Direction {
        self.split_direction.to_direction()
    }

    pub fn kube_worker_config(&self) -> KubeWorkerConfig {
        let Self {
            namespaces,
            context,
            all_namespaces,
            kubeconfig,
            ..
        } = self.clone();

        KubeWorkerConfig {
            kubeconfig,
            target_namespaces: namespaces,
            context,
            all_namespaces: all_namespaces.into(),
            ..Default::default()
        }
    }

    pub fn config_load_option(&self) -> Result<ConfigLoadOption> {
        let option = if let Some(path) = &self.config_file {
            match path.try_exists() {
                Ok(true) => ConfigLoadOption::Path(path.clone()),
                Ok(false) => {
                    eprintln!("Config file not found: {:?}", path);

                    ConfigLoadOption::Default
                }
                Err(err) => {
                    eprintln!("Failed to check config file exists: {}", err);

                    ConfigLoadOption::Default
                }
            }
        } else {
            let path = xdg_config_home().join("config.yaml");

            match path.try_exists() {
                Ok(true) => ConfigLoadOption::Path(path.clone()),
                Ok(false) => ConfigLoadOption::Default,
                Err(err) => {
                    eprintln!("Failed to check config file exists: {}", err);

                    ConfigLoadOption::Default
                }
            }
        };

        Ok(option)
    }
}

fn xdg_config_home() -> PathBuf {
    match std::env::var_os("XDG_CONFIG_HOME").map(|s| PathBuf::from(s).join("kubetui")) {
        Some(path) => path,
        None => dirs::home_dir()
            .expect("Failed to get home directory")
            .join(".config")
            .join("kubetui"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod split_direction {
        use clap::error::ErrorKind;
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn possible_valuesの値であるhを設定したときhorizontalを返す() {
            let cmd = Command::try_parse_from(["kubetui", "-s", "h"]).unwrap();
            assert_eq!(cmd.split_direction(), Direction::Horizontal)
        }

        #[test]
        fn possible_valuesの値にない値を設定したときerrを返す() {
            let cmd = Command::try_parse_from(["kubetui", "-s", "hoge"]);
            assert_eq!(cmd.unwrap_err().kind(), ErrorKind::ValueValidation)
        }
    }

    mod namespace {
        use clap::error::ErrorKind;
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[test]
        fn 値を設定しないとエラーを返す() {
            let cmd = Command::try_parse_from(["kubetui", "-n"]);
            assert_eq!(cmd.unwrap_err().kind(), ErrorKind::InvalidValue)
        }

        #[test]
        fn namespaceを1つ指定したときvecを返す() {
            let cmd = Command::try_parse_from(["kubetui", "-n", "hoge"]).unwrap();
            assert_eq!(cmd.namespaces, Some(vec!["hoge".to_string()]))
        }

        #[rstest]
        #[case::multiple_occurrences(&["kubetui", "-n", "foo", "-n", "bar", "-n", "zoo"])]
        #[case::delimiter(&["kubetui", "-n", "foo,bar,zoo"])]
        #[case::mixed(&["kubetui", "-n", "foo,bar", "-n", "zoo"])]
        fn namespaceを複数指定したときvecを返す(#[case] iter: &[&str]) {
            let cmd = Command::try_parse_from(iter).unwrap();
            assert_eq!(
                cmd.namespaces,
                Some(vec![
                    "foo".to_string(),
                    "bar".to_string(),
                    "zoo".to_string()
                ])
            )
        }

        #[test]
        fn all_namespacesと併用するとエラーを返す() {
            let cmd = Command::try_parse_from(["kubetui", "-A", "-n", "hoge"]);
            assert_eq!(cmd.unwrap_err().kind(), ErrorKind::ArgumentConflict)
        }
    }
    mod all_namespace {
        use clap::error::ErrorKind;
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[test]
        fn equalがない構文のときエラーになる() {
            let cmd = Command::try_parse_from(["kubetui", "--all-namespaces", "true"]);
            assert_eq!(cmd.is_err(), true)
        }

        #[rstest]
        #[case::is_true(AllNamespaces::True)]
        #[case::is_false(AllNamespaces::False)]
        fn 設定した値になる(#[case] value: AllNamespaces) {
            let cmd = Command::try_parse_from(["kubetui", &format!("--all-namespaces={}", value)])
                .unwrap();
            assert_eq!(cmd.all_namespaces, value)
        }

        #[test]
        fn 値が設定されていないときtrueを設定する() {
            let cmd = Command::try_parse_from(["kubetui", "-A"]).unwrap();
            assert_eq!(cmd.all_namespaces, AllNamespaces::True)
        }

        #[test]
        fn namespaceと併用するとエラーを返す() {
            let cmd = Command::try_parse_from(["kubetui", "-A", "-n", "hoge"]);
            assert_eq!(cmd.unwrap_err().kind(), ErrorKind::ArgumentConflict)
        }
    }

    mod pod_columns {
        use pretty_assertions::assert_eq;

        use crate::features::pod::PodColumn;

        use super::*;

        #[test]
        fn デフォルトのカラムを設定する() {
            let cmd = Command::try_parse_from(["kubetui"]).unwrap();
            assert_eq!(cmd.pod_columns, None);
        }

        #[test]
        fn フルを設定すると全カラムを設定する() {
            let cmd = Command::try_parse_from(["kubetui", "--pod-columns=full"]).unwrap();
            assert_eq!(
                cmd.pod_columns,
                Some(PodColumns::new([
                    PodColumn::Name,
                    PodColumn::Ready,
                    PodColumn::Status,
                    PodColumn::Restarts,
                    PodColumn::Age,
                    PodColumn::IP,
                    PodColumn::Node,
                    PodColumn::NominatedNode,
                    PodColumn::ReadinessGates
                ]))
            );
        }

        #[test]
        fn カンマ区切りでカラムを指定できる() {
            let cmd =
                Command::try_parse_from(["kubetui", "--pod-columns=name,ready,status"]).unwrap();
            assert_eq!(
                cmd.pod_columns,
                Some(PodColumns::new([
                    PodColumn::Name,
                    PodColumn::Ready,
                    PodColumn::Status
                ]))
            );
        }
    }
}
