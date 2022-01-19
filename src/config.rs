use std::str::FromStr;

use clap::Parser;

use tui_wrapper::tui::layout::Direction;

#[derive(Parser, Debug)]
#[clap(author, version, about)]
pub struct Config {
    /// Window split mode
    #[clap(
        short,
        long,
        default_value = "vertical",
        possible_values = ["v", "h", "vertical", "horizontal"]
        )]
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
