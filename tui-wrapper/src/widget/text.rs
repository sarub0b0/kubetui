use tui::layout::Rect;
use tui::style::Style;
use tui::text::{Span, Spans};
use tui::widgets::Paragraph;

use super::ansi::*;
use super::WidgetTrait;

use ansi::{self, AnsiEscapeSequence, TextParser};

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use rayon::prelude::*;

#[derive(Debug, Clone, Copy)]
struct TRect {
    width: usize,
    height: usize,
}

impl TRect {
    fn new(rect: Rect) -> Self {
        Self {
            width: rect.width as usize,
            height: rect.height as usize,
        }
    }
}

impl Default for TRect {
    fn default() -> Self {
        Self {
            width: 0,
            height: 0,
        }
    }
}

#[derive(Debug)]
pub struct Text<'a> {
    items: Vec<String>,
    state: TextState,
    spans: Vec<Spans<'a>>,
    row_size: u64,
    area: TRect,
}

#[derive(Debug, Clone, Copy)]
pub struct TextState {
    scroll: u64,
}

impl TextState {
    fn select(&mut self, index: u64) {
        self.scroll = index;
    }
    fn selected(&self) -> u64 {
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
            area: TRect::default(),
        }
    }

    pub fn select(&mut self, scroll: u64) {
        self.state.select(scroll);
    }

    pub fn state(&self) -> TextState {
        self.state
    }

    pub fn selected(&self) -> u64 {
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

    pub fn scroll_down(&mut self, index: u64) {
        (0..index).for_each(|_| self.select_next(1));
    }

    pub fn scroll_up(&mut self, index: u64) {
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
            area: TRect::default(),
        }
    }
}

// コンテンツ操作
impl<'a> Text<'a> {
    pub fn items(&self) -> &Vec<String> {
        &self.items
    }

    pub fn clear(&mut self) {
        let area = self.area;
        *self = Self::default();
        self.area = area;
    }

    pub fn spans(&self) -> &Vec<Spans> {
        &self.spans
    }

    pub fn widget(&self) -> Paragraph<'a> {
        let start = self.state.selected() as usize;

        let end = if self.spans.len() < self.area.height {
            self.spans.len()
        } else {
            start + self.area.height
        };

        Paragraph::new(self.spans[start..end].to_vec()).style(Style::default())
    }

    pub fn row_size(&self) -> u64 {
        self.row_size
    }

    pub fn append_items(&mut self, items: &Vec<String>) {
        self.items.append(&mut items.clone());

        let wrapped = wrap(items, self.area.width);

        self.spans.append(&mut generate_spans(&wrapped));

        self.update_rows_size();
    }

    fn update_spans(&mut self) {
        let lines = wrap(&self.items, self.area.width);

        self.spans = generate_spans(&lines);
    }

    fn update_rows_size(&mut self) {
        let mut count = self.spans.len() as u64;

        let height = self.area.height as u64; // 2: border-line

        if height < count {
            count -= height;
        } else {
            count = 0
        }

        self.row_size = count;
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
            i = i + index as u64;
        }

        self.state.select(i);
    }

    fn select_prev(&mut self, index: usize) {
        let mut i = self.state.selected();
        if i == 0 {
            i = 0;
        } else {
            i = i - index as u64;
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

        let wrapped = wrap(&items, self.area.width);

        self.spans.append(&mut generate_spans(&wrapped));

        self.update_rows_size();
    }

    fn update_area(&mut self, area: Rect) {
        self.area = TRect::new(area);

        self.update_spans();
        self.update_rows_size();
    }
}

pub fn wrap(lines: &Vec<String>, wrap_width: usize) -> Vec<String> {
    lines
        .par_iter()
        .map(|line| wrap_line(line, wrap_width))
        .flatten()
        .collect()
}

