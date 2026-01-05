use anyhow::Result;

pub struct SystemClipboard(arboard::Clipboard);

impl SystemClipboard {
    pub fn new(clipboard: arboard::Clipboard) -> Self {
        Self(clipboard)
    }

    #[cfg(target_os = "linux")]
    pub fn set_contents(&mut self, contents: String) -> Result<()> {
        use arboard::SetExtLinux;

        let mut errors = Vec::new();

        if let Err(err) = self
            .0
            .set()
            .clipboard(arboard::LinuxClipboardKind::Clipboard)
            .text(contents.clone())
        {
            errors.push(format!("clipboard: {}", err));
        }

        if let Err(err) = self
            .0
            .set()
            .clipboard(arboard::LinuxClipboardKind::Primary)
            .text(contents)
        {
            errors.push(format!("primary: {}", err));
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Clipboard error: {}", errors.join(", ")))
        }
    }

    #[cfg(not(target_os = "linux"))]
    pub fn set_contents(&mut self, contents: String) -> Result<()> {
        self.0.set_text(contents)?;
        Ok(())
    }
}

impl std::fmt::Debug for SystemClipboard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SystemClipboard").finish()
    }
}
