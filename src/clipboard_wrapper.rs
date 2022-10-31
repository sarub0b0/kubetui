use anyhow::Result;

pub enum Clipboard {
    OSC52,
    OS,
}

impl Clipboard {
    pub fn new() -> Self {
        if wsl::is_wsl() {
            Self::OSC52
        } else {
            Self::OS
        }
    }

    pub fn set_contents(&mut self, contents: String) -> Result<()> {
        match self {
            Clipboard::OSC52 => osc52::OSC52Clipboard::set_contents(contents),
            Clipboard::OS => os::OSClipboard::set_contents(contents),
        }
    }
}

mod os {
    use anyhow::Result;
    use arboard::Clipboard;

    pub struct OSClipboard(Clipboard);

    impl OSClipboard {
        pub fn set_contents(contents: String) -> Result<()> {
            let mut c = Clipboard::new()?;
            Ok(c.set_text(contents)?)
        }
    }
}

mod osc52 {
    use anyhow::Result;
    use base64::encode;

    pub struct OSC52Clipboard;

    impl OSC52Clipboard {
        fn is_tmux() -> bool {
            std::env::var("TMUX").is_ok()
        }

        pub fn set_contents(contents: String) -> Result<()> {
            let mut osc52 = format!("\x1b]52;;{}\x07", encode(contents));

            if Self::is_tmux() {
                osc52 = format!("\x1bPtmux;\x1b{}\x1b\x1b\\\x1b\\", osc52);
            }

            print!("{}", osc52);

            Ok(())
        }
    }
}
