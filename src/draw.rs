use tui_wrapper::*;
use window::window_layout_index;

use chrono::Local;

use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
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

            "event" => {
                let ev = pane.widget().text().unwrap();
                f.render_widget(ev.widget(block), pane.chunk());
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
    let _percent = if rows == 0 {
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

    let datetime = Spans::from(datetime);
    let block = Block::default().style(Style::default());
    let paragraph = Paragraph::new(datetime).block(block);

    f.render_widget(paragraph, chunks[0]);

    let pane = window.pane("logs").unwrap();
    let log_widget = pane.widget().text();
    let log_status = match log_widget {
        Some(t) => log_status((t.selected(), t.row_size())),
        None => log_status((0, 0)),
    };

    let logs = Spans::from(log_status);
    let block = Block::default().style(Style::default());
    let paragraph = Paragraph::new(logs)
        .block(block)
        .alignment(Alignment::Right);

    f.render_widget(paragraph, chunks[1]);
}

fn draw_context<B: Backend>(f: &mut Frame<B>, chunk: Rect, ctx: &str, ns: &str) {
    let block = Block::default().style(Style::default());

    let text = format!("{}: {}", ns, ctx);
    let spans = Spans::from(text);
    let paragraph = Paragraph::new(spans).block(block);

    f.render_widget(paragraph, chunk);
}

pub fn draw<B: Backend>(f: &mut Frame<B>, window: &mut Window, ctx: &str, ns: &str) {
    let chunks = window.chunks();

    draw_tab(f, &window);

    draw_context(f, chunks[window_layout_index::CONTEXT], ctx, ns);

    draw_panes(f, window.selected_tab());

    draw_status(f, chunks[window_layout_index::STATUSBAR], &window);

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
