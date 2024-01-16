mod action;
mod ansi;
mod clipboard_wrapper;
mod config;
mod context;
mod error;
mod event;
mod logging;
mod signal;
mod ui;
mod window;

use anyhow::Result;

use crossbeam::channel::{bounded, Receiver, Sender};

use crossterm::{
    cursor::Show,
    event::{DisableFocusChange, DisableMouseCapture, EnableFocusChange, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use self::{
    action::{update_contents, window_action},
    config::{configure, Config},
    context::{Context, Namespace},
    event::{input::read_key, kubernetes::KubeWorker, tick::tick, Event},
    logging::Logger,
    signal::signal_handler,
    ui::WindowEvent,
    window::WindowInit,
};

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

fn run(config: Config) -> Result<()> {
    let split_mode = config.split_mode();
    let kube_worker_config = config.kube_worker_config();

    let (tx_input, rx_main): (Sender<Event>, Receiver<Event>) = bounded(128);
    let (tx_main, rx_kube): (Sender<Event>, Receiver<Event>) = bounded(256);
    let tx_kube = tx_input.clone();
    let tx_tick = tx_input.clone();

    let is_terminated = Arc::new(AtomicBool::new(false));

    let is_terminated_clone = is_terminated.clone();

    let read_key_handler = thread::spawn(move || read_key(tx_input, is_terminated_clone));

    let is_terminated_clone = is_terminated.clone();
    let kube_process_handler = thread::spawn(move || {
        KubeWorker::new(tx_kube, rx_kube, is_terminated_clone, kube_worker_config).run()
    });

    let is_terminated_clone = is_terminated.clone();
    let tick_handler = thread::spawn(move || {
        tick(
            tx_tick,
            time::Duration::from_millis(200),
            is_terminated_clone,
        )
    });

    let backend = CrosstermBackend::new(io::stdout());

    let namespace = Rc::new(RefCell::new(Namespace::new()));
    let context = Rc::new(RefCell::new(Context::new()));

    let mut terminal = Terminal::with_options(
        backend,
        TerminalOptions {
            viewport: Viewport::Fullscreen,
        },
    )?;

    let mut window =
        WindowInit::new(split_mode, tx_main, context.clone(), namespace.clone()).build();

    terminal.clear()?;

    while !is_terminated.load(Ordering::Relaxed) {
        terminal.draw(|f| {
            window.render(f);
        })?;

        match window_action(&mut window, &rx_main) {
            WindowEvent::Continue => {}
            WindowEvent::CloseWindow => {
                is_terminated.store(true, std::sync::atomic::Ordering::Relaxed);
                // break
            }
            WindowEvent::UpdateContents(ev) => {
                update_contents(
                    &mut window,
                    ev,
                    &mut context.borrow_mut(),
                    &mut namespace.borrow_mut(),
                );
            }
        }
    }

    match read_key_handler.join() {
        Ok(ret) => ret?,
        Err(e) => {
            if let Some(e) = e.downcast_ref::<&str>() {
                panic!("read_key thread panicked: {:?}", e);
            };
        }
    }

    match kube_process_handler.join() {
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

    let config = configure();

    if config.logging {
        Logger::init()?;
    }

    enable_raw_mode!();

    let result = run(config);

    disable_raw_mode!();

    result

    // Ok(())
}
