use tui_wrapper::*;

use std::cell::RefCell;
use std::rc::Rc;
use std::thread;

#[allow(unused_imports)]
use chrono::{DateTime, Duration, FixedOffset, Local, Utc};

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
    layout::{Alignment, Constraint, Corner, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame, Terminal,
};

#[allow(unused_imports)]
use k8s_openapi::{
    api::core::v1::{Namespace, Pod, PodStatus},
    apimachinery::pkg::apis::meta::v1::Time,
};

fn draw_tab<B: Backend>(f: &mut Frame<B>, window: &Window) {
    f.render_widget(window.widget(), window.tab_chunk());
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
        let selected = if tab.selected_popup() {
            false
        } else {
            pane.is_selected(tab.selected_pane())
        };

        let border_color = if selected {
            Color::White
        } else {
            Color::DarkGray
        };

        let block = Block::default()
            .title(generate_title(pane.title(), border_color, selected))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));

        match pane.id() {
            "pods" => {
                let pod = pane.widget().list().unwrap();

                f.render_stateful_widget(
                    pod.widget(block),
                    pane.chunk(),
                    &mut pod.state().borrow_mut(),
                );
            }
            "logs" => {
                let log = pane.widget().text().unwrap();
                f.render_widget(log.widget(block), pane.chunk());
            }
            "configs" => {
                let configs = pane.widget().list().unwrap();

                f.render_stateful_widget(
                    configs.widget(block),
                    pane.chunk(),
                    &mut configs.state().borrow_mut(),
                );
            }
            "configs-raw" => {
                let raw = pane.widget().text().unwrap();
                f.render_widget(raw.widget(block), pane.chunk());
            }

            _ => {}
        }
    }
}

fn datetime() -> Span<'static> {
    Span::raw(format!(
        " {}",
        Local::now().format("%Y年%m月%d日 %H時%M分%S秒")
    ))
}

fn log_status((current, rows): (u16, u16)) -> Span<'static> {
    let percent = if rows == 0 {
        100
    } else {
        (current * 100) / rows
    };

    // Span::raw(format!("{}%", percent))
    Span::raw(format!("{}/{}", current, rows))
}

fn draw_status<B: Backend>(f: &mut Frame<B>, chunk: Rect, window: &Window) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunk);

    let datetime = datetime();

    let text = Spans::from(datetime);
    let block = Block::default().style(Style::default());
    let paragraph = Paragraph::new(text).block(block);

    f.render_widget(paragraph, chunks[0]);

    let log_widget = window.selected_tab().selected_pane().widget().text();
    let log_status = match log_widget {
        Some(t) => log_status((t.selected(), t.row_size())),
        None => log_status((0, 0)),
    };

    let text = Spans::from(log_status);
    let block = Block::default().style(Style::default());
    let paragraph = Paragraph::new(text)
        .block(block)
        .alignment(Alignment::Right);

    f.render_widget(paragraph, chunks[1]);
}

pub fn draw<B: Backend>(f: &mut Frame<B>, window: &mut Window) {
    let chunks = window.chunks();

    draw_tab(f, &window);

    draw_panes(f, window.selected_tab());

    draw_status(f, chunks[2], &window);

    if window.selected_popup() {
        match window.popup() {
            Some(p) => {
                let ns = p.widget().list().unwrap();
                f.render_widget(Clear, p.chunk());

                let block = Block::default()
                    .title(generate_title(p.title(), Color::White, true))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::White));

                f.render_stateful_widget(ns.widget(block), p.chunk(), &mut ns.state().borrow_mut());
            }
            None => {}
        }
    }
}
