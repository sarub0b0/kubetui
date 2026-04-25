use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph, Wrap},
    Frame,
};

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

/// ウィジェットのブロック（タイトル・ボーダー）内にエラーテキストを描画する。
///
/// - `chunk`: 描画領域
/// - `block`: ウィジェットのタイトル・ボーダーを含む Block
/// - `error_lines`: 表示するエラーテキストの行
/// - `theme`: エラー表示のテーマ
pub fn render_widget_error(
    f: &mut Frame,
    chunk: Rect,
    block: Block,
    error_lines: &[String],
    theme: &ErrorTheme,
) {
    let lines: Vec<Line> = error_lines
        .iter()
        .map(|line| Line::from(Span::styled(line.clone(), theme.style)))
        .collect();

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, chunk);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, buffer::Buffer, widgets::Borders, Terminal};

    #[test]
    fn render_widget_error_draws_lines_with_style() {
        let backend = TestBackend::new(40, 6);
        let mut terminal = Terminal::new(backend).unwrap();

        let theme = ErrorTheme::default();
        let lines = vec!["error line 1".to_string(), "error line 2".to_string()];

        terminal
            .draw(|f| {
                let block = Block::default().borders(Borders::ALL).title("Title");
                render_widget_error(f, f.area(), block, &lines, &theme);
            })
            .unwrap();

        let buffer: &Buffer = terminal.backend().buffer();
        // Block タイトルが描画されていること
        let title_row: String = (0..40)
            .map(|x| buffer[(x, 0)].symbol().to_string())
            .collect();
        assert!(
            title_row.contains("Title"),
            "title not rendered: {title_row}"
        );
        // 最初の行にエラーが描画されていること
        let line1_row: String = (0..40)
            .map(|x| buffer[(x, 1)].symbol().to_string())
            .collect();
        assert!(
            line1_row.contains("error line 1"),
            "error line 1 not rendered: {line1_row}"
        );
    }
}
