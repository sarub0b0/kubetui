use std::io::{self, Write};

use anyhow::Result;
use base64::Engine;

#[derive(Debug)]
pub struct Osc52Clipboard {
    terminal_type: TerminalType,
}

#[derive(Debug, Clone, Copy)]
enum TerminalType {
    Tmux,
    Screen,
    Standard,
}

impl TerminalType {
    fn detect() -> Self {
        // Check for tmux first (TMUX environment variable)
        if std::env::var("TMUX").is_ok() {
            return Self::Tmux;
        }

        // Check for screen (TERM starts with "screen")
        if let Ok(term) = std::env::var("TERM") {
            if term.starts_with("screen") {
                return Self::Screen;
            }
        }

        Self::Standard
    }
}

impl Osc52Clipboard {
    pub fn new() -> Self {
        Self {
            terminal_type: TerminalType::detect(),
        }
    }

    pub fn set_contents(&mut self, contents: String) -> Result<()> {
        let encoded = base64::engine::general_purpose::STANDARD.encode(contents.as_bytes());

        // OSC 52 sequence: \x1b]52;c;<base64-data>\x07
        // c = clipboard selection (system clipboard)
        let osc52 = format!("\x1b]52;c;{}\x07", encoded);

        let sequence = match self.terminal_type {
            // tmux: wrap in DCS Ptmux sequence
            // All ESC characters inside DCS must be doubled
            TerminalType::Tmux => {
                let escaped = osc52.replace('\x1b', "\x1b\x1b");
                format!("\x1bPtmux;{}\x1b\\", escaped)
            }
            // screen: wrap in DCS sequence
            // All ESC characters inside DCS must be doubled
            TerminalType::Screen => {
                let escaped = osc52.replace('\x1b', "\x1b\x1b");
                format!("\x1bP{}\x1b\\", escaped)
            }
            // Standard terminal: use OSC 52 directly
            TerminalType::Standard => osc52,
        };

        let mut stdout = io::stdout();
        stdout.write_all(sequence.as_bytes())?;
        stdout.flush()?;

        Ok(())
    }
}
