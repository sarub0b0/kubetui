#[allow(unused_imports)]
use chrono::{DateTime, Duration, Utc};

#[allow(unused_imports)]
use std::{
    error::Error,
    io::{self, stdout, Write},
};

#[allow(unused_imports)]
use crossterm::{
    event::{self, poll, read, DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyCode},
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
    Api, Client,
};

extern crate kubetui;
#[allow(unused_imports)]
use kubetui::{util::age, window::*};

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

fn generate_title(title: &str, selected: bool) -> Spans {
    let title = if selected {
        format!("✔︎ {}", title)
    } else {
        format!("──{}", title)
    };
    Spans::from(vec![
        Span::styled("─", Style::default()),
        Span::styled(title, Style::default().add_modifier(Modifier::BOLD)),
    ])
}

fn draw_panes<B: Backend>(f: &mut Frame<B>, area: Rect, tab: &Tab) {
    let chunks = tab.chunks(area);

    for pane in tab.panes() {
        let block = widgets::Block::default()
            .title(generate_title(
                pane.title(),
                pane.selected(tab.selected_pane()),
            ))
            .borders(widgets::Borders::ALL)
            .border_style(Style::default().add_modifier(Modifier::BOLD));

        match pane.widget() {
            Widget::List(list) => {
                draw_list(
                    f,
                    block,
                    chunks[pane.chunk_index()],
                    &list.items(),
                    &mut list.state().borrow_mut(),
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
) {
    let items: Vec<widgets::ListItem> = items
        .iter()
        .map(|i| widgets::ListItem::new(i.as_ref()))
        .collect();

    let li = widgets::List::new(items)
        .block(block)
        .style(Style::default())
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    f.render_stateful_widget(li, area, state);
}

fn draw<B: Backend>(f: &mut Frame<B>, window: &mut Window) {
    let areas = window.chunks(f.size());

    draw_tab(f, areas[0], &window.tabs(), window.selected_tab_index());

    draw_panes(f, areas[1], window.selected_tab());
}

// ❯  k get pod
// NAME                        READY   STATUS    RESTARTS   AGE
// bff-7d5ddf8f9f-49jfp        1/1     Running   0          12h
// database-8c7c97bbb-tl7wb    1/1     Running   0          12h
// front-95998dd5c-rhvqj       1/1     Running   0          12h
// mroonga-6c787bccc9-6dc8t    1/1     Running   0          12h
// mysql-7b4fd69f94-9mgs7      1/1     Running   0          12h
// netshoot-6dfdc678b8-4lhrp   1/1     Running   0          12h
async fn get_pod_info() -> Vec<String> {
    let client = Client::try_default().await.unwrap();
    let pods: Api<Pod> = Api::namespaced(client, "kosay");
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
        let status = pods.get_status(&p.name()).await.unwrap();
        let meta = Meta::meta(&p);
        let creation_timestamp: DateTime<Utc> = match meta.creation_timestamp {
            Some(ref time) => time.0,
            None => current_datetime,
        };
        let duration: Duration = current_datetime - creation_timestamp;

        ret.push(format!(
            "{:width$} {}    {}",
            p.name(),
            status.status.unwrap().phase.unwrap(),
            age(&duration),
            width = max_name_len + 4
        ));
    }
    ret
}

#[tokio::main]
async fn main() -> Result<(), io::Error> {
    let pod_info = get_pod_info().await;

    enable_raw_mode().unwrap();

    execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture).unwrap();

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend).unwrap();

    let tabs = vec![
        Tab::new(
            "1:Pods".to_string(),
            vec![
                Pane::new(String::from("Pods"), Widget::List(List::new(pod_info)), 0),
                Pane::new(
                    String::from("List 1"),
                    Widget::List(List::new(vec![
                        String::from("Item 1"),
                        String::from("Item 2"),
                        String::from("Item 3"),
                    ])),
                    1,
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

        match read().unwrap() {
            CEvent::Key(ev) => match ev.code {
                KeyCode::Char('q') => break,
                KeyCode::Char('j') => window.select_next_item(),
                KeyCode::Char('k') => window.select_prev_item(),
                KeyCode::Tab => {
                    window.select_next_pane();
                }
                KeyCode::Char(n @ '1'..='9') => window.select_tab(n as usize - b'0' as usize),
                KeyCode::Char(_) => {}
                _ => {}
            },
            CEvent::Mouse(_) => {}
            CEvent::Resize(_, _) => {}
        }
    }

    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture).unwrap();
    disable_raw_mode().unwrap();
    Ok(())
}
