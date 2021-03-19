// use std::sync::mpsc::{self, Receiver, Sender};
use crossbeam::channel::{unbounded, Receiver, Sender};
use std::thread;
use std::time;

use std::io::{self, Write};

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    Terminal,
};

extern crate kubetui;
use kubetui::{
    draw::*,
    event::{input::*, kubernetes::*, tick::*, Event, Kube},
};
use tui_wrapper::*;

fn update_pod_logs(window: &mut Window, logs: Vec<String>) {
    let pane = window.pane_mut("logs");
    if let Some(p) = pane {
        let rect = p.chunk();
        let log = p.widget_mut().text_mut().unwrap();
        log.set_items(logs.to_vec());
        log.update_spans(rect.width);
        log.update_rows_size(rect.height);
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
        let widget = p.widget_mut();
        widget.set_items(configs.to_vec());
    }
}

fn update_configs_raw(window: &mut Window, configs: Vec<String>) {
    let pane = window.pane_mut("configs-raw");

    if let Some(p) = pane {
        let ch = p.chunk();
        let widget = p.widget_mut().text_mut().unwrap();
        widget.set_items(configs.to_vec());
        widget.update_spans(ch.width);
        widget.update_rows_size(ch.width);
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
    // let split: Vec<&str> = pane.widget().list().unwrap().items()[selected_index]
    //     .split(' ')
    //     .collect();
    // split[2].to_string()
}

fn setup_namespaces_popup(window: &mut Window, items: Option<Vec<String>>) {
    if let Some(items) = items {
        let popup = window.selected_tab_mut().popup_mut();
        if let Some(popup) = popup {
            let ns = popup.widget_mut().list_mut();
            if let Some(ns) = ns {
                ns.set_items(items);
            }
        }
    }
}

fn main() -> Result<(), io::Error> {
    let (tx_input, rx_main): (Sender<Event>, Receiver<Event>) = unbounded();
    let (tx_main, rx_kube): (Sender<Event>, Receiver<Event>) = unbounded();
    let tx_kube = tx_input.clone();
    let tx_tick = tx_input.clone();

    thread::spawn(move || read_key(tx_input));
    thread::spawn(move || kube_process(tx_kube, rx_kube));
    thread::spawn(move || tick(tx_tick, time::Duration::from_millis(200)));

    enable_raw_mode().unwrap();
    execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture).unwrap();

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
            Some(Popup::new(
                "Namespace",
                Widget::List(List::new(vec![])),
                "namespace",
            )),
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
            Some(Popup::new(
                "Namespace",
                Widget::List(List::new(vec![])),
                "namespace",
            )),
        ),
    ];

    let mut window = Window::new(tabs);

    terminal.clear().unwrap();

    window.update_chunks(terminal.size().unwrap());

    loop {
        terminal.draw(|f| draw(f, &mut window)).unwrap();
        match rx_main.recv().unwrap() {
            Event::Input(ev) => match ev.code {
                KeyCode::Char('q') => {
                    if window.selected_popup() {
                        window.unselect_popup();
                    } else {
                        break;
                    }
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    window.select_next_item();
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    window.select_prev_item();
                }
                KeyCode::Char('n') if ev.modifiers == KeyModifiers::CONTROL => {
                    window.select_next_item()
                }
                KeyCode::Char('p') if ev.modifiers == KeyModifiers::CONTROL => {
                    window.select_prev_item()
                }
                KeyCode::Char('u')
                    if ev.modifiers == KeyModifiers::CONTROL && !window.selected_popup() =>
                {
                    window.scroll_up()
                }
                KeyCode::Char('d')
                    if ev.modifiers == KeyModifiers::CONTROL && !window.selected_popup() =>
                {
                    window.scroll_down()
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
                    tx_main
                        .send(Event::Kube(Kube::GetNamespaceRequest))
                        .unwrap();
                }
                KeyCode::Enter => {
                    if window.selected_popup() {
                        let popup = window.popup().unwrap();
                        let ns = popup.widget().list().unwrap();
                        let index = ns.state().borrow().selected();
                        let select = ns.items()[index.unwrap()].clone();
                        tx_main
                            .send(Event::Kube(Kube::SetNamespace(select)))
                            .unwrap();
                        window.unselect_popup();
                    } else {
                        match window.selected_pane_id() {
                            "pods" => {
                                tx_main
                                    .send(Event::Kube(Kube::LogRequest(selected_pod(&window))))
                                    .unwrap();
                            }
                            "configs" => {
                                tx_main
                                    .send(Event::Kube(Kube::ConfigRequest(selected_config(
                                        &window,
                                    ))))
                                    .unwrap();
                            }
                            _ => {}
                        }
                    }
                }

                KeyCode::Char('G') => {
                    window.select_last_item();
                }
                KeyCode::Char('g') => {
                    window.select_first_item();
                }
                KeyCode::Char(_) => {}
                _ => {}
            },
            Event::Mouse => {}
            Event::Resize(w, h) => {
                window.update_chunks(Rect::new(0, 0, w, h));
                window.update_wrap();
            }
            Event::Tick => {}
            Event::Kube(k) => match k {
                Kube::Pod(info) => {
                    update_pod_status(&mut window, info);
                }

                Kube::Configs(configs) => {
                    update_configs(&mut window, configs);
                }
                Kube::GetNamespaceResponse(ns) => {
                    setup_namespaces_popup(&mut window, ns);

                    window.select_popup();
                }
                Kube::LogResponse(log) => {
                    update_pod_logs(&mut window, log);
                }
                Kube::ConfigResponse(raw) => {
                    update_configs_raw(&mut window, raw);
                }
                _ => {}
            },
        }
    }

    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture).unwrap();
    disable_raw_mode().unwrap();
    Ok(())
}
