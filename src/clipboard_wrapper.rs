use anyhow::Result;

pub enum Clipboard {
    OSC52(osc52::OSC52Clipboard),
    OS(os::OSClipboard),
}

impl Clipboard {
    pub fn new() -> Result<Self> {
        if wsl::is_wsl() {
            Ok(Self::OSC52(osc52::OSC52Clipboard::new()))
        } else {
            Ok(Self::OS(os::OSClipboard::new()?))
        }
    }

    pub fn set_contents(&mut self, contents: String) -> Result<()> {
        match self {
            Clipboard::OSC52(c) => {
                c.set_contents(contents);
                Ok(())
            }
            Clipboard::OS(c) => {
                #[cfg(windows)]
                let mut c = os::OSClipboard::new()?;

                c.set_contents(contents)
            }
        }
    }
}

mod os {
    use anyhow::Result;
    use arboard::Clipboard;

    pub struct OSClipboard(Clipboard);

    impl OSClipboard {
        pub fn new() -> Result<Self> {
            Ok(Self(Clipboard::new()?))
        }

        pub fn set_contents(&mut self, contents: String) -> Result<()> {
            Ok(self.0.set_text(contents)?)
        }
    }
}

mod osc52 {
    use base64::encode;
    use std::env;

    pub struct OSC52Clipboard {
        tmux: bool,
    }

    impl OSC52Clipboard {
        pub fn new() -> Self {
            Self {
                tmux: env::var("TMUX").is_ok(),
            }
        }

        pub fn set_contents(&mut self, contents: String) {
            let mut osc52 = format!("\x1b]52;;{}\x07", encode(contents));

            if self.tmux {
                osc52 = format!("\x1bPtmux;\x1b{}\x1b\x1b\\\x1b\\", osc52);
            }

            print!("{}", osc52);
        }
    }

    impl Default for OSC52Clipboard {
        fn default() -> Self {
            Self::new()
        }
    }
}
