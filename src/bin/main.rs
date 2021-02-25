use std::sync::mpsc::{self, Receiver, Sender};
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
    layout::{Constraint, Direction, Layout},
    Terminal,
};

extern crate kubetui;
use kubetui::{
    draw::*,
    event::{input::*, kubernetes::*, tick::*, Event, Kube},
    widget::{log::Logs, pod::Pods, Widgets},
    window::*,
};

fn main() -> Result<(), io::Error> {
    let (tx_input, rx_main): (Sender<Event>, Receiver<Event>) = mpsc::channel();
    let (tx_main, rx_kube): (Sender<Event>, Receiver<Event>) = mpsc::channel();
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
                Pane::new("Pods", Widgets::Pod(Pods::new(vec![])), 0, Type::POD),
                Pane::new("Logs", Widgets::Log(Logs::new(vec![])), 1, Type::LOG),
            ],
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref()),
        ),
        Tab::new(
            "Tab 1",
            vec![Pane::new(
                "List 0",
                Widgets::Pod(Pods::new(vec![
                    String::from("Item 1"),
                    String::from("Item 2"),
                    String::from("Item 3"),
                ])),
                0,
                Type::NONE,
            )],
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50)].as_ref()),
        ),
    ];

    let mut window = Window::new(tabs);

    terminal.clear().unwrap();
    loop {
        terminal.draw(|f| draw(f, &mut window)).unwrap();
        match rx_main.recv().unwrap() {
            Event::Input(ev) => match ev.code {
                KeyCode::Char('q') => break,
                KeyCode::Char('j') | KeyCode::Down => window.select_next_item(),
                KeyCode::Char('k') | KeyCode::Up => window.select_prev_item(),
                KeyCode::Char('n') if ev.modifiers == KeyModifiers::CONTROL => {
                    window.select_next_item()
                }
                KeyCode::Char('p') if ev.modifiers == KeyModifiers::CONTROL => {
                    window.select_prev_item()
                }
                KeyCode::Tab if ev.modifiers == KeyModifiers::NONE => {
                    window.select_next_pane();
                }
                KeyCode::BackTab | KeyCode::Tab if ev.modifiers == KeyModifiers::SHIFT => {
                    window.select_prev_pane();
                }
                KeyCode::Char(n @ '1'..='9') => window.select_tab(n as usize - b'0' as usize),
                KeyCode::Char('n') if ev.modifiers == KeyModifiers::NONE => {
                    tx_main.send(Event::Kube(Kube::Namespace(None))).unwrap()
                }
                KeyCode::Enter if window.focus_pane_type() == Type::POD => {
                    tx_main
                        .send(Event::Kube(Kube::LogRequest(window.selected_pod())))
                        .unwrap();
                    window.reset_pod_logs();
                }
                KeyCode::Char('G') => window.select_last_item(),
                KeyCode::Char('g') => window.select_first_item(),
                KeyCode::Char(_) => {}
                _ => {}
            },
            Event::Mouse => {}
            Event::Resize => {}
            Event::Tick => {}
            Event::Kube(k) => match k {
                Kube::Pod(info) => {
                    window.update_pod_status(info);
                }
                Kube::Namespace(_ns) => {}
                Kube::LogResponse(log) => {
                    window.update_pod_logs(log);
                }
                _ => {}
            },
        }
    }

    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture).unwrap();
    disable_raw_mode().unwrap();
    Ok(())
}
