use std::io::stdout;

use crossterm::{
    event::{read, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::Widget,
    Terminal,
};

#[cfg(feature = "stack-widget")]
use kubetui::tui_wrapper::{
    util::child_window_chunk,
    widget::{config::WidgetConfig, List, ListBuilder, RenderTrait, Stack, WidgetTrait},
};

fn main() {
    enable_raw_mode().unwrap();

    execute!(stdout(), EnterAlternateScreen, EnableMouseCapture).unwrap();

    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend).unwrap();

    terminal.clear().unwrap();

    let chunk = child_window_chunk(50, 50, terminal.size().unwrap());

    let mut stack = Stack::builder()
        .widget_config(&WidgetConfig::builder().title("Stack").build())
        .build();

    stack.push_widget(
        List::builder()
            .widget_config(&WidgetConfig::builder().title("List-0").build())
            .build(),
    );
    stack.push_widget(
        List::builder()
            .widget_config(&WidgetConfig::builder().title("List-1").build())
            .build(),
    );
    stack.push_widget(
        List::builder()
            .widget_config(&WidgetConfig::builder().title("List-2").build())
            .build(),
    );
    stack.push_widget(
        List::builder()
            .widget_config(&WidgetConfig::builder().title("List-3").build())
            .build(),
    );
    stack.push_widget(
        List::builder()
            .widget_config(&WidgetConfig::builder().title("List-4").build())
            .build(),
    );

    stack.update_chunk(chunk);

    // dbg!(stack);

    loop {
        terminal
            .draw(|f| {
                let chunk = child_window_chunk(50, 50, f.size());
                stack.update_chunk(chunk);
                stack.render(f, true);
            })
            .unwrap();

        match read() {
            Ok(ev) => match ev {
                Event::Key(key) => match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('j') => stack.next_widget(),
                    KeyCode::Char('r') => stack.clear(),
                    _ => {}
                },
                Event::Mouse(_) => {}
                Event::Resize(_, _) => {}
                _ => {}
            },
            Err(_) => break,
        }
    }

    disable_raw_mode().unwrap();
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )
    .unwrap();
}