fn wrap_line(text: &String, wrap_width: usize) -> Vec<String> {
    if text.len() == 0 {
        return vec!["".to_string()];
    }

    text.lines()
        .map(|line| {
            if wrap_width < line.width() {
                wrap_one_line(line, wrap_width)
            } else {
                vec![line.to_string()]
            }
        })
        .flatten()
        .collect()
}

fn wrap_one_line(line: &str, wrap_width: usize) -> Vec<String> {
    let mut ret = Vec::new();
    let mut iter = line.ansi_parse();

    let mut buf = String::with_capacity(line.len());
    let mut sum_width = 0;

    while let Some(parsed) = iter.next() {
        if parsed.ty == AnsiEscapeSequence::Chars {
            let parsed_width = parsed.chars.width();

            if wrap_width <= sum_width + parsed_width {
                let graphemes: Vec<&str> = parsed.chars.graphemes(true).collect();

                graphemes.iter().for_each(|c| {
                    sum_width += c.width();

                    if wrap_width <= sum_width {
                        if wrap_width == sum_width {
                            buf += c;
                            ret.push(buf.to_string());
                            buf.clear();

                            sum_width = 0;
                        } else {
                            ret.push(buf.to_string());
                            buf.clear();

                            buf += c;
                            sum_width = c.width();
                        }
                    } else {
                        buf += c;
                    }
                });
            } else {
                buf += parsed.chars;
                sum_width += parsed_width;
            }
        } else {
            buf += parsed.chars;
        }
    }

    if !buf.is_empty() {
        if 0 < sum_width {
            ret.push(buf);
        } else {
            if let Some(last) = ret.last_mut() {
                *last += &buf;
            } else {
                ret.push(buf);
            }
        }
    }

    ret
}

