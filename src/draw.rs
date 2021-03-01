use super::{event::*, view::*};

use std::cell::RefCell;
use std::rc::Rc;
use std::thread;

#[allow(unused_imports)]
use chrono::{DateTime, Duration, Utc};

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
    f.render_widget(window.tabs(), window.tab_chunk());
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

        let block = Block::default()
            .title(generate_title(pane.title(), border_color, selected))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));

        match pane.ty() {
            Type::POD => {
                let pod = pane.widget().pod().unwrap();

                f.render_stateful_widget(
                    pod.widget(block),
                    pane.chunk(),
                    &mut pod.state().borrow_mut().state(),
                );
            }
            Type::LOG => {
                let log = pane.widget().log().unwrap();
                f.render_widget(log.widget(block), pane.chunk());
            }
            Type::NONE => {}
        }
    }
}

fn datetime() -> Span<'static> {
    Span::raw(format!(
        " {}",
        Utc::now().format("%Y年%m月%d日 %H時%M分%S秒")
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

    let log_status = log_status(window.log_status());
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

    if window.drawable_popup() {
        let (list, state, chunk) = window.popup();

        f.render_widget(Clear, chunk);
        f.render_stateful_widget(list, chunk, &mut state.borrow_mut());
    }
}
