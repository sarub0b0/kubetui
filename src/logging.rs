use log::LevelFilter;
use log4rs::{
    append::file::FileAppender,
    config::{Appender, Config, Root},
    encode::json::JsonEncoder,
};
use std::env;
use std::str::FromStr;

use once_cell::sync::OnceCell;

pub struct Logger;

pub static LOGGER_ENABLED: OnceCell<bool> = OnceCell::new();

#[macro_export]
macro_rules! logger {
    ($level:ident, $($arg:tt)+) => {
        if let Some(true) = crate::logging::LOGGER_ENABLED.get() {
            ::log::$level!($($arg)+);
        }
    };
}

impl Logger {
    pub fn init() -> Result<(), anyhow::Error> {
        let level_filter =
            LevelFilter::from_str(&env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()))?;

        let log_path = env::var("LOG_PATH").unwrap_or_else(|_| "output.log".to_string());

        let logfile = FileAppender::builder()
            .append(false)
            .encoder(Box::new(JsonEncoder::new()))
            .build(log_path)?;

        let config = Config::builder()
            .appender(Appender::builder().build("logfile", Box::new(logfile)))
            .build(Root::builder().appender("logfile").build(level_filter))?;

        log4rs::init_config(config)?;

        LOGGER_ENABLED.set(true).expect("Error: logger enable");

        Ok(())
    }
}
