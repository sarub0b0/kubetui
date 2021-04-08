// use std::sync::mpsc::{self, Receiver, Sender};
use crossbeam::channel::{unbounded, Receiver, Sender};
use std::panic;
use std::thread;
use std::time;

use std::io::{self, Write};

use crossterm::{
    cursor::Show,
    event::{DisableMouseCapture, EnableMouseCapture, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    Terminal,
};

extern crate kubetui;
use event::{input::*, kubernetes::*, tick::*, Event};
use kubetui::draw::*;
use tui_wrapper::*;

fn update_event(window: &mut Window, ev: Vec<String>) {
    let pane = window.pane_mut("event");
    if let Some(p) = pane {
        let widget = p.widget_mut().text_mut().unwrap();
        let is_bottom = widget.is_bottom();

        widget.append_items(&ev);

        if is_bottom {
            widget.select_last();
        }
    }
}

fn update_pod_logs(window: &mut Window, logs: Vec<String>) {
    let pane = window.pane_mut("logs");
    if let Some(p) = pane {
        let widget = p.widget_mut().text_mut().unwrap();

        let is_bottom = widget.is_bottom();

        widget.append_items(&logs);

        if is_bottom {
            widget.select_last();
        }
    }
}

fn update_pod_status(window: &mut Window, info: Vec<String>) {
    let pane = window.pane_mut("pods");

    if let Some(p) = pane {
        let pod = p.widget_mut();
        pod.set_items(info.to_vec());
    }
}

fn update_configs(window: &mut Window, configs: Vec<String>) {
    let pane = window.pane_mut("configs");

    if let Some(p) = pane {
        p.widget_mut().set_items(configs.to_vec());
    }
}

fn update_configs_raw(window: &mut Window, configs: Vec<String>) {
    let pane = window.pane_mut("configs-raw");

    if let Some(p) = pane {
        p.widget_mut().set_items(configs.to_vec());
    }
}

fn selected_pod(window: &Window) -> String {
    let pane = window.pane("pods").unwrap();
    let selected_index = pane
        .widget()
        .list()
        .unwrap()
        .state()
        .borrow()
        .selected()
        .unwrap();
    let split: Vec<&str> = pane.widget().list().unwrap().items()[selected_index]
        .split(' ')
        .collect();
    split[0].to_string()
}

fn selected_config(window: &Window) -> String {
    let pane = window.pane("configs").unwrap();
    let selected_index = pane
        .widget()
        .list()
        .unwrap()
        .state()
        .borrow()
        .selected()
        .unwrap();
    pane.widget().list().unwrap().items()[selected_index].clone()
}

fn setup_namespaces_popup(window: &mut Window, items: Vec<String>) {
    let popup = window.popup_mut();
    let ns = popup.widget_mut().list_mut();
    if let Some(ns) = ns {
        ns.set_items(items);
    }
}

enum EventType {
    Quit,
    NoMatch,
    Match,
}

fn global_key(ev: KeyEvent, window: &mut Window, tx: &Sender<Event>) -> EventType {
    match ev.code {
        KeyCode::Char('q') => {
            return EventType::Quit;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            window.select_next_item();
        }
        KeyCode::Char('k') | KeyCode::Up => {
            window.select_prev_item();
        }
        KeyCode::Char('n') if ev.modifiers == KeyModifiers::CONTROL => {
            window.select_next_item();
        }
        KeyCode::Char('p') if ev.modifiers == KeyModifiers::CONTROL => {
            window.select_prev_item();
        }
        KeyCode::Char('u') if ev.modifiers == KeyModifiers::CONTROL => {
            window.scroll_up();
        }
        KeyCode::Char('d') if ev.modifiers == KeyModifiers::CONTROL => {
            window.scroll_down();
        }
        KeyCode::Tab if ev.modifiers == KeyModifiers::NONE => {
            window.select_next_pane();
        }
        KeyCode::BackTab | KeyCode::Tab if ev.modifiers == KeyModifiers::SHIFT => {
            window.select_prev_pane();
        }
        KeyCode::Char(n @ '1'..='9') => {
            window.select_tab(n as usize - b'0' as usize);
        }
        KeyCode::Char('n') => {
            tx.send(Event::Kube(Kube::GetNamespacesRequest)).unwrap();
        }
        KeyCode::Char('G') => {
            window.select_last_item();
        }
        KeyCode::Char('g') => {
            window.select_first_item();
        }
        _ => {
            return EventType::NoMatch;
        }
    }

    return EventType::Match;
}

fn normal_mode_key(ev: KeyEvent, window: &mut Window, tx: &Sender<Event>) -> EventType {
    match ev.code {
        KeyCode::Enter => match window.selected_pane_id() {
            "pods" => {
                let pane = window.pane_mut("logs");
                if let Some(p) = pane {
                    p.widget_mut().clear();
                }
                tx.send(Event::Kube(Kube::LogStreamRequest(selected_pod(&window))))
                    .unwrap();
            }
            "configs" => {
                let pane = window.pane_mut("configs-raw");
                if let Some(p) = pane {
                    p.widget_mut().clear();
                }
                tx.send(Event::Kube(Kube::ConfigRequest(selected_config(&window))))
                    .unwrap();
            }
            _ => {}
        },
        _ => {
            return EventType::NoMatch;
        }
    }

    return EventType::Match;
}
fn popup_mode_key(
    ev: KeyEvent,
    window: &mut Window,
    tx: &Sender<Event>,
    current_namespace: &mut String,
) -> EventType {
    match ev.code {
        KeyCode::Char('q') => {
            window.unselect_popup();
        }
        KeyCode::Enter => {
            let popup = window.popup();
            let ns = popup.widget().list().unwrap();
            let index = ns.state().borrow().selected();
            let selected_ns = ns.items()[index.unwrap()].clone();
            tx.send(Event::Kube(Kube::SetNamespace(selected_ns.clone())))
                .unwrap();
            window.unselect_popup();

            if let Some(p) = window.pane_mut("event") {
                let w = p.widget_mut().text_mut().unwrap();
                w.clear();
            }

            if let Some(p) = window.pane_mut("logs") {
                let w = p.widget_mut().text_mut().unwrap();
                w.clear();
            }

            if let Some(p) = window.pane_mut("configs-raw") {
                let w = p.widget_mut().text_mut().unwrap();
                w.clear();
            }

            *current_namespace = selected_ns;
        }
        _ => {
            return EventType::NoMatch;
        }
    }

    return EventType::Match;
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
            "1:Pods",
            vec![
                Pane::new("Pods", Widget::List(List::new(vec![])), 0, "pods"),
                Pane::new("Logs", Widget::Text(Text::new(vec![])), 1, "logs"),
            ],
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref()),
        ),
        Tab::new(
            "2:Configs",
            vec![
                Pane::new("Configs", Widget::List(List::new(vec![])), 0, "configs"),
                Pane::new(
                    "Raw Data",
                    Widget::Text(Text::new(vec![])),
                    1,
                    "configs-raw",
                ),
            ],
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref()),
        ),
        Tab::new(
            "3:Event",
            vec![Pane::new(
                "Event",
                Widget::Text(Text::new(vec![])),
                0,
                "event",
            )],
            Layout::default().constraints([Constraint::Percentage(100)].as_ref()),
        ),
    ];

    let mut window = Window::new(
        tabs,
        Popup::new("Namespace", Widget::List(List::new(vec![])), "namespace"),
    );

    terminal.clear().unwrap();

    window.update_chunks(terminal.size().unwrap());

    let mut current_namespace = "None".to_string();
    let mut current_context = "None".to_string();

    tx_main
        .send(Event::Kube(Kube::GetCurrentContextRequest))
        .unwrap();

    loop {
        terminal
            .draw(|f| draw(f, &mut window, &current_context, &current_namespace))
            .unwrap();

        match rx_main.recv().unwrap() {
            Event::Input(ev) => {
                let ty = if window.selected_popup() {
                    popup_mode_key(ev, &mut window, &tx_main, &mut current_namespace)
                } else {
                    normal_mode_key(ev, &mut window, &tx_main)
                };

                let ty = match ty {
                    EventType::NoMatch => global_key(ev, &mut window, &tx_main),
                    _ => EventType::NoMatch,
                };

                match ty {
                    EventType::Quit => break,
                    _ => {}
                }
            }

            Event::Mouse => {}
            Event::Resize(w, h) => {
                window.update_chunks(Rect::new(0, 0, w, h));
            }
            Event::Tick => {}
            Event::Kube(k) => match k {
                Kube::Pod(info) => {
                    update_pod_status(&mut window, info);
                }

                Kube::Configs(configs) => {
                    update_configs(&mut window, configs);
                }
                Kube::GetNamespacesResponse(ns) => {
                    setup_namespaces_popup(&mut window, ns);

                    window.select_popup();
                }
                Kube::LogStreamResponse(logs) => {
                    update_pod_logs(&mut window, logs);
                }

                Kube::ConfigResponse(raw) => {
                    update_configs_raw(&mut window, raw);
                }

                Kube::GetCurrentContextResponse(ctx, ns) => {
                    current_context = ctx;
                    current_namespace = ns;
                }
                Kube::Event(ev) => {
                    update_event(&mut window, ev);
                }
                _ => unreachable!(),
            },
        }
    }
}

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
