mod action;
mod ansi;
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

use anyhow::{Context, Result};

use crossbeam::channel::{bounded, Receiver, Sender};

use crossterm::{
    cursor::Show,
    event::{DisableFocusChange, DisableMouseCapture, EnableFocusChange, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use crate::cmd::Command;

use logging::Logger;
use message::Message;
use signal::signal_handler;
use workers::{KubeWorker, Render, Tick, UserInput};

use std::{
    io, panic,
    sync::{atomic::AtomicBool, Arc},
    thread, time,
};

macro_rules! enable_raw_mode {
    () => {
        enable_raw_mode().expect("failed to enable raw mode");
        execute!(
            io::stdout(),
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
            io::stdout(),
            LeaveAlternateScreen,
            DisableMouseCapture,
            DisableFocusChange,
            Show
        )
        .expect("failed to restore terminal");
        disable_raw_mode().expect("failed to disable raw mode");
    };
}

fn run(config: Command) -> Result<()> {
    let split_direction = config.split_direction();
    let kube_worker_config = config.kube_worker_config();

    let (tx_input, rx_main): (Sender<Message>, Receiver<Message>) = bounded(128);
    let (tx_main, rx_kube): (Sender<Message>, Receiver<Message>) = bounded(256);
    let tx_kube = tx_input.clone();
    let tx_tick = tx_input.clone();

    let is_terminated = Arc::new(AtomicBool::new(false));

    let user_input = UserInput::new(tx_input.clone(), is_terminated.clone());

    let kube = KubeWorker::new(
        tx_kube.clone(),
        rx_kube.clone(),
        is_terminated.clone(),
        kube_worker_config,
    );

    let tick = Tick::new(
        tx_tick.clone(),
        time::Duration::from_millis(200),
        is_terminated.clone(),
    );

    let render = Render::new(
        tx_main.clone(),
        rx_main.clone(),
        is_terminated.clone(),
        split_direction,
    );

    thread::scope(|s| {
        let kube_handler = s.spawn(|| {
            kube.set_panic_hook();
            kube.start()
        });

        let tick_handler = s.spawn(move || {
            tick.set_panic_hook();
            tick.start()
        });

        let user_input_handler = s.spawn(move || {
            user_input.set_panic_hook();
            user_input.start()
        });

        let render_handler = s.spawn(move || {
            render.set_panic_hook();
            render.start()
        });

        kube_handler
            .join()
            .expect("kube thread panicked")
            .context("kube thread error")?;

        tick_handler
            .join()
            .expect("tick thread panicked")
            .context("tick thread error")?;

        user_input_handler
            .join()
            .expect("user_input thread panicked")
            .context("user_input thread error")?;

        render_handler
            .join()
            .expect("render thread panicked")
            .context("render thread error")?;

        anyhow::Ok(())
    })?;

    // SendErrorを防ぐためrx_mainのdropを遅らせる
    drop(rx_main);

    Ok(())
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

    let result = run(command);

    disable_raw_mode!();

    result

    // Ok(())
}
