use anyhow::Result;
use crossbeam::channel::{bounded, Receiver, Sender};
use crossterm::{
    cursor::Show,
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use kubetui::{
    action::{update_contents, window_action},
    config::{configure, Config},
    context::{Context, Namespace},
    event::{input::read_key, kubernetes::KubeWorker, tick::tick, Event},
    tui_wrapper::WindowEvent,
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
use tui::{
    backend::Backend, backend::CrosstermBackend, layout::Rect, Terminal, TerminalOptions, Viewport,
};

#[cfg(feature = "logging")]
use kubetui::logging::logging;

macro_rules! enable_raw_mode {
    () => {
        enable_raw_mode().expect("failed to enable raw mode");
        execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture)
            .expect("failed to enable raw mode");
    };
}

macro_rules! disable_raw_mode {
    () => {
        execute!(
            io::stdout(),
            LeaveAlternateScreen,
            DisableMouseCapture,
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

    // TODO: 画面サイズ変更時にクラッシュする問題の解決
    //
    // Terminal::new()の場合は、terminal.draw実行時にautoresizeを実行してバッファを更新する。
    // そのため、リサイズイベント時に使用したサイズとterminal.draw実行時のサイズに差がでで
    // クラッシュすることがある。
    // 応急処置として、ドキュメントにはUNSTABLEとあるがdraw実行時のautoresizeを無効にする
    // オプションを使用する。
    //
    // UNSTABLE CODE
    let chunk = backend.size()?;
    let mut terminal = Terminal::with_options(
        backend,
        TerminalOptions {
            viewport: Viewport::fixed(chunk),
        },
    )?;

    let mut window =
        WindowInit::new(split_mode, tx_main, context.clone(), namespace.clone()).build();

    terminal.clear()?;
    window.update_chunks(terminal.size()?);

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
            WindowEvent::ResizeWindow(w, h) => {
                let chunk = Rect::new(0, 0, w, h);
                terminal.resize(chunk)?;
                window.update_chunks(chunk);
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
    #[cfg(feature = "logging")]
    logging();

    let default_hook = panic::take_hook();

    panic::set_hook(Box::new(move |info| {
        disable_raw_mode!();

        eprintln!("\x1b[31mPanic! disable raw mode\x1b[39m");

        default_hook(info);
    }));

    let config = configure();

    enable_raw_mode!();

    let result = run(config);

    disable_raw_mode!();

    result
}
