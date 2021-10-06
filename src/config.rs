use clap::{crate_authors, crate_description, crate_name, crate_version, App, Arg};

use tui_wrapper::tui::layout::Direction;

#[derive(Debug, Default)]
pub struct Config {
    split_mode: DirectionWrapper,
}

impl Config {
    pub fn split_mode(&self) -> Direction {
        match self.split_mode {
            DirectionWrapper::Vertical => Direction::Vertical,
            DirectionWrapper::Horizontal => Direction::Horizontal,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum DirectionWrapper {
    Horizontal,
    Vertical,
}

impl Default for DirectionWrapper {
    fn default() -> Self {
        Self::Vertical
    }
}

pub fn configure() -> Config {
    let app = App::new(crate_name!())
        .author(crate_authors!())
        .version(crate_version!())
        .about(crate_description!())
        .arg(
            Arg::with_name("split-mode")
                .short("s")
                .long("split-mode")
                .help("Window split mode")
                .value_name("direction")
                .default_value("vertical")
                .possible_values(&["vertical", "v", "horizontal", "h"])
                .takes_value(true),
        )
        .get_matches();

    let mut config = Config::default();

    if let Some(d) = app.value_of("split-mode") {
        match d {
            "vertical" | "v" => {
                config.split_mode = DirectionWrapper::Vertical;
            }
            "horizontal" | "h" => {
                config.split_mode = DirectionWrapper::Horizontal;
            }
            _ => {}
        }
    }

    config
}
