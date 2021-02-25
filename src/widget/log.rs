use std::cell::RefCell;
use std::rc::Rc;

use tui::style::{Color, Style};
use tui::text::{Span, Spans, Text};
use tui::widgets::{Block, Paragraph, Wrap};

use super::WidgetTrait;

pub struct Logs<'a> {
    items: Vec<String>,
    state: LogState,
    spans: Vec<Spans<'a>>,
}

pub struct LogState {
    scroll: u16,
}
impl LogState {
    fn select(&mut self, index: u16) {
        self.scroll = index;
    }
    fn selected(&self) -> u16 {
        self.scroll
    }
}
impl Default for LogState {
    fn default() -> Self {
        Self { scroll: 0 }
    }
}
impl<'a> Logs<'a> {
    pub fn new(items: Vec<String>) -> Self {
        Self {
            items,
            state: LogState::default(),
            spans: vec![Spans::default()],
        }
    }

    pub fn selected(&self) -> u16 {
        self.state.selected()
    }

    pub fn select(&mut self, scroll: u16) {
        self.state.select(scroll);
    }

    pub fn state(&self) -> &LogState {
        &self.state
    }

    pub fn scroll_top(&mut self) {
        self.state.select(0);
    }

    pub fn scroll_bottom(&mut self) {
        let last_index: u16 = self.items.len() as u16 - 1;
        self.state.select(last_index);
    }

    pub fn next(&mut self) {
        let mut i = self.state.selected();

        if self.items.len() - 1 <= i as usize {
            i = (self.items.len() - 1) as u16;
        } else {
            i = i + 1;
        }

        self.state.select(i);
    }

    pub fn prev(&mut self) {
        let mut i = self.state.selected();
        if i == 0 {
            i = 0;
        } else {
            i = i - 1;
        }
        self.state.select(i);
    }

    pub fn set_items(&mut self, items: Vec<String>) {
        self.state.select(0);
        self.items = items.clone();

        self.spans = items
            .iter()
            .cloned()
            .map(|t| {
                let mut start = 0;
                let mut end = 0;
                let mut finded = false;

                let mut spans: Vec<Span> = vec![];

                while let Some(i) = t[start..].find("\x1b[") {
                    finded = true;
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

                if finded == false {
                    Spans::from(t.clone())
                } else {
                    Spans::from(spans)
                }
            })
            .collect();
    }

    pub fn items(&self) -> &Vec<String> {
        &self.items
    }

    pub fn add_item(&mut self, item: impl Into<String>) {
        self.items.push(item.into().clone());
        self.state.select(self.items.len() as u16 - 1);
    }

    pub fn spans(&self) -> &Vec<Spans> {
        &self.spans
    }

    pub fn paragraph(&self, block: Block<'a>) -> Paragraph<'a> {
        let scroll = self.state().selected();

        Paragraph::new(self.spans.clone())
            .block(block)
            .scroll((scroll, 0))
            .style(Style::default())
            .wrap(Wrap { trim: true })
    }

    fn unselect(&mut self) {
        self.state.select(0);
    }
}

impl WidgetTrait for Logs<'_> {
    fn selectable(&self) -> bool {
        true
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

fn generate_spans(text: &Vec<std::string::String>) -> Vec<Spans> {
    let mut ret: Vec<Spans> = Vec::with_capacity(1024);

    for t in text {
        // println!("{} {:#?}\n", t.len(), t);

        let buf = &t[0..];
        let mut start = 0;
        let mut end = 0;
        let mut finded = false;

        let mut spans: Vec<Span> = vec![];
        while let Some(i) = buf[start..].find("\x1b[") {
            finded = true;
            start = i + 5 + end;

            let (c0, c1) = (i + 2 + end, i + 4 + end);

            if let Some(next) = buf[start..].find("\x1b[") {
                end = next + start;
            } else {
                end = t.len();
            }
            // println!("i:{} range:{}-{} {}", i, start, end, &buf[start..]);
            spans.push(Span::styled(&buf[start..end], style(&buf[c0..c1])));

            start = end;
        }

        if finded == false {
            ret.push(Spans::from(t.clone()));
        } else {
            ret.push(Spans::from(spans));
        }
    }

    // println!("{:#?}", ret);

    ret
}

#[test]
fn parse_ansi_escape_colors() {
    let text = vec![
            "> taskbox@0.1.0 start /app",
            "> react-scripts start",
            "",
            "\x1b[34mℹ\x1b[39m \x1b[90m｢wds｣\x1b[39m: Project is running at http://10.1.157.32/",
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
            "  On Your Network:  http://10.1.157.32:3000",
            "",
            "Note that the development build is not optimized.",
            "To create a production build, use npm run build.",
        ];

    println!("{:?}", text);
}
