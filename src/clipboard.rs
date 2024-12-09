use anyhow::Result;

pub struct Clipboard(arboard::Clipboard);

impl Clipboard {
    pub fn new(clipboard: arboard::Clipboard) -> Self {
        Self(clipboard)
    }

    pub fn set_contents(&mut self, contents: String) -> Result<()> {
        self.0.set_text(contents)?;
        Ok(())
    }
}

impl std::fmt::Debug for Clipboard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Clipboard").finish()
    }
}
