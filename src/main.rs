mod action;
mod ansi;
mod app;
mod clipboard;
mod cmd;
mod context;
mod error;
mod logging;
mod message;
mod signal;
mod ui;
mod window;
mod workers;

use anyhow::Result;

use crossterm::{
    cursor::Show,
    event::{DisableFocusChange, DisableMouseCapture, EnableFocusChange, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use crate::{app::App, cmd::Command};

use logging::Logger;
use signal::signal_handler;

use std::panic;

macro_rules! enable_raw_mode {
    () => {
        enable_raw_mode().expect("failed to enable raw mode");
        execute!(
            std::io::stdout(),
            EnterAlternateScreen,
            EnableMouseCapture,
            EnableFocusChange
        )
        .expect("failed to enable raw mode");
    };
}

macro_rules! disable_raw_mode {
    () => {
        execute!(
            std::io::stdout(),
            LeaveAlternateScreen,
            DisableMouseCapture,
            DisableFocusChange,
            Show
        )
        .expect("failed to restore terminal");
        disable_raw_mode().expect("failed to disable raw mode");
    };
}

fn main() -> Result<()> {
    signal_handler();

    let default_hook = panic::take_hook();

    panic::set_hook(Box::new(move |info| {
        disable_raw_mode!();

        eprintln!("\x1b[31mPanic! disable raw mode\x1b[39m");

        default_hook(info);
    }));

    let command = Command::init();

    if command.logging {
        Logger::init()?;
    }

    enable_raw_mode!();

    let result = App::run(command);

    disable_raw_mode!();

    result
}
