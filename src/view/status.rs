use chrono::{DateTime, Duration, Utc};

use tui::style::Style;
use tui::text::{Span, Spans};
use tui::widgets::{Block, Paragraph};

pub struct Status {
    scroll: Scroll,
    time: Time,
}

impl Status {
    pub fn new() -> Self {
        Self {
            scroll: Scroll {
                current: 0,
                rows: 0,
            },
            time: Time {},
        }
    }

    fn update_scroll(&mut self, scroll: u16) {
        self.scroll.current = scroll;
    }
}

struct Scroll {
    current: u16,
    rows: u16,
}

struct Time;

impl Time {
    fn widget(&self) -> Paragraph {
        Paragraph::new(self.datetime()).block(Block::default().style(Style::default()))
    }

    fn datetime(&self) -> Spans {
        Spans::from(vec![Span::raw(format!(
            " {}",
            Utc::now().format("%Y年%m月%d日 %H時%M分%S秒")
        ))])
    }
}
