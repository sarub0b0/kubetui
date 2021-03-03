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
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    Terminal,
};

extern crate kubetui;
use kubetui::{
    draw::*,
    event::{input::*, kubernetes::*, tick::*, Event, Kube},
    view::*,
    widget::*,
};

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

pub fn update_pod_status(window: &mut Window, info: Vec<String>) {
    let pane = window.pane_mut("pods");

    if let Some(p) = pane {
        let pod = p.widget_mut();
        pod.set_items(info.to_vec());
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
            "Tab 1",
            vec![Pane::new(
                "List 0",
                Widget::List(List::new(vec![
                    String::from("Item 1"),
                    String::from("Item 2"),
                    String::from("Item 3"),
                ])),
                0,
                "none",
            )],
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50)].as_ref()),
            None,
        ),
    ];

    let mut window = Window::new(tabs);

    terminal.clear().unwrap();

    window.update_chunks(terminal.size().unwrap());

    loop {
        terminal.draw(|f| draw(f, &mut window)).unwrap();
        match rx_main.recv().unwrap() {
            Event::Input(ev) => match ev.code {
                KeyCode::Char('q') if window.selected_popup() == false => break,
                KeyCode::Char('j') | KeyCode::Down => window.select_next_item(),
                KeyCode::Char('k') | KeyCode::Up => window.select_prev_item(),
                KeyCode::Char('n') if ev.modifiers == KeyModifiers::CONTROL => {
                    window.select_next_item()
                }
                KeyCode::Char('p') if ev.modifiers == KeyModifiers::CONTROL => {
                    window.select_prev_item()
                }
                KeyCode::Char('u') if ev.modifiers == KeyModifiers::CONTROL => window.scroll_up(),
                KeyCode::Char('d') if ev.modifiers == KeyModifiers::CONTROL => window.scroll_down(),
                KeyCode::Tab if ev.modifiers == KeyModifiers::NONE => {
                    window.select_next_pane();
                }
                KeyCode::BackTab | KeyCode::Tab if ev.modifiers == KeyModifiers::SHIFT => {
                    window.select_prev_pane();
                }
                KeyCode::Char(n @ '1'..='9') => window.select_tab(n as usize - b'0' as usize),
                KeyCode::Char('n') if ev.modifiers == KeyModifiers::NONE => tx_main
                    .send(Event::Kube(Kube::GetNamespaceRequest))
                    .unwrap(),
                KeyCode::Enter
                    if window.selected_pane_id() == "pods" && !window.selected_popup() =>
                {
                    tx_main
                        .send(Event::Kube(Kube::LogRequest(window.selected_pod())))
                        .unwrap();
                }

                KeyCode::Enter if window.selected_popup() => {
                    let popup = window.popup().unwrap();
                    let ns = popup.widget().list().unwrap();
                    let index = ns.state().borrow().selected();
                    let select = ns.items()[index.unwrap()].clone();
                    tx_main
                        .send(Event::Kube(Kube::SetNamespace(select)))
                        .unwrap();
                    window.unselect_popup();
                }
                KeyCode::Char('q') if window.selected_popup() => {
                    window.unselect_popup();
                }
                KeyCode::Char('G') => window.select_last_item(),
                KeyCode::Char('g') => window.select_first_item(),
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
                Kube::GetNamespaceResponse(ns) => {
                    window.setup_namespaces_popup(ns);
                    window.select_popup();
                }
                Kube::LogResponse(log) => {
                    update_pod_logs(&mut window, log);
                }
                _ => {}
            },
        }
    }

    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture).unwrap();
    disable_raw_mode().unwrap();
    Ok(())
}
