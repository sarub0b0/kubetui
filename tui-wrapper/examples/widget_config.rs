use std::io::stdout;

use tui_wrapper::{
    crossterm::{
        event::{read, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    tui::{backend::CrosstermBackend, Terminal},
    util::child_window_chunk,
    widget::config::WidgetConfig,
};

fn main() {
    enable_raw_mode().unwrap();

    execute!(stdout(), EnterAlternateScreen, EnableMouseCapture).unwrap();

    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend).unwrap();

    terminal.clear().unwrap();

    let widget_config = WidgetConfig::builder()
        .title("Title")
        // .disable_focus()
        .build();

    // dbg!(stack);

    let mut focus = true;
    loop {
        terminal
            .draw(|f| {
                let chunk = child_window_chunk(50, 50, f.size());

                let block = widget_config.render_block(focus);

                f.render_widget(block, chunk);
            })
            .unwrap();

        match read() {
            Ok(ev) => match ev {
                Event::Key(key) => match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('j') => focus = !focus,
                    _ => {}
                },
                Event::Mouse(_) => {}
                Event::Resize(_, _) => {}
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
