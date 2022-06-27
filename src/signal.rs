use crossterm::{
    cursor::Show,
    event::DisableMouseCapture,
    execute,
    terminal::{disable_raw_mode, LeaveAlternateScreen},
};
use ctrlc;

pub fn signal_handler() {
    ctrlc::set_handler(|| {
        execute!(
            std::io::stdout(),
            LeaveAlternateScreen,
            DisableMouseCapture,
            Show
        )
        .expect("failed to restore terminal");
        disable_raw_mode().expect("failed to disable raw mode");

        std::process::exit(0);
    })
    .expect("Error setting Ctrl-C handler")
}
