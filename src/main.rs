use crossbeam::channel::{unbounded, Receiver, Sender};
use std::cell::RefCell;
use std::panic;
use std::rc::Rc;
use std::thread;
use std::time;

use std::io;

use crossterm::{
    cursor::Show,
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders},
    Terminal, TerminalOptions, Viewport,
};

use clipboard::{ClipboardContext, ClipboardProvider};

use event::{input::*, kubernetes::*, tick::*, Event};
use tui_wrapper::{widget::*, *};

extern crate kubetui;
use component::{multiple_select::MultipleSelect, single_select::SingleSelect};
use kubetui::*;

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

fn run() {
    let (tx_input, rx_main): (Sender<Event>, Receiver<Event>) = unbounded();
    let (tx_main, rx_kube): (Sender<Event>, Receiver<Event>) = unbounded();
    let tx_kube = tx_input.clone();
    let tx_tick = tx_input.clone();

    thread::spawn(move || read_key(tx_input));
    thread::spawn(move || kube_process(tx_kube, rx_kube));
    thread::spawn(move || tick(tx_tick, time::Duration::from_millis(200)));

    let backend = CrosstermBackend::new(io::stdout());
    let chunk = backend.size().unwrap();

    let clipboard: Result<ClipboardContext, _> = ClipboardProvider::new();

    // TODO: 画面サイズ変更時にクラッシュする問題の解決
    //
    // Terminal::new()の場合は、teminal.draw実行時にautoresizeを実行してバッファを更新する。
    // そのため、リサイズイベント時に使用したサイズとterminal.draw実行時のサイズに差がでで
    // クラッシュすることがある。
    // 応急処置として、ドキュメントにはUNSTABLEとあるがdraw実行時のautoresizeを無効にする
    // オプションを使用する。
    //
    // UNSTABLE CODE
    let mut terminal = Terminal::with_options(
        backend,
        TerminalOptions {
            viewport: Viewport::fixed(chunk),
        },
    )
    .unwrap();

    let mut logs_widget = Text::new(vec![]).enable_wrap().enable_follow();
    let mut raw_data_widget = Text::new(vec![]).enable_wrap();

    if let Ok(cb) = clipboard {
        let cb = Rc::new(RefCell::new(cb));
        logs_widget = logs_widget.clipboard(cb.clone());
        raw_data_widget = raw_data_widget.clipboard(cb);
    }

    let tabs = vec![
        Tab::new(
            view_id::tab_pods,
            "1:Pods",
            vec![
                Pane::new(
                    "Pods",
                    Widget::Table(Table::new(
                        vec![vec![]],
                        vec![
                            "NAME".to_string(),
                            "READY".to_string(),
                            "STATUS".to_string(),
                            "AGE".to_string(),
                        ],
                    )),
                    0,
                    view_id::tab_pods_pane_pods,
                ),
                Pane::new(
                    "Logs",
                    Widget::Text(logs_widget),
                    1,
                    view_id::tab_pods_pane_logs,
                ),
            ],
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref()),
        ),
        Tab::new(
            view_id::tab_configs,
            "2:Configs",
            vec![
                Pane::new(
                    "Configs",
                    Widget::List(List::new(vec![])),
                    0,
                    view_id::tab_configs_pane_configs,
                ),
                Pane::new(
                    "Raw Data",
                    Widget::Text(raw_data_widget),
                    1,
                    view_id::tab_configs_pane_raw_data,
                ),
            ],
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref()),
        ),
        Tab::new(
            view_id::tab_event,
            "3:Event",
            vec![Pane::new(
                "Event",
                Widget::Text(Text::new(vec![]).enable_wrap().enable_follow()),
                0,
                view_id::tab_event_pane_event,
            )],
            Layout::default().constraints([Constraint::Percentage(100)].as_ref()),
        ),
        Tab::new(
            view_id::tab_apis,
            "4:APIs",
            vec![Pane::new(
                "APIs",
                Widget::Text(Text::new(vec![])),
                0,
                view_id::tab_apis_pane_apis,
            )],
            Layout::default().constraints([Constraint::Percentage(100)].as_ref()),
        ),
    ];

    let mut subwin_namespace = SubWindow::new(
        view_id::subwin_ns,
        "Namespace",
        SingleSelect::new(view_id::subwin_ns_pane_ns, "Namespace"),
        Some(Block::default().borders(Borders::ALL)),
    );

    let mut subwin_apis = SubWindow::new(
        view_id::subwin_apis,
        "APIs",
        MultipleSelect::new(view_id::subwin_apis_pane, "Select APIs"),
        Some(Block::default().borders(Borders::ALL)),
    );

    let mut window = Window::new(tabs);

    terminal.clear().unwrap();

    window.update_chunks(terminal.size().unwrap());
    subwin_namespace.update_chunks(terminal.size().unwrap());
    subwin_apis.update_chunks(terminal.size().unwrap());

    let mut current_namespace = "None".to_string();
    let mut current_context = "None".to_string();

    tx_main
        .send(Event::Kube(Kube::GetCurrentContextRequest))
        .unwrap();

    let mut subwin_id: Option<&str> = None;

    loop {
        terminal
            .draw(|f| {
                window.render(f, &current_context, &current_namespace);

                if let Some(id) = subwin_id {
                    match id {
                        view_id::subwin_ns => {
                            subwin_namespace.render(f);
                        }
                        view_id::subwin_apis => {
                            subwin_apis.render(f);
                        }
                        _ => {}
                    }
                }
            })
            .unwrap();

        let event = if let Some(id) = subwin_id {
            match id {
                view_id::subwin_ns => namespace_subwin_action(
                    &mut window,
                    &mut subwin_namespace,
                    &tx_main,
                    &rx_main,
                    &mut current_namespace,
                ),

                view_id::subwin_apis => {
                    apis_subwin_action(&mut window, &mut subwin_apis, &tx_main, &rx_main)
                }
                _ => WindowEvent::Continue,
            }
        } else {
            window_action(&mut window, &tx_main, &rx_main)
        };

        match event {
            WindowEvent::Continue => {}
            WindowEvent::CloseWindow => {
                break;
            }
            WindowEvent::CloseSubWindow => {
                subwin_id = None;
            }
            WindowEvent::OpenSubWindow(id) => {
                subwin_id = Some(id);
            }
            WindowEvent::ResizeWindow(w, h) => {
                let chunk = Rect::new(0, 0, w, h);
                terminal.resize(chunk).unwrap();
                window.update_chunks(chunk);
                subwin_namespace.update_chunks(chunk);
                subwin_apis.update_chunks(chunk);
            }
            WindowEvent::UpdateContents(kube_ev) => match kube_ev {
                Kube::Pod(info) => {
                    set_items_window_pane(
                        &mut window,
                        view_id::tab_pods_pane_pods,
                        WidgetItem::DoubleArray(info),
                    );
                }

                Kube::Configs(configs) => {
                    set_items_window_pane(
                        &mut window,
                        view_id::tab_configs_pane_configs,
                        WidgetItem::Array(configs),
                    );
                }
                Kube::LogStreamResponse(logs) => {
                    append_items_window_pane(
                        &mut window,
                        view_id::tab_pods_pane_logs,
                        WidgetItem::Array(logs),
                    );
                }

                Kube::ConfigResponse(raw) => {
                    set_items_window_pane(
                        &mut window,
                        view_id::tab_configs_pane_raw_data,
                        WidgetItem::Array(raw),
                    );
                }

                Kube::GetCurrentContextResponse(ctx, ns) => {
                    current_context = ctx;
                    current_namespace = ns;
                }
                Kube::Event(ev) => {
                    set_items_window_pane(
                        &mut window,
                        view_id::tab_event_pane_event,
                        WidgetItem::Array(ev),
                    );
                }
                Kube::APIsResults(apis) => {
                    set_items_window_pane(
                        &mut window,
                        view_id::tab_apis_pane_apis,
                        WidgetItem::Array(apis),
                    );
                }
                Kube::GetNamespacesResponse(ns) => {
                    let pane = subwin_namespace.pane_mut();
                    pane.set_items(ns);
                }

                Kube::GetAPIsResponse(apis) => {
                    let pane = subwin_apis.pane_mut();
                    pane.set_list_items(apis);
                }
                _ => unreachable!(),
            },
        }
    }
}

fn main() {
    let default_hook = panic::take_hook();

    panic::set_hook(Box::new(move |info| {
        disable_raw_mode!();

        eprintln!("\x1b[31mPanic! disable raw mode\x1b[39m");

        default_hook(info);
    }));

    enable_raw_mode!();

    run();

    disable_raw_mode!();
}
