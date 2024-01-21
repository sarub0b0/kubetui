mod action;
mod ansi;
mod clipboard;
mod cmd;
mod context;
mod error;
mod event;
mod logging;
mod signal;
mod ui;
mod window;
mod workers;

use anyhow::Result;

use crossbeam::channel::{bounded, Receiver, Sender};

use crossterm::{
    cursor::Show,
    event::{DisableFocusChange, DisableMouseCapture, EnableFocusChange, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use crate::cmd::Command;

use action::{update_contents, window_action};
use context::{Context, Namespace};
use event::Event;
use logging::Logger;
use signal::signal_handler;
use ui::WindowEvent;
use window::WindowInit;
use workers::{KubeWorker, Render, Tick, UserInput};

use std::{
    cell::RefCell,
    io, panic,
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread, time,
};

use ratatui::{backend::CrosstermBackend, Terminal, TerminalOptions, Viewport};

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
    let split_mode = config.split_direction();
    let kube_worker_config = config.kube_worker_config();

    let (tx_input, rx_main): (Sender<Event>, Receiver<Event>) = bounded(128);
    let (tx_main, rx_kube): (Sender<Event>, Receiver<Event>) = bounded(256);
    let tx_kube = tx_input.clone();
    let tx_tick = tx_input.clone();

    let is_terminated = Arc::new(AtomicBool::new(false));

    let user_input = UserInput::new(tx_input.clone(), is_terminated.clone());

    let user_input_handler = user_input.start();

    let is_terminated_clone = is_terminated.clone();

    let kube = KubeWorker::new(tx_kube, rx_kube, is_terminated_clone, kube_worker_config);
    let kube_handler = kube.start();

    let tick = Tick::new(
        tx_tick.clone(),
        time::Duration::from_millis(200),
        is_terminated.clone(),
    );

    let tick_handler = tick.start();

    let render = Render::new(
        tx_main.clone(),
        rx_main.clone(),
        is_terminated.clone(),
        split_mode,
    );

    let render_handler = render.start();

    match render_handler.join() {
        Ok(ret) => ret?,
        Err(e) => {
            if let Some(e) = e.downcast_ref::<&str>() {
                panic!("render thread panicked: {:?}", e);
            };
        }
    }

    match user_input_handler.join() {
        Ok(ret) => ret?,
        Err(e) => {
            if let Some(e) = e.downcast_ref::<&str>() {
                panic!("read_key thread panicked: {:?}", e);
            };
        }
    }

    match kube_handler.join() {
        Ok(ret) => ret?,
        Err(e) => {
            if let Some(e) = e.downcast_ref::<&str>() {
                panic!("kube_process thread panicked: {:?}", e);
            };
        }
    }

    match tick_handler.join() {
        Ok(ret) => ret?,
        Err(e) => {
            if let Some(e) = e.downcast_ref::<&str>() {
                panic!("tick thread panicked: {:?}", e);
            };
        }
    }

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
