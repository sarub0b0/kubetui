use crossbeam::channel::{unbounded, Receiver, Sender};

use std::{
    cell::RefCell,
    io, panic,
    rc::Rc,
    sync::{atomic::AtomicBool, Arc},
    thread, time,
};

use ::event::{error::Result, input::*, kubernetes::*, tick::*, Event};

use tui_wrapper::{
    crossterm::{
        cursor::Show,
        event::{DisableMouseCapture, EnableMouseCapture},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    tui::{
        backend::{Backend, CrosstermBackend},
        layout::Rect,
        Terminal, TerminalOptions, Viewport,
    },
    WindowEvent,
};

extern crate kubetui;
use kubetui::{
    action::{update_contents, window_action},
    config::{configure, Config},
    window::Init,
    Context, Namespace,
};

#[cfg(feature = "logging")]
use kubetui::log::logging;

macro_rules! enable_raw_mode {
    () => {
        enable_raw_mode().unwrap();
        execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture).unwrap();
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
        .unwrap();
        disable_raw_mode().unwrap();
    };
}

fn run(config: Config) -> Result<()> {
    let (tx_input, rx_main): (Sender<Event>, Receiver<Event>) = unbounded();
    let (tx_main, rx_kube): (Sender<Event>, Receiver<Event>) = unbounded();
    let tx_kube = tx_input.clone();
    let tx_tick = tx_input.clone();

    let is_terminated = Arc::new(AtomicBool::new(false));

    let is_terminated_clone = is_terminated.clone();

    let read_key_handler = thread::spawn(move || read_key(tx_input, is_terminated_clone));

    let is_terminated_clone = is_terminated.clone();
    let kube_process_handler =
        thread::spawn(move || kube_process(tx_kube, rx_kube, is_terminated_clone));

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

    let mut window = Init::new(
        config.split_mode(),
        tx_main,
        context.clone(),
        namespace.clone(),
    )
    .build();

    terminal.clear()?;
    window.update_chunks(terminal.size()?);

    loop {
        terminal.draw(|f| {
            window.render(f);
        })?;

        match window_action(&mut window, &rx_main) {
            WindowEvent::Continue => {}
            WindowEvent::CloseWindow => {
                break;
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

    is_terminated.store(true, std::sync::atomic::Ordering::Relaxed);

    read_key_handler.join().unwrap();

    kube_process_handler
        .join()
        .unwrap_or_else(|e| *e.downcast().unwrap())?;

    tick_handler.join().unwrap();

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
