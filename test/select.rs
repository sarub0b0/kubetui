use std::io::{self, Write};

use crossterm::{
    cursor::Show,
    event::{poll, read, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use tui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::*,
    text::Spans,
    widgets::{Block, Borders},
    Terminal,
};

use std::time::{Duration, Instant};
use tui_wrapper::select::*;

fn main() {
    enable_raw_mode().unwrap();
    execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture).unwrap();

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.clear().unwrap();

    let mut select = Select::new("select", "Select").block(Block::default().borders(Borders::ALL));
    select.set_items(
        vec!["abc", "abd", "acd", "hoge", "fuga"]
            .iter()
            .map(ToString::to_string)
            .collect(),
    );

    // let mut state = SelectState::default();

    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_millis(200);

    loop {
        terminal
            .draw(|f| {
                let h = 40;
                let w = 60;
                let chunk = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Percentage((100 - h) / 2),
                        Constraint::Percentage(h),
                        Constraint::Percentage((100 - h) / 2),
                    ])
                    .split(f.size());

                let chunk = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage((100 - w) / 2),
                        Constraint::Percentage(w),
                        Constraint::Percentage((100 - w) / 2),
                    ])
                    .split(chunk[1])[1];
                select.update_chunk(chunk);
                select.render(f);

                // let chunk = Layout::default()
                //     .constraints([Constraint::Percentage(100)])
                //     .margin(20)
                //     .split(f.size());

                // let block = Block::default()
                //     .borders(Borders::ALL)
                //     .border_style(Style::default().fg(Color::Gray))
                //     .alignment(Alignment::Left)
                //     .title_offset(1)
                //     .title("Select");

                // let items: Vec<SelectItem> = ["Item", "Item", "Item"]
                //     .iter()
                //     .map(|item| SelectItem::new(item.to_string()))
                //     .collect();

                // let select = Select::new(items).block(block);

                // f.render_stateful_widget(select, chunk[0], &mut state);
            })
            .unwrap();

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if poll(timeout).unwrap() {
            match read().unwrap() {
                Event::Key(key) => match key.code {
                    KeyCode::Char('q') if key.modifiers == KeyModifiers::CONTROL => break,
                    KeyCode::Char('n') if key.modifiers == KeyModifiers::CONTROL => {
                        select.select_next_item();
                    }
                    KeyCode::Char('p') if key.modifiers == KeyModifiers::CONTROL => {
                        select.select_prev_item();
                    }
                    KeyCode::Delete | KeyCode::Char('h')
                        if key.modifiers == KeyModifiers::CONTROL =>
                    {
                        select.remove_char();
                    }
                    KeyCode::Char(c) => {
                        select.insert_char(c);
                    }

                    KeyCode::Right => {
                        select.forward_cursor();
                    }
                    KeyCode::Left => {
                        select.back_cursor();
                    }
                    KeyCode::Tab => {
                        select.toggle_focus();
                    }
                    KeyCode::Enter => {
                        select.toggle_select_unselect();
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }
    execute!(
        io::stdout(),
        LeaveAlternateScreen,
        DisableMouseCapture,
        Show
    )
    .unwrap();
    disable_raw_mode().unwrap();
}
