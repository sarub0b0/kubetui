use tui::style::{Color, Style};
use tui::text::{Span, Spans};
use tui::widgets::{Block, Paragraph};

use super::WidgetTrait;

const BORDER_WIDTH: usize = 2;

pub struct Text<'a> {
    items: Vec<String>,
    state: TextState,
    spans: Vec<Spans<'a>>,
    row_size: u16,
}

#[derive(Clone, Copy)]
pub struct TextState {
    scroll: u16,
}

impl TextState {
    fn select(&mut self, index: u16) {
        self.scroll = index;
    }
    fn selected(&self) -> u16 {
        self.scroll
    }
}

impl Default for TextState {
    fn default() -> Self {
        Self { scroll: 0 }
    }
}

// ステート
impl Text<'_> {
    pub fn new(items: Vec<String>) -> Self {
        Self {
            items,
            state: TextState::default(),
            spans: vec![Spans::default()],
            row_size: 0,
        }
    }

    pub fn select(&mut self, scroll: u16) {
        self.state.select(scroll);
    }

    pub fn state(&self) -> TextState {
        self.state
    }

    pub fn selected(&self) -> u16 {
        self.state.selected()
    }
    pub fn scroll_top(&mut self) {
        self.state.select(0);
    }

    pub fn scroll_bottom(&mut self) {
        self.state.select(self.row_size);
    }

    pub fn is_bottom(&self) -> bool {
        self.selected() == self.row_size
    }

    pub fn scroll_down(&mut self, index: u16) {
        (0..index).for_each(|_| self.select_next(1));
    }

    pub fn scroll_up(&mut self, index: u16) {
        (0..index).for_each(|_| self.select_prev(1));
    }
}

impl Default for Text<'_> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            state: TextState::default(),
            spans: Vec::new(),
            row_size: 0,
        }
    }
}

// コンテンツ操作
impl<'a> Text<'a> {
    pub fn items(&self) -> &Vec<String> {
        &self.items
    }

