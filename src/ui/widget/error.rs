use ratatui::style::{Color, Style};

/// エラー表示用のテーマ
#[derive(Debug, Clone)]
pub struct ErrorTheme {
    pub style: Style,
}

impl Default for ErrorTheme {
    fn default() -> Self {
        Self {
            style: Style::default().fg(Color::Red),
        }
    }
}

impl ErrorTheme {
    pub fn style(mut self, style: impl Into<Style>) -> Self {
        self.style = style.into();
        self
    }
}
