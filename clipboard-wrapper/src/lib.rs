use std::env;
use std::error::Error;

use base64::encode;
use clipboard::ClipboardContext;

pub use clipboard::ClipboardProvider;

pub struct OSC52ClipboardContext {
    tmux: bool,
}

pub enum ClipboardContextWrapper {
    OSC52(OSC52ClipboardContext),
    OS(ClipboardContext),
}

impl ClipboardProvider for ClipboardContextWrapper {
    fn new() -> Result<Self, Box<(dyn std::error::Error + 'static)>> {
        if wsl::is_wsl() {
            Ok(Self::OSC52(OSC52ClipboardContext::new()?))
        } else {
            Ok(Self::OS(ClipboardContext::new()?))
        }
    }
    fn get_contents(&mut self) -> Result<String, Box<(dyn std::error::Error + 'static)>> {
        match self {
            ClipboardContextWrapper::OSC52(c) => c.get_contents(),
            ClipboardContextWrapper::OS(c) => c.get_contents(),
        }
    }
    fn set_contents(
        &mut self,
        contents: String,
    ) -> Result<(), Box<(dyn std::error::Error + 'static)>> {
        match self {
            ClipboardContextWrapper::OSC52(c) => c.set_contents(contents),
            ClipboardContextWrapper::OS(c) => c.set_contents(contents),
        }
    }
}

impl ClipboardProvider for OSC52ClipboardContext
where
    Self: Sized,
{
    fn new() -> Result<Self, Box<(dyn Error)>> {
        Ok(Self {
            tmux: env::var("TMUX").is_ok(),
        })
    }

    fn get_contents(&mut self) -> Result<String, Box<dyn Error>> {
        Ok(String::default())
    }

    fn set_contents(&mut self, contents: String) -> Result<(), Box<dyn Error>> {
        let mut osc52 = format!("\x1b]52;;{}\x07", encode(contents));

        if self.tmux {
            osc52 = format!("\x1bPtmux;\x1b{}\x1b\x1b\\\x1b\\", osc52);
        }

        print!("{}", osc52);

        Ok(())
    }
}
