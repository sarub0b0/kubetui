use arboard::SetExtLinux;

pub struct Clipboard(arboard::Clipboard);

impl Clipboard {
    pub fn new(clipboard: arboard::Clipboard) -> Self {
        Self(clipboard)
    }

    pub fn set_contents(&mut self, contents: String) -> Result<(), ClipboardError> {
        let mut error = ClipboardError::new();

        if let Err(err) = self
            .0
            .set()
            .clipboard(arboard::LinuxClipboardKind::Clipboard)
            .text(contents.clone())
        {
            error.clipboard = Some(err);
        }

        if let Err(err) = self
            .0
            .set()
            .clipboard(arboard::LinuxClipboardKind::Primary)
            .text(contents)
        {
            error.primary = Some(err);
        }

        if error.has_errors() {
            Err(error)
        } else {
            Ok(())
        }
    }
}

impl std::fmt::Debug for Clipboard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("arboard::Clipboard").finish()
    }
}

#[derive(Debug)]
pub struct ClipboardError {
    primary: Option<arboard::Error>,
    clipboard: Option<arboard::Error>,
}

impl ClipboardError {
    pub fn new() -> Self {
        Self {
            primary: None,
            clipboard: None,
        }
    }

    pub fn has_errors(&self) -> bool {
        self.primary.is_some() || self.clipboard.is_some()
    }

    pub fn to_error_message(&self) -> String {
        let mut message = Vec::new();

        if let Some(err) = &self.primary {
            message.push(format!("primary selection error: {}", err));
        }

        if let Some(err) = &self.clipboard {
            message.push(format!("clipboard selection error: {}", err));
        }

        message.join(", ")
    }
}

impl std::fmt::Display for ClipboardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.has_errors() {
            write!(f, "Clipboard Error: {}", self.to_error_message())
        } else {
            write!(f, "No errors")
        }
    }
}

impl std::error::Error for ClipboardError {}
