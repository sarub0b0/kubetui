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

        match pane.ty() {
            Type::POD => {
                draw_list(
                    f,
                    block,
                    chunks[pane.chunk_index()],
                    &pane.widget().pod().unwrap().items(),
                    &mut pane.widget().pod().unwrap().state().borrow_mut().state(),
                    selected,
                );
            }
            Type::LOG => {
                let chunk = chunks[pane.chunk_index()];
                let chunk_height = chunk.y;
                let log = pane.widget().log();
                let current_selected = log.unwrap().selected();

                let items_len = log.unwrap().items().len() as u16;
                let selected = if chunk_height < items_len {
                    current_selected
                } else {
                    current_selected
                };
                // let limit_index = if chunk_height < log.unwrap().items().len() as u16 - 1 {
                //     chunk_height - log.unwrap().items().len() as u16 - 1
                // } else {
                //     chunk_height
                // };

                // draw_paragraph(f, block, chunk, &log.unwrap().spans(), selected);
                f.render_widget(log.unwrap().paragraph(block), chunk);
            }
            Type::NONE => {}
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

fn draw_paragraph<B: Backend>(
    f: &mut Frame<B>,
    block: widgets::Block,
    area: Rect,
    items: &Vec<Spans>,
    state: u16,
) {
    let paragraph = widgets::Paragraph::new(items.clone())
        .block(block)
        .style(Style::default().fg(Color::White).bg(Color::Black))
        .scroll((state, 0))
        .wrap(widgets::Wrap { trim: false });

    f.render_widget(paragraph, area);
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

pub fn draw<B: Backend>(f: &mut Frame<B>, window: &mut Window) {
    let areas = window.chunks(f.size());

    draw_tab(f, areas[0], &window.tabs(), window.selected_tab_index());

    draw_panes(f, areas[1], window.selected_tab());

    draw_datetime(f, areas[2]);
}
