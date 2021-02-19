#[allow(unused_imports)]
use chrono::{DateTime, Duration, Utc};

#[allow(unused_imports)]
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time;

#[allow(unused_imports)]
use tokio::runtime::Runtime;

#[allow(unused_imports)]
use std::{
    error::Error,
    io::{self, stdout, Write},
};

#[allow(unused_imports)]
use crossterm::{
    event::{
        self, poll, read, DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyCode,
        KeyEvent, KeyModifiers,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

#[allow(unused_imports)]
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Corner, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets, Frame, Terminal,
};

#[allow(unused_imports)]
use k8s_openapi::{
    api::core::v1::{Pod, PodStatus},
    apimachinery::pkg::apis::meta::v1::Time,
};
use kube::{
    api::{ListParams, Meta},
    config::Kubeconfig,
    Api, Client, Config,
};

extern crate kubetui;
#[allow(unused_imports)]
use kubetui::{util::age, window::*};

enum Event {
    Input(KeyEvent),
    Kube(Kube),
    Tick,
    Resize,
    Mouse,
}

fn draw_tab<B: Backend>(f: &mut Frame<B>, chunk: Rect, tabs: &Vec<Tab>, index: usize) {
    let titles: Vec<Spans> = tabs
        .iter()
        .map(|t| Spans::from(format!(" {} ", t.title())))
        .collect();

    let block = widgets::Block::default().style(Style::default());

    let tabs = widgets::Tabs::new(titles)
        .block(block)
        .select(index)
        .highlight_style(Style::default().fg(Color::White).bg(Color::LightBlue));

    f.render_widget(tabs, chunk);
}

fn generate_title(title: &str, border_color: Color, selected: bool) -> Spans {
    let prefix = if selected { "✔︎ " } else { "──" };
    Spans::from(vec![
        Span::styled("─", Style::default()),
        Span::styled(prefix, Style::default().fg(border_color)),
        Span::styled(
            title,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
    ])
}

fn draw_panes<B: Backend>(f: &mut Frame<B>, area: Rect, tab: &Tab) {
    let chunks = tab.chunks(area);

    for pane in tab.panes() {
        let selected = pane.selected(tab.selected_pane());

        let border_color = if selected {
            Color::White
        } else {
            Color::DarkGray
        };

        let block = widgets::Block::default()
            .title(generate_title(pane.title(), border_color, selected))
            .borders(widgets::Borders::ALL)
            .border_style(Style::default().fg(border_color));

        match pane.widget() {
            Widget::List(list) => {
                draw_list(
                    f,
                    block,
                    chunks[pane.chunk_index()],
                    &list.items(),
                    &mut list.state().borrow_mut(),
                    selected,
                );
            }
        }
    }
}

fn draw_list<B: Backend>(
    f: &mut Frame<B>,
    block: widgets::Block,
    area: Rect,
    items: &Vec<String>,
    state: &mut widgets::ListState,
    selected: bool,
) {
    let items: Vec<widgets::ListItem> = items
        .iter()
        .map(|i| widgets::ListItem::new(i.as_ref()))
        .collect();

    let style = if selected {
        Style::default().add_modifier(Modifier::REVERSED)
    } else {
        Style::default()
    };

    let li = widgets::List::new(items)
        .block(block)
        .style(Style::default())
        .highlight_style(style);

    f.render_stateful_widget(li, area, state);
}

fn draw_datetime<B: Backend>(f: &mut Frame<B>, area: Rect) {
    let block = widgets::Block::default().style(Style::default());

    let text = Spans::from(vec![Span::raw(format!(
        " {}",
        Utc::now().format("%Y年%m月%d日 %H時%M分%S秒")
    ))]);

    let paragraph = widgets::Paragraph::new(text).block(block);

    f.render_widget(paragraph, area);
}

fn draw<B: Backend>(f: &mut Frame<B>, window: &mut Window) {
    let areas = window.chunks(f.size());

    draw_tab(f, areas[0], &window.tabs(), window.selected_tab_index());

    draw_panes(f, areas[1], window.selected_tab());

    draw_datetime(f, areas[2]);
}

async fn get_pod_info(client: Client, namespace: &str) -> Vec<String> {
    let pods: Api<Pod> = Api::namespaced(client, namespace);
    let lp = ListParams::default();

    let pods_list = pods.list(&lp).await.unwrap();

    let max_name_len = pods_list
        .iter()
        .max_by(|r, l| r.name().len().cmp(&l.name().len()))
        .unwrap()
        .name()
        .len();

    let current_datetime: DateTime<Utc> = Utc::now();

    let mut ret: Vec<String> = Vec::new();
    for p in pods_list {
        let meta = Meta::meta(&p);
        let status = &p.status;
        let name = meta.name.clone().unwrap();

        let phase = match status {
            Some(s) => s.phase.clone().unwrap(),
            None => "Unknown".to_string(),
        };
        let creation_timestamp: DateTime<Utc> = match &meta.creation_timestamp {
            Some(ref time) => time.0,
            None => current_datetime,
        };
        let duration: Duration = current_datetime - creation_timestamp;

        ret.push(format!(
            "{:width$} {}    {}",
            name,
            phase,
            age(&duration),
            width = max_name_len + 4
        ));
    }
    ret
}

fn read_key(tx: Sender<Event>) {
    loop {
        match read().unwrap() {
            CEvent::Key(ev) => tx.send(Event::Input(ev)).unwrap(),
            CEvent::Mouse(_) => tx.send(Event::Mouse).unwrap(),
            CEvent::Resize(_, _) => tx.send(Event::Resize).unwrap(),
        }
    }
}

enum Kube {
    Pod(Vec<String>),
    Namespace(Option<Vec<String>>),
}

fn get_namespace_list() -> Option<Vec<String>> {
    Some(vec![
        "ns0".to_string(),
        "ns1".to_string(),
        "ns2".to_string(),
        "ns3".to_string(),
    ])
}

fn kube_process(tx: Sender<Event>, rx: Receiver<Event>) {
    let rt = Runtime::new().unwrap();
    rt.block_on(async move {
        let kubeconfig = Kubeconfig::read().unwrap();
        let current_context = kubeconfig.current_context.unwrap();

        let current_context = kubeconfig
            .contexts
            .iter()
            .find(|n| n.name == current_context);

        let namespace = current_context.unwrap().clone().context.namespace.unwrap();

        let client = Client::try_default().await.unwrap();

        let timeout = time::Duration::from_secs(2);
        loop {
            match rx.recv_timeout(timeout) {
                Ok(ev) => match ev {
                    Event::Kube(_) => tx
                        .send(Event::Kube(Kube::Namespace(get_namespace_list())))
                        .unwrap(),
                    _ => {
                        unreachable!()
                    }
                },
                Err(_) => {
                    let pod_info = get_pod_info(client.clone(), &namespace).await;
                    tx.send(Event::Kube(Kube::Pod(pod_info))).unwrap();
                }
            }
        }
    });
}

fn main() -> Result<(), io::Error> {
    let (tx_input, rx_main): (Sender<Event>, Receiver<Event>) = mpsc::channel();
    let (tx_main, rx_kube): (Sender<Event>, Receiver<Event>) = mpsc::channel();
    let tx_kube = tx_input.clone();

    thread::spawn(move || read_key(tx_input));
    thread::spawn(move || kube_process(tx_kube, rx_kube));

    enable_raw_mode().unwrap();

    execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture).unwrap();

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend).unwrap();

    let tabs = vec![
        Tab::new(
            "1:Pods".to_string(),
            vec![
                Pane::new(
                    String::from("Pods"),
                    Widget::List(List::new(vec![String::new()])),
                    0,
                    Type::POD,
                ),
                Pane::new(
                    String::from("List 1"),
                    Widget::List(List::new(vec![
                        String::from("Item 1"),
                        String::from("Item 2"),
                        String::from("Item 3"),
                    ])),
                    1,
                    Type::LOG,
                ),
            ],
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref()),
        ),
        Tab::new(
            "Tab 1".to_string(),
            vec![Pane::new(
                String::from("List 0"),
                Widget::List(List::new(vec![
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

    let timeout = time::Duration::from_millis(500);
    loop {
        terminal.draw(|f| draw(f, &mut window)).unwrap();

        match rx_main.recv_timeout(timeout) {
            Ok(ev) => match ev {
                Event::Input(ev) => match ev.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('j') => window.select_next_item(),
                    KeyCode::Char('k') => window.select_prev_item(),
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
                    KeyCode::Char(_) => {}
                    _ => {}
                },
                Event::Mouse => {}
                Event::Resize => {}
                Event::Tick => {}
                Event::Kube(k) => match k {
                    Kube::Pod(info) => {
                        window.update_pod_status(&info);
                    }
                    Kube::Namespace(_) => {}
                },
            },
            Err(_) => {}
        }
    }

    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture).unwrap();
    disable_raw_mode().unwrap();
    Ok(())
}
