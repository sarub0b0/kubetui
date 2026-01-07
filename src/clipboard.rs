mod osc52;
mod system;

use anyhow::Result;

use crate::{cmd::ClipboardMode, logger};

use osc52::Osc52Clipboard;
use system::SystemClipboard;

#[derive(Debug)]
pub enum Clipboard {
    System(SystemClipboard),
    Osc52(Osc52Clipboard),
}

impl Clipboard {
    /// Create clipboard based on mode and environment detection
    pub fn new(mode: ClipboardMode) -> Option<Self> {
        match mode {
            ClipboardMode::System => Self::try_system(),
            ClipboardMode::Osc52 => Self::try_osc52(),
            ClipboardMode::Auto => Self::auto_detect(),
        }
    }

    fn try_system() -> Option<Self> {
        arboard::Clipboard::new()
            .inspect_err(|err| {
                logger!(error, "Failed to create system clipboard: {}", err);
            })
            .ok()
            .map(|cb| {
                logger!(info, "Using system clipboard");
                Clipboard::System(SystemClipboard::new(cb))
            })
    }

    fn try_osc52() -> Option<Self> {
        logger!(info, "Using OSC 52 clipboard");
        Some(Clipboard::Osc52(Osc52Clipboard::new()))
    }

    /// Auto detection logic:
    /// 1. If SSH detected -> OSC 52
    /// 2. Else try arboard, fallback to OSC 52
    fn auto_detect() -> Option<Self> {
        if Self::is_ssh_session() {
            logger!(info, "SSH session detected, using OSC 52 clipboard");
            return Self::try_osc52();
        }

        Self::try_system().or_else(|| {
            logger!(info, "System clipboard unavailable, falling back to OSC 52");
            Self::try_osc52()
        })
    }

    fn is_ssh_session() -> bool {
        std::env::var("SSH_CONNECTION").is_ok()
            || std::env::var("SSH_CLIENT").is_ok()
            || std::env::var("SSH_TTY").is_ok()
    }

    pub fn set_contents(&mut self, contents: String) -> Result<()> {
        match self {
            Clipboard::System(cb) => cb.set_contents(contents),
            Clipboard::Osc52(cb) => cb.set_contents(contents),
        }
    }
}
