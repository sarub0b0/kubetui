use log::LevelFilter;
use log4rs::{
    append::file::FileAppender,
    config::{Appender, Config as LConfig, Root},
    encode::pattern::PatternEncoder,
};
use std::env;
use std::str::FromStr;

pub fn logging() {
    let level_filter =
        LevelFilter::from_str(&env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()))
            .unwrap_or(LevelFilter::Info);

    let logfile = FileAppender::builder()
        .append(false)
        .encoder(Box::new(PatternEncoder::new("{h({l})} - {m}\n")))
        .build("log/output.log")
        .unwrap();

    let config = LConfig::builder()
        .appender(Appender::builder().build("logfile", Box::new(logfile)))
        .build(Root::builder().appender("logfile").build(level_filter))
        .unwrap();

    log4rs::init_config(config).unwrap();
}
