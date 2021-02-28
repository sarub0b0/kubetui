use super::{event::*, window::*};

use std::cell::RefCell;
use std::rc::Rc;
use std::thread;

#[allow(unused_imports)]
use chrono::{DateTime, Duration, Utc};
use widgets::ListState;

#[allow(unused_imports)]
use std::sync::{
    mpsc::{self, Receiver, Sender},
    Arc, RwLock,
};

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
    api::core::v1::{Namespace, Pod, PodStatus},
    apimachinery::pkg::apis::meta::v1::Time,
};

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

fn draw_panes<B: Backend>(f: &mut Frame<B>, tab: &Tab) {
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

        match pane.ty() {
            Type::POD => {
                let pod = pane.widget().pod().unwrap();

                f.render_stateful_widget(
                    pod.list(block),
                    pane.chunk(),
                    &mut pod.state().borrow_mut().state(),
                );
            }
            Type::LOG => {
                let log = pane.widget().log().unwrap();
                f.render_widget(log.paragraph(block), pane.chunk());
            }
            Type::NONE => {}
        }
    }
}

fn draw_datetime<B: Backend>(f: &mut Frame<B>, chunk: Rect) {
    let block = widgets::Block::default().style(Style::default());

    let text = Spans::from(vec![Span::raw(format!(
        " {}",
        Utc::now().format("%Y年%m月%d日 %H時%M分%S秒")
    ))]);

    let paragraph = widgets::Paragraph::new(text).block(block);

    f.render_widget(paragraph, chunk);
}

fn draw_status<B: Backend>(f: &mut Frame<B>, chunk: Rect) {
    draw_datetime(f, chunk);
}

pub fn draw<B: Backend>(f: &mut Frame<B>, window: &mut Window) {
    let chunks = window.chunks();

    draw_tab(f, chunks[0], &window.tabs(), window.selected_tab_index());

    draw_panes(f, window.selected_tab());

    draw_status(f, chunks[2]);
}
