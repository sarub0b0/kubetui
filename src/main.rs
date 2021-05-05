// use std::sync::mpsc::{self, Receiver, Sender};
use crossbeam::channel::{unbounded, Receiver, Sender};
use std::panic;
use std::thread;
use std::time;

use std::io::{self, Write};

use crossterm::{
    cursor::Show,
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders},
    Terminal,
};

use event::{input::*, kubernetes::*, tick::*, Event};
use tui_wrapper::*;

extern crate kubetui;
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
    let mut terminal = Terminal::new(backend).unwrap();

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
                    Widget::Text(Text::new(vec![])),
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
                    Widget::Text(Text::new(vec![])),
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
                Widget::Text(Text::new(vec![])),
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
        Pane::new(
            "Namespace",
            Widget::List(List::new(vec![])),
            0,
            view_id::subwin_ns_pane_ns,
        ),
        None,
    );

    let mut subwin_apis = SubWindow::new(
        view_id::subwin_apis,
        "APIs",
        Select::new(view_id::subwin_apis_pane, "Select APIs"),
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

        if let Some(id) = subwin_id {
            let event = match id {
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
            };

            if let WindowEvent::CloseSubWindow = event {
                subwin_id = None;
            }
        } else {
            match window_action(
                &mut window,
                &mut subwin_namespace,
                &tx_main,
                &rx_main,
                &mut current_namespace,
                &mut current_context,
            ) {
                WindowEvent::CloseWindow => break,
                WindowEvent::Continue => {}
                WindowEvent::OpenSubWindow(id) => subwin_id = Some(id),
                WindowEvent::CloseSubWindow => {
                    unreachable!()
                }
            }
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
