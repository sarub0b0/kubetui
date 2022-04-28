use std::{path::PathBuf, str::FromStr};

use clap::Parser;

use tui::layout::Direction;

use crate::event::kubernetes::kube_worker::KubeWorkerConfig;

#[derive(Parser, Debug, Clone)]
#[clap(author, version, about)]
pub struct Config {
    /// Window split mode
    #[clap(
        short,
        long,
        name = "v|h",
        possible_values = ["v", "h", "vertical", "horizontal"],
        display_order = 1000,
        )]
    pub split_mode: Option<DirectionWrapper>,

    /// Namespaces (e.g. -n val1,val2,val3 | -n val1 -n val2 -n val3)
    #[clap(
        short,
        long,
        conflicts_with = "all-namespaces",
        value_delimiter = ',',
        display_order = 1000
    )]
    pub namespaces: Option<Vec<String>>,

    /// Context
    #[clap(short, long, display_order = 1000)]
    pub context: Option<String>,

    /// Select all namespaces
    #[clap(
        short = 'A',
        long,
        parse(try_from_str),
        default_value_t = false,
        value_name = "[true|false]",
        possible_values = ["false", "true"],
        min_values = 0,
        require_equals = true,
        default_missing_value = "true",
        hide_default_value = true,
        hide_possible_values = true,
        display_order = 1000,
        )]
    pub all_namespaces: bool,

    /// kubeconfig path
    #[clap(short = 'C', long, display_order = 1000)]
    pub kubeconfig: Option<PathBuf>,
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
            namespaces,
            context,
            all_namespaces,
        }
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
}

#[cfg(test)]
mod tests {
    use super::*;

    mod split_mode {
        use clap::ErrorKind;
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
        use clap::ErrorKind;
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[test]
        fn 値を設定しないとエラーを返す() {
            let parse = Config::try_parse_from(["kubetui", "-n"]);
            assert_eq!(parse.unwrap_err().kind(), ErrorKind::EmptyValue)
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
        use clap::ErrorKind;
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[test]
        fn equalがない構文のときエラーになる() {
            let parse = Config::try_parse_from(["kubetui", "--all-namespaces", "true"]);
            assert_eq!(parse.unwrap_err().kind(), ErrorKind::UnknownArgument)
        }

        #[rstest]
        #[case::is_true(true)]
        #[case::is_false(false)]
        fn 設定した値になる(#[case] value: bool) {
            let parse = Config::try_parse_from(["kubetui", &format!("--all-namespaces={}", value)])
                .unwrap();
            assert_eq!(parse.all_namespaces, value)
        }

        #[test]
        fn 値が設定されていないときtrueを設定する() {
            let parse = Config::try_parse_from(["kubetui", "-A"]).unwrap();
            assert_eq!(parse.all_namespaces, true)
        }

        #[test]
        fn namespaceと併用するとエラーを返す() {
            let parse = Config::try_parse_from(["kubetui", "-A", "-n", "hoge"]);
            assert_eq!(parse.unwrap_err().kind(), ErrorKind::ArgumentConflict)
        }
    }
}
