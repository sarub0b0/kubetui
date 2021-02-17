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
    backend::CrosstermBackend,
    layout::{Constraint, Corner, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, List, ListItem, ListState},
    Terminal,
};

struct Events {
    items: Vec<String>,
    state: ListState,
}

impl Events {
    fn new(items: Vec<String>) -> Events {
        Events {
            items,
            state: ListState::default(),
        }
    }

    pub fn set_items(&mut self, items: Vec<String>) {
        self.items = items;
        self.state = ListState::default();
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if self.items.len() - 1 <= i {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };

        self.state.select(Some(i));
    }

    pub fn prev(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };

        self.state.select(Some(i));
    }

    pub fn unselect(&mut self) {
        self.state.select(None);
    }
}

// use k8s_openapi::api::core::v1::Pod;
// use kube::{
//     api::{Api, DeleteParams, ListParams, Meta, Patch, PostParams, WatchEvent},
//     Client,
// };

// #[tokio::main]
fn main() -> Result<(), io::Error> {
    // let client = Client::try_default().await.unwrap();
    // let pods: Api<Pod> = Api::namespaced(client, "taskbox");
    // let lp = ListParams::default();
    // for p in pods.list(&lp).await.unwrap() {
    //     println!("Found Pod: {}", Meta::name(&p));
    // }

    enable_raw_mode().unwrap();

    execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture).unwrap();

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend).unwrap();

    let mut events = Events::new(vec![
        String::from("Item 1"),
        String::from("Item 2"),
        String::from("Item 3"),
    ]);
    loop {
        terminal
            .draw(|f| {
                let chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(
                        [
                            Constraint::Percentage(50),
                            Constraint::Percentage(30),
                            Constraint::Percentage(20),
                        ]
                        .as_ref(),
                    )
                    .split(f.size());

                let block = Block::default()
                    .title(vec![
                        Span::styled("─", Style::default()),
                        Span::styled("Pods", Style::default().add_modifier(Modifier::BOLD)),
                    ])
                    .borders(Borders::ALL)
                    .border_style(Style::default().add_modifier(Modifier::BOLD));
                let items: Vec<ListItem> = events
                    .items
                    .iter()
                    .map(|i| ListItem::new(i.as_ref()))
                    .collect();
                let list = List::new(items)
                    .block(block)
                    .style(Style::default())
                    .highlight_symbol(">");
                f.render_stateful_widget(list, chunks[0], &mut events.state);
            })
            .unwrap();

        match read().unwrap() {
            CEvent::Key(ev) => match ev.code {
                KeyCode::Char('q') => break,
                KeyCode::Char('j') => events.next(),
                KeyCode::Char('k') => events.prev(),
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
