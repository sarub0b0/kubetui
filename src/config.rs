use crate::event::kubernetes::KubeWorkerConfig;
use clap::{
    builder::{PossibleValuesParser, TypedValueParser},
    Parser, ValueEnum,
};
use ratatui::layout::Direction;
use std::{path::PathBuf, str::FromStr};

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Config {
    /// Window split mode
    #[arg(
        short,
        long,
        value_name = "v|h",
        display_order = 1000,
        value_parser = PossibleValuesParser::new(["v", "h", "vertical", "horizontal"]).map(|s| s.parse::<DirectionWrapper>().unwrap()),
        )]
    pub split_mode: Option<DirectionWrapper>,

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
}

impl Config {
    pub fn split_mode(&self) -> Direction {
        match self.split_mode {
            Some(d) => match d {
                DirectionWrapper::Vertical => Direction::Vertical,
                DirectionWrapper::Horizontal => Direction::Horizontal,
            },
            None => Direction::Vertical,
        }
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
        }
    }
}

#[derive(Debug, ValueEnum, Clone, Copy, PartialEq, Eq)]
pub enum AllNamespaces {
    True,
    False,
}

impl From<AllNamespaces> for bool {
    fn from(e: AllNamespaces) -> Self {
        match e {
            AllNamespaces::True => true,
            AllNamespaces::False => false,
        }
    }
}

impl std::fmt::Display for AllNamespaces {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_possible_value()
            .expect("no values are skipped")
            .get_name()
            .fmt(f)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum DirectionWrapper {
    Horizontal,
    Vertical,
}

impl Default for DirectionWrapper {
    fn default() -> Self {
        Self::Vertical
    }
}

impl From<DirectionWrapper> for Direction {
    fn from(d: DirectionWrapper) -> Self {
        match d {
            DirectionWrapper::Vertical => Direction::Vertical,
            DirectionWrapper::Horizontal => Direction::Horizontal,
        }
    }
}

impl FromStr for DirectionWrapper {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "vertical" | "v" => Ok(DirectionWrapper::Vertical),
            "horizontal" | "h" => Ok(DirectionWrapper::Horizontal),
            _ => Err("invalid value"),
        }
    }
}

pub fn configure() -> Config {
    Config::parse()

    // let cmd = Command::new("kubetui")
    //     .arg(
    //         Arg::new("all-namespaces")
    //             .short('A')
    //             .long("all-namespaces")
    //             .value_name("true|false")
    //             .value_parser(value_parser!(bool))
    //             .num_args(0..=1)
    //             .require_equals(true)
    //             .default_missing_value("true"),
    //     )
    //     .get_matches();
    //
    // dbg!(cmd.get_one::<bool>("all-namespaces"));
    // dbg!(Config::parse());
    //
    // std::process::exit(0);
}

#[cfg(test)]
mod tests {
    use super::*;

    mod split_mode {
        use clap::error::ErrorKind;
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn possible_valuesの値であるhを設定したときhorizontalを返す() {
            let parse = Config::try_parse_from(["kubetui", "-s", "h"]).unwrap();
            assert_eq!(parse.split_mode(), Direction::Horizontal)
        }

        #[test]
        fn possible_valuesの値にない値を設定したときerrを返す() {
            let parse = Config::try_parse_from(["kubetui", "-s", "hoge"]);
            assert_eq!(parse.unwrap_err().kind(), ErrorKind::InvalidValue)
        }
    }

    mod namespace {
        use clap::error::ErrorKind;
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[test]
        fn 値を設定しないとエラーを返す() {
            let parse = Config::try_parse_from(["kubetui", "-n"]);
            assert_eq!(parse.unwrap_err().kind(), ErrorKind::InvalidValue)
        }

        #[test]
        fn namespaceを1つ指定したときvecを返す() {
            let parse = Config::try_parse_from(["kubetui", "-n", "hoge"]).unwrap();
            assert_eq!(parse.namespaces, Some(vec!["hoge".to_string()]))
        }

        #[rstest]
        #[case::multiple_occurrences(&["kubetui", "-n", "foo", "-n", "bar", "-n", "zoo"])]
        #[case::delimiter(&["kubetui", "-n", "foo,bar,zoo"])]
        #[case::mixed(&["kubetui", "-n", "foo,bar", "-n", "zoo"])]
        fn namespaceを複数指定したときvecを返す(#[case] iter: &[&str]) {
            let parse = Config::try_parse_from(iter).unwrap();
            assert_eq!(
                parse.namespaces,
                Some(vec![
                    "foo".to_string(),
                    "bar".to_string(),
                    "zoo".to_string()
                ])
            )
        }

        #[test]
        fn all_namespacesと併用するとエラーを返す() {
            let parse = Config::try_parse_from(["kubetui", "-A", "-n", "hoge"]);
            assert_eq!(parse.unwrap_err().kind(), ErrorKind::ArgumentConflict)
        }
    }
    mod all_namespace {
        use clap::error::ErrorKind;
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[test]
        fn equalがない構文のときエラーになる() {
            let parse = Config::try_parse_from(["kubetui", "--all-namespaces", "true"]);
            assert_eq!(parse.unwrap_err().kind(), ErrorKind::UnknownArgument)
        }

        #[rstest]
        #[case::is_true(AllNamespaces::True)]
        #[case::is_false(AllNamespaces::False)]
        fn 設定した値になる(#[case] value: AllNamespaces) {
            let parse = Config::try_parse_from(["kubetui", &format!("--all-namespaces={}", value)])
                .unwrap();
            assert_eq!(parse.all_namespaces, value)
        }

        #[test]
        fn 値が設定されていないときtrueを設定する() {
            let parse = Config::try_parse_from(["kubetui", "-A"]).unwrap();
            assert_eq!(parse.all_namespaces, AllNamespaces::True)
        }

        #[test]
        fn namespaceと併用するとエラーを返す() {
            let parse = Config::try_parse_from(["kubetui", "-A", "-n", "hoge"]);
            assert_eq!(parse.unwrap_err().kind(), ErrorKind::ArgumentConflict)
        }
    }
}
