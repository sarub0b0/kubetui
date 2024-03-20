use anyhow::Result;

pub struct Clipboard;

impl Clipboard {
    pub fn new() -> Self {
        Self
    }

    pub fn set_contents(&mut self, contents: String) -> Result<()> {
        let mut c = arboard::Clipboard::new()?;
        Ok(c.set_text(contents)?)
    }
}

impl Default for Clipboard {
    fn default() -> Self {
        Self::new()
    }
}
