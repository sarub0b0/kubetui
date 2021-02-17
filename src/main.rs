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
    layout::{Constraint, Corner, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame, Terminal,
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

fn draw<B: Backend>(f: &mut Frame<B>, events: &mut Vec<&mut Events>) {
    let mut index = 0;
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(f.size());

    for e in events {
        let block = Block::default()
            .title(vec![
                Span::styled("─", Style::default()),
                Span::styled(
                    format!("Block {}", index),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
            ])
            .borders(Borders::ALL)
            .border_style(Style::default().add_modifier(Modifier::BOLD));

        let items: Vec<ListItem> = e.items.iter().map(|i| ListItem::new(i.as_ref())).collect();

        let list = List::new(items)
            .block(block)
            .style(Style::default())
            .highlight_symbol(">");

        f.render_stateful_widget(list, chunks[index], &mut e.state);
        index += 1;
    }
}

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

    let mut e0 = Events::new(vec![
        String::from("Item 1"),
        String::from("Item 2"),
        String::from("Item 3"),
    ]);
    let mut e1 = Events::new(vec![
        String::from("Item 1"),
        String::from("Item 2"),
        String::from("Item 3"),
    ]);

    let mut events: Vec<&mut Events> = vec![&mut e0, &mut e1];

    let mut focus_index = 0;

    loop {
        terminal.draw(|f| draw(f, &mut events)).unwrap();

        let e = &mut events;
        match read().unwrap() {
            CEvent::Key(ev) => match ev.code {
                KeyCode::Char('q') => break,
                KeyCode::Char('j') => e[focus_index].next(),
                KeyCode::Char('k') => e[focus_index].prev(),
                KeyCode::Tab => {
                    focus_index = if events.len() - 1 <= focus_index {
                        0
                    } else {
                        focus_index + 1
                    };
                }
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