    pub fn add_item(&mut self, item: impl Into<String>) {
        self.items.push(item.into());
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn spans(&self) -> &Vec<Spans> {
        &self.spans
    }

    pub fn widget(&self, block: Block<'a>) -> Paragraph<'a> {
        Paragraph::new(self.spans.clone())
            .block(block)
            .style(Style::default())
            .scroll((self.selected(), 0))
    }

    pub fn row_size(&self) -> u16 {
        self.row_size
    }

    pub fn update_spans(&mut self, width: u16) {
        let w = width as usize - BORDER_WIDTH;
        let lines = wrap(&self.items, w);
        self.spans = generate_spans(&lines);
    }

    pub fn update_rows_size(&mut self, height: u16) {
        let mut count = self.spans.len() as u16;

        let height = height - BORDER_WIDTH as u16; // 2: border-line

        if height < count {
            count -= height;
        } else {
            count = 0
        }

        self.row_size = count;
    }

    pub fn update_span(&mut self, width: u16) {
        let w = width as usize - BORDER_WIDTH;
        let lines = wrap_line(&self.items[self.items.len() - 1], w);

        self.spans.append(&mut generate_spans(&lines));
    }
}

impl WidgetTrait for Text<'_> {
    fn selectable(&self) -> bool {
        true
    }

    fn select_next(&mut self, index: usize) {
        let mut i = self.state.selected();

        if self.row_size <= i {
            i = self.row_size;
        } else {
            i = i + index as u16;
        }

        self.state.select(i);
    }

    fn select_prev(&mut self, index: usize) {
        let mut i = self.state.selected();
        if i == 0 {
            i = 0;
        } else {
            i = i - index as u16;
        }
        self.state.select(i);
    }

    fn select_first(&mut self) {
        self.state.select(0);
    }
    fn select_last(&mut self) {
        self.state.select(self.row_size);
    }

    fn set_items(&mut self, items: Vec<String>) {
        self.state.select(0);
        self.items = items.clone();
    }
}

fn style(num: &str) -> Style {
    let color = match num {
        "30" => Color::Black,
        "31" => Color::Red,
        "32" => Color::Green,
        "33" => Color::Yellow,
        "34" => Color::Blue,
        "35" => Color::Magenta,
        "36" => Color::Cyan,
        "37" => Color::White,
        "39" => Color::Reset,
        "90" => Color::DarkGray,
        "91" => Color::LightRed,
        "92" => Color::LightGreen,
        "93" => Color::LightYellow,
        "94" => Color::LightBlue,
        "95" => Color::LightMagenta,
        "96" => Color::LightCyan,
        "97" => Color::Gray,
        _ => Color::Reset,
    };

    Style::default().fg(color)
}

fn wrap(lines: &Vec<String>, width: usize) -> Vec<String> {
    let mut ret = Vec::new();

    for line in lines.iter() {
        ret.append(&mut wrap_line(line, width));
    }

    ret
}

fn wrap_line(text: &String, width: usize) -> Vec<String> {
    let mut ret = Vec::new();
    if text.len() == 0 {
        ret.push("".to_string());
        return ret;
    }

    let lines = text.lines();

    for l in lines {
        let len = l.chars().count();
        let tmp = l;
        if width < len {
            let crs = len / width;
            for i in 0..=crs {
                let start = width * i;
                let mut end = width * (i + 1);

                if len <= end {
                    end = len
                }

                ret.push(String::from(&tmp[start..end]));
            }
        } else {
            ret.push(l.to_string());
        }
    }
    ret
}

fn generate_spans<'a>(lines: &Vec<String>) -> Vec<Spans<'a>> {
    lines
        .iter()
        .cloned()
        .map(|t| {
            let mut start = 0;
            let mut end = 0;
            let mut found = false;

            let mut spans: Vec<Span> = vec![];

            while let Some(i) = t[start..].find("\x1b[") {
                found = true;
                start = i + 5 + end;

                let (c0, c1) = (i + 2 + end, i + 4 + end);

                if let Some(next) = t[start..].find("\x1b[") {
                    end = next + start;
                } else {
                    end = t.len();
                }
                spans.push(Span::styled(
                    String::from(&t[start..end]),
                    style(&t[c0..c1]),
                ));

                start = end;
            }

            if found == false {
                Spans::from(t.clone())
            } else {
                Spans::from(spans)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_only_newline() {
        let text = vec!["hoge".to_string(), "".to_string(), "hoge".to_string()];

        assert_eq!(
            wrap(&text, 100),
            vec!["hoge".to_string(), "".to_string(), "hoge".to_string()]
        );
    }

    #[test]
    fn wrap_short() {
        let text = vec!["aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string()];

        assert_eq!(
            wrap(&text, 100),
            vec!["aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string()]
        );
    }

    #[test]
    fn wrap_long() {
        let text = vec!["aaaaaaaaaaaaaaa".to_string()];

        assert_eq!(
            wrap(&text, 10),
            vec!["aaaaaaaaaa".to_string(), "aaaaa".to_string()]
        );
    }
    #[test]
    fn wrap_too_long_0() {
        let text = vec!["aaaaaaaaaaaaaa".to_string()];

        assert_eq!(
            wrap(&text, 5),
            vec!["aaaaa".to_string(), "aaaaa".to_string(), "aaaa".to_string(),]
        );
    }

    #[test]
    fn wrap_too_long_1() {
        let text = vec!["123456789\n123456789\n123456789\n123456789\n123456789\n123456789\n123456789\n123456789\n".to_string()];

        assert_eq!(
            wrap(&text, 12),
            vec![
                "123456789".to_string(),
                "123456789".to_string(),
                "123456789".to_string(),
                "123456789".to_string(),
                "123456789".to_string(),
                "123456789".to_string(),
                "123456789".to_string(),
                "123456789".to_string(),
            ]
        );
    }

    #[test]
    fn spans() {
        let text = vec![
            "> taskbox@0.1.0 start /app",
            "> react-scripts start",
            "",
            "\x1b[34mℹ\x1b[39m \x1b[90m｢wds｣\x1b[39m: Project is running at http://10.1.157.9/",
            "\x1b[34mℹ\x1b[39m \x1b[90m｢wds｣\x1b[39m: webpack output is served from",
            "\x1b[34mℹ\x1b[39m \x1b[90m｢wds｣\x1b[39m: Content not from webpack is served from /app/public",
            "\x1b[34mℹ\x1b[39m \x1b[90m｢wds｣\x1b[39m: 404s will fallback to /",
            "Starting the development server...",
            "",
            "Compiled successfully!",
            "",
            "You can now view taskbox in the browser.",
            "",
            "  Local:            http://localhost:3000",
            "  On Your Network:  http://10.1.157.9:3000",
            "",
            "Note that the development build is not optimized.",
            "To create a production build, use npm run build.",
        ];

        let wrapped = wrap(&text.iter().cloned().map(String::from).collect(), 100);

        let expected = vec![
            Spans::from("> taskbox@0.1.0 start /app"),
            Spans::from("> react-scripts start"),
            Spans::from(""),
            Spans::from(vec![
                Span::styled("ℹ", Style::default().fg(Color::Blue)),
                Span::styled(" ", Style::default().fg(Color::Reset)),
                Span::styled("｢wds｣", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    ": Project is running at http://10.1.157.9/",
                    Style::default().fg(Color::Reset),
                ),
            ]),
            Spans::from(vec![
                Span::styled("ℹ", Style::default().fg(Color::Blue)),
                Span::styled(" ", Style::default().fg(Color::Reset)),
                Span::styled("｢wds｣", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    ": webpack output is served from",
                    Style::default().fg(Color::Reset),
                ),
            ]),
            Spans::from(vec![
                Span::styled("ℹ", Style::default().fg(Color::Blue)),
                Span::styled(" ", Style::default().fg(Color::Reset)),
                Span::styled("｢wds｣", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    ": Content not from webpack is served from /app/public",
                    Style::default().fg(Color::Reset),
                ),
            ]),
            Spans::from(vec![
                Span::styled("ℹ", Style::default().fg(Color::Blue)),
                Span::styled(" ", Style::default().fg(Color::Reset)),
                Span::styled("｢wds｣", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    ": 404s will fallback to /",
                    Style::default().fg(Color::Reset),
                ),
            ]),
            Spans::from("Starting the development server..."),
            Spans::from(""),
            Spans::from("Compiled successfully!"),
            Spans::from(""),
            Spans::from("You can now view taskbox in the browser."),
            Spans::from(""),
            Spans::from("  Local:            http://localhost:3000"),
            Spans::from("  On Your Network:  http://10.1.157.9:3000"),
            Spans::from(""),
            Spans::from("Note that the development build is not optimized."),
            Spans::from("To create a production build, use npm run build."),
        ];

        let result = generate_spans(&wrapped);
        for (i, l) in result.iter().enumerate() {
            assert_eq!(*l, expected[i]);
        }
    }
}
