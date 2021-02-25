use std::cell::RefCell;
use std::rc::Rc;

use tui::style::{Color, Style};
use tui::text::{Span, Spans, Text};
use tui::widgets::{Block, Paragraph, Wrap};

use super::WidgetTrait;

pub struct Logs<'a> {
    items: Vec<String>,
    state: Rc<RefCell<LogState>>,
    spans: Vec<Spans<'a>>,
    paragraph: Paragraph<'a>,
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
        let paragraph = Paragraph::new(vec![Spans::default()]);

        Self {
            items,
            state: Rc::new(RefCell::new(LogState::default())),
            spans: vec![Spans::default()],
            paragraph,
        }
    }

    pub fn selected(&self) -> u16 {
        self.state.borrow().selected()
    }

    pub fn select(&self, scroll: u16) {
        self.state.borrow_mut().select(scroll);
    }

    pub fn state(&self) -> Rc<RefCell<LogState>> {
        Rc::clone(&self.state)
    }

    pub fn scroll_top(&self) {
        self.state.borrow_mut().select(0);
    }

    pub fn scroll_bottom(&self) {
        let last_index: u16 = self.items.len() as u16 - 1;
        self.state.borrow_mut().select(last_index);
    }

    pub fn next(&mut self) {
        let mut i = self.state.borrow().selected();

        if self.items.len() - 1 <= i as usize {
            i = (self.items.len() - 1) as u16;
        } else {
            i = i + 1;
        }

        self.state.borrow_mut().select(i);
        self.paragraph = self.paragraph.clone().scroll((i, 0));
    }

    pub fn prev(&mut self) {
        let mut i = self.state.borrow().selected();
        if i == 0 {
            i = 0;
        } else {
            i = i - 1;
        }
        self.state.borrow_mut().select(i);
        self.paragraph = self.paragraph.clone().scroll((i, 0));
    }

    pub fn set_items(&mut self, items: Vec<String>) {
        self.state.borrow_mut().select(0);
        self.items = items.clone();

        self.spans = items.iter().cloned().map(Spans::from).collect();

        self.paragraph = Paragraph::new(self.spans.clone())
            .style(Style::default())
            .wrap(Wrap { trim: false });
    }

    pub fn items(&self) -> &Vec<String> {
        &self.items
    }

    pub fn add_item(&mut self, item: &String) {
        self.items.push(item.clone());
        self.state.borrow_mut().select(self.items.len() as u16 - 1);
    }

    pub fn spans(&self) -> &Vec<Spans> {
        &self.spans
    }

    pub fn paragraph(&self, block: Block<'a>) -> Paragraph<'a> {
        let scroll = self.state().borrow().selected();

        self.paragraph.clone().block(block).scroll((scroll, 0))
    }

    fn unselect(&self) {
        self.state().borrow_mut().select(0);
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
