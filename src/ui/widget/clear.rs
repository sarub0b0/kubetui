use ratatui::{buffer::Buffer, layout::Rect, style::Style, widgets::Widget};

/// Clearウィジェットを拡張し、スタイルを設定できるようにしたウィジェット
pub struct StyledClear {
    style: Style,
}

impl StyledClear {
    pub fn new(style: impl Into<Style>) -> Self {
        Self {
            style: style.into(),
        }
    }
}

impl Widget for StyledClear {
    fn render(self, area: Rect, buf: &mut Buffer) {
        for x in area.left()..area.right() {
            for y in area.top()..area.bottom() {
                buf[(x, y)].reset();

                buf[(x, y)].set_style(self.style);
            }
        }
    }
}