pub fn generate_spans<'a>(lines: &Vec<String>) -> Vec<Spans<'a>> {
    lines
        .par_iter()
        .map(|line| {
            if line.is_empty() {
                return Spans::from(Span::styled("", Style::default()));
            }
            let mut span_vec: Vec<Span> = vec![];
            let mut style = Style::default();

            let mut iter = line.ansi_parse().peekable();

            while let Some(parsed) = iter.next() {
                match parsed.ty {
                    AnsiEscapeSequence::Chars => {
                        span_vec.push(Span::styled(parsed.chars.to_string(), style));
                    }
                    AnsiEscapeSequence::SelectGraphicRendition(color) => {
                        style = generate_style_from_ansi_color(color);
                        if let None = iter.peek() {
                            span_vec.push(Span::styled("", style));
                        }
                    }
                    _ => {}
                }
            }

            Spans::from(span_vec)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use tui::style::{Color, Modifier, Style};

    mod one_line {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn contains_escape_sequence() {
            let text = "\x1b[1Aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\x1b[1A\x1b[1A";

            assert_eq!(
                wrap_one_line(text, 10),
                vec![
                    "\x1b[1Aaaaaaaaaaa".to_string(),
                    "aaaaaaaaaa".to_string(),
                    "aaaaaaaaaa\x1b[1A\x1b[1A".to_string(),
                ]
            );

            let text =
                "\x1b[34mℹ\x1b[39m \x1b[90m｢wds｣\x1b[39m: Project is running at http://10.1.157.9/";

            assert_eq!(
                wrap_one_line(text, 40),
                vec![
                    "\x1b[34mℹ\x1b[39m \x1b[90m｢wds｣\x1b[39m: Project is running at http://10"
                        .to_string(),
                    ".1.157.9/".to_string(),
                ]
            );
        }

        #[test]
        fn text_only() {
            let text = vec!["ℹ ｢wds｣: Project is running at http://10.1.157.45/".to_string()];
            assert_eq!(
                wrap(&text, 40),
                vec![
                    "ℹ ｢wds｣: Project is running at http://10".to_string(),
                    ".1.157.45/".to_string(),
                ]
            );
        }

        #[test]
        fn wrap_japanese() {
            let text = "あいうえおかきくけこさしすせそ";

            assert_eq!(
                wrap_one_line(text, 10),
                vec![
                    "あいうえお".to_string(),
                    "かきくけこ".to_string(),
                    "さしすせそ".to_string(),
                ]
            );

            assert_eq!(
                wrap_one_line(text, 9),
                vec![
                    "あいうえ".to_string(),
                    "おかきく".to_string(),
                    "けこさし".to_string(),
                    "すせそ".to_string(),
                ]
            );
        }
    }

    mod wrap {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn unwrap() {
            let text = vec!["aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string()];

            assert_eq!(
                wrap(&text, 100),
                vec!["aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string()]
            );
        }

        #[test]
        fn unwrap_contains_escape_sequence() {
            let text = vec!["\x1b[1Aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\x1b[1A\x1b[1A".to_string()];

            assert_eq!(
                wrap(&text, 30),
                vec!["\x1b[1Aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\x1b[1A\x1b[1A".to_string()]
            );
        }
        #[test]
        fn contains_newline_string() {
            let text = vec!["hoge".to_string(), "".to_string(), "hoge".to_string()];

            assert_eq!(
                wrap(&text, 100),
                vec!["hoge".to_string(), "".to_string(), "hoge".to_string()]
            );
        }

        #[test]
        fn short() {
            let text = vec!["aaaaaaaaaaaaaaa".to_string()];

            assert_eq!(
                wrap(&text, 10),
                vec!["aaaaaaaaaa".to_string(), "aaaaa".to_string()]
            );
            assert_eq!(
                wrap(&text, 5),
                vec![
                    "aaaaa".to_string(),
                    "aaaaa".to_string(),
                    "aaaaa".to_string(),
                ]
            );
        }

        #[test]
        fn string_contains_newline() {
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

        let wrapped = wrap(&text.iter().cloned().map(String::from).collect(), 40);

        let expected = vec![
            Spans::from("> taskbox@0.1.0 start /app"),
            Spans::from("> react-scripts start"),
            Spans::from(""),
            Spans::from(vec![
                Span::styled("ℹ", Style::default().fg(Color::Blue)),
                Span::styled(" ", Style::default().fg(Color::Reset)),
                Span::styled("｢wds｣", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    ": Project is running at http://10",
                    Style::default().fg(Color::Reset),
                ),
            ]),
            Spans::from(".1.157.9/"),
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
                    ": Content not from webpack is ser",
                    Style::default().fg(Color::Reset),
                ),
            ]),
            Spans::from("ved from /app/public"),
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
            Spans::from("  Local:            http://localhost:300"),
            Spans::from("0"),
            Spans::from("  On Your Network:  http://10.1.157.9:30"),
            Spans::from("00"),
            Spans::from(""),
            Spans::from("Note that the development build is not o"),
            Spans::from("ptimized."),
            Spans::from("To create a production build, use npm ru"),
            Spans::from("n build."),
        ];

        let result = generate_spans(&wrapped);
        for (i, l) in result.iter().enumerate() {
            assert_eq!(*l, expected[i]);
        }
    }

    mod generate_spans_color {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn color_3_4bit_fg() {
            let text = vec!["hoge\x1b[33mhoge\x1b[39m".to_string()];

            assert_eq!(
                generate_spans(&text),
                vec![Spans::from(vec![
                    Span::raw("hoge"),
                    Span::styled("hoge", Style::default().fg(Color::Yellow)),
                    Span::styled("", Style::default().fg(Color::Reset)),
                ])]
            )
        }

        #[test]
        fn color_3_4bit_fg_bold() {
            let text = vec!["\x1b[1;33mhoge\x1b[39m".to_string()];

            assert_eq!(
                generate_spans(&text),
                vec![Spans::from(vec![
                    Span::styled(
                        "hoge",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    ),
                    Span::styled("", Style::default().fg(Color::Reset)),
                ])]
            )
        }

        #[test]
        fn color_8bit_fg() {
            let text = vec!["\x1b[38;5;33mhoge\x1b[39m".to_string()];

            assert_eq!(
                generate_spans(&text),
                vec![Spans::from(vec![
                    Span::styled("hoge", Style::default().fg(Color::Indexed(33))),
                    Span::styled("", Style::default().fg(Color::Reset)),
                ])]
            )
        }

        #[test]
        fn color_8bit_fg_bold() {
            let text = vec!["\x1b[1;38;5;33mhoge\x1b[39m".to_string()];

            assert_eq!(
                generate_spans(&text),
                vec![Spans::from(vec![
                    Span::styled(
                        "hoge",
                        Style::default()
                            .fg(Color::Indexed(33))
                            .add_modifier(Modifier::BOLD)
                    ),
                    Span::styled("", Style::default().fg(Color::Reset)),
                ])]
            )
        }

        #[test]
        fn color_24bit_fg() {
            let text = vec!["\x1b[38;2;33;10;10mhoge\x1b[39m".to_string()];

            assert_eq!(
                generate_spans(&text),
                vec![Spans::from(vec![
                    Span::styled("hoge", Style::default().fg(Color::Rgb(33, 10, 10))),
                    Span::styled("", Style::default().fg(Color::Reset)),
                ])]
            )
        }

        #[test]
        fn color_24bit_fg_bold() {
            let text = vec!["\x1b[1;38;2;33;10;10mhoge\x1b[39m".to_string()];

            assert_eq!(
                generate_spans(&text),
                vec![Spans::from(vec![
                    Span::styled(
                        "hoge",
                        Style::default()
                            .fg(Color::Rgb(33, 10, 10))
                            .add_modifier(Modifier::BOLD)
                    ),
                    Span::styled("", Style::default().fg(Color::Reset)),
                ])]
            )
        }

        #[test]
        fn color_3_4bit_bg() {
            let text = vec!["\x1b[43mhoge\x1b[49m".to_string()];

            assert_eq!(
                generate_spans(&text),
                vec![Spans::from(vec![
                    Span::styled("hoge", Style::default().bg(Color::Yellow)),
                    Span::styled("", Style::default().bg(Color::Reset)),
                ])]
            )
        }

        #[test]
        fn color_3_4bit_bg_bold() {
            let text = vec!["\x1b[1;43mhoge\x1b[49m".to_string()];

            assert_eq!(
                generate_spans(&text),
                vec![Spans::from(vec![
                    Span::styled(
                        "hoge",
                        Style::default()
                            .bg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    ),
                    Span::styled("", Style::default().bg(Color::Reset)),
                ])]
            );

            let text = vec!["\x1b[43;1mhoge\x1b[49m".to_string()];

            assert_eq!(
                generate_spans(&text),
                vec![Spans::from(vec![
                    Span::styled(
                        "hoge",
                        Style::default()
                            .bg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    ),
                    Span::styled("", Style::default().bg(Color::Reset)),
                ])]
            );
        }

        #[test]
        fn color_8bit_bg() {
            let text = vec!["\x1b[48;5;33mhoge\x1b[49m".to_string()];

            assert_eq!(
                generate_spans(&text),
                vec![Spans::from(vec![
                    Span::styled("hoge", Style::default().bg(Color::Indexed(33))),
                    Span::styled("", Style::default().bg(Color::Reset)),
                ])]
            );
        }

        #[test]
        fn color_8bit_bg_bold() {
            let text = vec!["\x1b[1;48;5;33mhoge\x1b[49m".to_string()];

            assert_eq!(
                generate_spans(&text),
                vec![Spans::from(vec![
                    Span::styled(
                        "hoge",
                        Style::default()
                            .bg(Color::Indexed(33))
                            .add_modifier(Modifier::BOLD)
                    ),
                    Span::styled("", Style::default().bg(Color::Reset)),
                ])]
            );

            let text = vec!["\x1b[48;5;33;1mhoge\x1b[49m".to_string()];

            assert_eq!(
                generate_spans(&text),
                vec![Spans::from(vec![
                    Span::styled(
                        "hoge",
                        Style::default()
                            .bg(Color::Indexed(33))
                            .add_modifier(Modifier::BOLD)
                    ),
                    Span::styled("", Style::default().bg(Color::Reset)),
                ])]
            );
        }

        #[test]
        fn color_24bit_bg() {
            let text = vec!["\x1b[48;2;33;10;10mhoge\x1b[49m".to_string()];

            assert_eq!(
                generate_spans(&text),
                vec![Spans::from(vec![
                    Span::styled("hoge", Style::default().bg(Color::Rgb(33, 10, 10))),
                    Span::styled("", Style::default().bg(Color::Reset)),
                ])]
            );
        }

        #[test]
        fn color_24bit_bg_bold() {
            let text = vec!["\x1b[1;48;2;33;10;10mhoge\x1b[49m".to_string()];

            assert_eq!(
                generate_spans(&text),
                vec![Spans::from(vec![
                    Span::styled(
                        "hoge",
                        Style::default()
                            .bg(Color::Rgb(33, 10, 10))
                            .add_modifier(Modifier::BOLD)
                    ),
                    Span::styled("", Style::default().bg(Color::Reset)),
                ])]
            );

            let text = vec!["\x1b[48;2;33;10;10;1mhoge\x1b[49m".to_string()];

            assert_eq!(
                generate_spans(&text),
                vec![Spans::from(vec![
                    Span::styled(
                        "hoge",
                        Style::default()
                            .bg(Color::Rgb(33, 10, 10))
                            .add_modifier(Modifier::BOLD)
                    ),
                    Span::styled("", Style::default().bg(Color::Reset)),
                ])]
            );
        }

        #[test]
        fn color_24bit_rainbow() {
            let rainbow = vec!["[48;2;0;0;0m [48;2;1;0;0m [48;2;2;0;0m [48;2;3;0;0m [48;2;4;0;0m [48;2;5;0;0m [48;2;6;0;0m [48;2;7;0;0m [48;2;8;0;0m [48;2;9;0;0m [48;2;10;0;0m [0m".to_string()];

            let wrapped = wrap(&rainbow, 3);

            assert_eq!(
                wrapped,
                vec![
                    "[48;2;0;0;0m [48;2;1;0;0m [48;2;2;0;0m ".to_string(),
                    "[48;2;3;0;0m [48;2;4;0;0m [48;2;5;0;0m ".to_string(),
                    "[48;2;6;0;0m [48;2;7;0;0m [48;2;8;0;0m ".to_string(),
                    "[48;2;9;0;0m [48;2;10;0;0m [0m".to_string(),
                ]
            );

            assert_eq!(
                generate_spans(&wrapped),
                vec![
                    Spans::from(vec![
                        Span::styled(" ", Style::default().bg(Color::Rgb(0, 0, 0))),
                        Span::styled(" ", Style::default().bg(Color::Rgb(1, 0, 0))),
                        Span::styled(" ", Style::default().bg(Color::Rgb(2, 0, 0)))
                    ]),
                    Spans::from(vec![
                        Span::styled(" ", Style::default().bg(Color::Rgb(3, 0, 0))),
                        Span::styled(" ", Style::default().bg(Color::Rgb(4, 0, 0))),
                        Span::styled(" ", Style::default().bg(Color::Rgb(5, 0, 0))),
                    ]),
                    Spans::from(vec![
                        Span::styled(" ", Style::default().bg(Color::Rgb(6, 0, 0))),
                        Span::styled(" ", Style::default().bg(Color::Rgb(7, 0, 0))),
                        Span::styled(" ", Style::default().bg(Color::Rgb(8, 0, 0))),
                    ]),
                    Spans::from(vec![
                        Span::styled(" ", Style::default().bg(Color::Rgb(9, 0, 0))),
                        Span::styled(" ", Style::default().bg(Color::Rgb(10, 0, 0))),
                        Span::styled("", Style::reset())
                    ]),
                ]
            );
        }
    }
}
