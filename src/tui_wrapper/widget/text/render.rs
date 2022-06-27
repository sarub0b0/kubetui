/// 文字列を描画するためのモジュ&ール
/// - 渡された１行ずつのデータを描画する
/// - 渡された縦横スクロールの位置をもとに描画位置を決定する
///
/// 考慮しないこと
/// - 折り返しする・しないの制御
/// - スクロールをする・しないの制御
///
/// このモジュールではステートを持たないこととし、
/// 上位のレイヤーでスクロールの位置や折り返しを管理すること
use tui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    widgets::{Block, Widget},
};
use unicode_width::UnicodeWidthStr;

use super::{
    highlight_content::{HighlightArea, Point},
    item::WrappedLine,
    styled_graphemes::StyledGrapheme,
};

#[derive(Debug, Default, Clone, Copy)]
pub struct Scroll {
    pub x: usize,
    pub y: usize,
}

#[derive(Debug, Default, Clone)]
pub struct Render<'a> {
    block: Block<'a>,
    lines: &'a [WrappedLine],
    scroll: Scroll,
    highlight_area: Option<HighlightArea>,
}

pub struct RenderBuilder<'a>(Render<'a>);

impl<'a> RenderBuilder<'a> {
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.0.block = block;
        self
    }

    pub fn lines(mut self, lines: &'a [WrappedLine]) -> Self {
        self.0.lines = lines;
        self
    }

    pub fn scroll(mut self, scroll: Scroll) -> Self {
        self.0.scroll = scroll;
        self
    }

    pub fn highlight_area(mut self, highlight_area: Option<HighlightArea>) -> Self {
        self.0.highlight_area = highlight_area;
        self
    }

    pub fn build(self) -> Render<'a> {
        self.0
    }
}

impl<'a> Render<'a> {
    pub fn builder() -> RenderBuilder<'a> {
        RenderBuilder(Render::default())
    }
}

impl Widget for Render<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let text_area = self.block.inner(area);

        self.block.render(area, buf);

        let start = self.scroll.y;
        let end = text_area.height as usize;
        let area_width = text_area.width as usize;

        for (y, line) in self.lines.iter().skip(start).take(end).enumerate() {
            let mut x = 0;

            let iter = LineIterator::new(line.line(), self.scroll.x, text_area.width as usize)
                .collect::<Vec<_>>();

            for sg in iter.iter() {
                let symbol = sg.symbol();
                let mut style = *sg.style();

                if let Some(highlight_area) = self.highlight_area {
                    if highlight_area.contains(Point {
                        x: x as usize + self.scroll.x,
                        y: y as usize + self.scroll.y,
                    }) {
                        style = style.add_modifier(Modifier::REVERSED);
                    }
                }

                buf.get_mut(text_area.left() + x as u16, text_area.top() + y as u16)
                    .set_symbol(symbol)
                    .set_style(style);

                x += symbol.width()
            }

            if let Some(next_line) = self.lines.get(start + y + 1) {
                if line.index() == next_line.index() && x < area_width {
                    buf.get_mut(text_area.left() + x as u16, text_area.top() + y as u16)
                        .set_symbol(RENDER_RIGHT_PADDING.symbol())
                        .set_style(RENDER_RIGHT_PADDING.style);

                    x += 1
                }
            }

            while x < area_width {
                buf.get_mut(text_area.left() + x as u16, text_area.top() + y as u16)
                    .set_symbol(" ");

                x += " ".width()
            }
        }
    }
}

#[derive(Debug, Default)]
struct LineIterator<'a> {
    /// 一行分のStyledGraphemeの配列の参照
    line: &'a [StyledGrapheme],

    /// 右にスクロールする数
    /// 半角文字基準
    scroll: usize,

    /// 描画エリアの横幅
    render_width: usize,

    /// 次のlineのインデックス
    n: usize,

    /// nまでの文字幅の合計
    sum_width: usize,

    /// offset
    sum_width_offset: usize,
}

const RENDER_LEFT_PADDING_SYMBOL: &str = "<";
const RENDER_RIGHT_PADDING_SYMBOL: &str = ">";

const RENDER_LEFT_PADDING: StyledGrapheme = StyledGrapheme {
    symbol_ptr: RENDER_LEFT_PADDING_SYMBOL,
    style: Style {
        fg: None,
        bg: None,
        #[cfg(not(test))]
        add_modifier: Modifier::DIM,
        #[cfg(test)]
        add_modifier: Modifier::empty(),
        sub_modifier: Modifier::empty(),
    },
};

const RENDER_RIGHT_PADDING: StyledGrapheme = StyledGrapheme {
    symbol_ptr: RENDER_RIGHT_PADDING_SYMBOL,
    style: Style {
        fg: None,
        bg: None,
        #[cfg(not(test))]
        add_modifier: Modifier::DIM,
        #[cfg(test)]
        add_modifier: Modifier::empty(),
        sub_modifier: Modifier::empty(),
    },
};

impl<'a> LineIterator<'a> {
    fn new(line: &'a [StyledGrapheme], scroll: usize, width: usize) -> Self {
        let (n, offset) = Self::start(line, scroll);

        Self {
            line,
            scroll,
            render_width: width,
            n,
            sum_width_offset: offset,
            ..Default::default()
        }
    }

    fn start(line: &'a [StyledGrapheme], scroll: usize) -> (usize, usize) {
        let mut sum = 0;
        let mut i = 0;
        for sg in line {
            if scroll < sum + sg.symbol().width() {
                break;
            }

            sum += sg.symbol().width();
            i += 1;
        }

        (i, sum)
    }
}

/// 右スクロール分(start)ずらした場所からStyledGraphemeを取り出す
/// 奇数かつ全角文字の場合は行頭に"<"を挿入する
/// 奇数かつ全角文字の場合は行末に">"を挿入する
///
/// # Examples
/// |aああああああああああああ>|
/// |<ああああああああああああa|
/// |あああああああああああああ|
/// |<ああああああああああああ>|
impl<'a> Iterator for LineIterator<'a> {
    type Item = &'a StyledGrapheme;

    fn next(&mut self) -> Option<Self::Item> {
        if self.line.len() <= self.n {
            return None;
        }

        let sg = &self.line[self.n];
        self.sum_width += sg.symbol().width();
        self.n += 1;

        if sg.symbol().width() == 2
            && (self.sum_width + self.sum_width_offset).saturating_sub(self.scroll) == 1
        {
            self.sum_width -= 1;
            return Some(&RENDER_LEFT_PADDING);
        }

        if self.sum_width <= self.render_width {
            Some(sg)
        } else if sg.symbol().width() == 2
            && (self.sum_width).saturating_sub(self.render_width) == 1
        {
            self.sum_width -= 1;
            Some(&RENDER_RIGHT_PADDING)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use tui::{backend::TestBackend, widgets::Borders, Terminal};
    use unicode_segmentation::UnicodeSegmentation;

    use super::*;

    const TERMINAL_WIDTH: u16 = 20;
    const TERMINAL_HEIGHT: u16 = 10;

    trait StyledGraphemes<'a> {
        fn styled_graphemes(&self) -> Vec<StyledGrapheme>;
    }

    impl<'a> StyledGraphemes<'a> for &'a str {
        fn styled_graphemes(&self) -> Vec<StyledGrapheme> {
            self.graphemes(true)
                .map(|g| StyledGrapheme::new(g, Style::default()))
                .collect::<Vec<_>>()
        }
    }

    trait VecStyledGraphemes<'a> {
        fn styled_graphemes(&self) -> Vec<Vec<StyledGrapheme>>;
    }

    impl<'a> VecStyledGraphemes<'a> for Vec<&'a str> {
        fn styled_graphemes(&self) -> Vec<Vec<StyledGrapheme>> {
            self.iter()
                .map(|line| line.styled_graphemes())
                .collect::<Vec<_>>()
        }
    }

    fn setup_terminal(width: u16, height: u16) -> (Terminal<TestBackend>, Rect) {
        (
            Terminal::new(TestBackend::new(width, height)).unwrap(),
            Rect::new(0, 0, width, height),
        )
    }

    mod 描画 {
        use super::*;

        mod 枠あり {
            use super::*;

            macro_rules! render_test {
                ($terminal_width:expr, $terminal_height:expr, $lines:expr, $expected:expr) => {{
                    let (mut terminal, area) = setup_terminal($terminal_width, $terminal_height);

                    let lines = $lines;

                    let styled_graphemes = lines.styled_graphemes();

                    let lines: Vec<_> = styled_graphemes
                        .iter()
                        .enumerate()
                        .map(|(i, sg)| WrappedLine::new(i, sg))
                        .collect();

                    let render = Render {
                        block: Block::default().borders(Borders::ALL),
                        lines: &lines,
                        ..Default::default()
                    };

                    terminal
                        .draw(|f| {
                            f.render_widget(render, area);
                        })
                        .unwrap();

                    let expected = Buffer::with_lines($expected);

                    terminal.backend().assert_buffer(&expected);
                }};
            }

            #[test]
            fn 文字列がない場合は枠のみを描画する() {
                render_test!(
                    TERMINAL_WIDTH,
                    TERMINAL_HEIGHT,
                    vec![],
                    vec![
                        "┌──────────────────┐",
                        "│                  │",
                        "│                  │",
                        "│                  │",
                        "│                  │",
                        "│                  │",
                        "│                  │",
                        "│                  │",
                        "│                  │",
                        "└──────────────────┘",
                    ]
                )
            }

            #[test]
            fn 文字列が収まらない場合は収まる分だけ描画する() {
                // cSpell:ignore abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqr
                render_test!(
                    TERMINAL_WIDTH,
                    TERMINAL_HEIGHT,
                    vec![
                        "abcdefghijklmnopqrstuvwxyz",
                        "01234567890123456789",
                        "hello world",
                    ],
                    vec![
                        "┌──────────────────┐",
                        "│abcdefghijklmnopqr│",
                        "│012345678901234567│",
                        "│hello world       │",
                        "│                  │",
                        "│                  │",
                        "│                  │",
                        "│                  │",
                        "│                  │",
                        "└──────────────────┘",
                    ]
                )
            }

            #[test]
            fn 二文字幅を含む文字列は枠内に収まるよう描画する() {
                render_test!(
                    TERMINAL_WIDTH + 1,
                    TERMINAL_HEIGHT,
                    vec![
                        "あいうえおかきくけ",
                        "アイウエオカキクケ",
                        "ｱｲｳｴｵｶｷｸｹｺ",
                        "一二三四五六七八九",
                    ],
                    vec![
                        "┌───────────────────┐",
                        "│あいうえおかきくけ │",
                        "│アイウエオカキクケ │",
                        "│ｱｲｳｴｵｶｷｸｹｺ         │",
                        "│一二三四五六七八九 │",
                        "│                   │",
                        "│                   │",
                        "│                   │",
                        "│                   │",
                        "└───────────────────┘",
                    ]
                )
            }

            #[test]
            fn 二文字幅の文字で折り返り時にスペースが空くときパディングする() {
                let (mut terminal, area) = setup_terminal(TERMINAL_WIDTH + 1, TERMINAL_HEIGHT);

                let lines = vec![
                    "あいうえおかきくけ",
                    "こ",
                    "アイウエオカキクケ",
                    "コ",
                    "ｱｲｳｴｵｶｷｸｹｺ",
                    "一二三四五六七八九",
                ];

                let styled_graphemes = lines.styled_graphemes();

                let lines = vec![
                    WrappedLine::new(0, &styled_graphemes[0]),
                    WrappedLine::new(0, &styled_graphemes[1]),
                    WrappedLine::new(1, &styled_graphemes[2]),
                    WrappedLine::new(1, &styled_graphemes[3]),
                    WrappedLine::new(2, &styled_graphemes[4]),
                    WrappedLine::new(3, &styled_graphemes[5]),
                ];

                let render = Render {
                    block: Block::default().borders(Borders::ALL),
                    lines: &lines,
                    ..Default::default()
                };

                terminal
                    .draw(|f| {
                        f.render_widget(render, area);
                    })
                    .unwrap();

                let expected = Buffer::with_lines(vec![
                    "┌───────────────────┐",
                    "│あいうえおかきくけ>│",
                    "│こ                 │",
                    "│アイウエオカキクケ>│",
                    "│コ                 │",
                    "│ｱｲｳｴｵｶｷｸｹｺ         │",
                    "│一二三四五六七八九 │",
                    "│                   │",
                    "│                   │",
                    "└───────────────────┘",
                ]);

                terminal.backend().assert_buffer(&expected);
            }
        }
    }

    mod スクロール {
        use super::*;

        mod 縦スクロール {
            use super::*;

            mod 下にスクロール {
                use super::*;

                macro_rules! render_test {
                    ($terminal_width:expr, $terminal_height:expr, $lines:expr, $expected:expr, $scroll:literal) => {{
                        let (mut terminal, area) =
                            setup_terminal($terminal_width, $terminal_height);

                        let lines = $lines;

                        let styled_graphemes = lines.styled_graphemes();

                        let lines: Vec<_> = styled_graphemes
                            .iter()
                            .enumerate()
                            .map(|(i, sg)| WrappedLine::new(i, sg))
                            .collect();

                        let render = Render {
                            block: Block::default().borders(Borders::ALL),
                            lines: &lines,
                            scroll: Scroll { x: 0, y: $scroll },
                            ..Default::default()
                        };

                        terminal
                            .draw(|f| {
                                f.render_widget(render, area);
                            })
                            .unwrap();

                        let expected = Buffer::with_lines($expected);

                        terminal.backend().assert_buffer(&expected);
                    }};
                }

                #[test]
                fn 文字列の範囲外にスクロールしたとき何も描画しない() {
                    render_test!(
                        TERMINAL_WIDTH,
                        TERMINAL_HEIGHT,
                        vec![
                            "abcdefghijklmnopqrstuvwxyz",
                            "01234567890123456789",
                            "hello world",
                        ],
                        vec![
                            "┌──────────────────┐",
                            "│                  │",
                            "│                  │",
                            "│                  │",
                            "│                  │",
                            "│                  │",
                            "│                  │",
                            "│                  │",
                            "│                  │",
                            "└──────────────────┘",
                        ],
                        20
                    );
                }

                #[test]
                fn 指定した数だけスクロールする() {
                    render_test!(
                        TERMINAL_WIDTH,
                        TERMINAL_HEIGHT,
                        vec![
                            "abcdefghijklmnopqrstuvwxyz",
                            "01234567890123456789",
                            "hello world",
                            "0",
                            "1",
                            "2",
                            "3",
                            "4",
                            "5",
                            "6",
                            "7",
                            "8",
                            "9",
                            "10",
                        ],
                        vec![
                            "┌──────────────────┐",
                            "│hello world       │",
                            "│0                 │",
                            "│1                 │",
                            "│2                 │",
                            "│3                 │",
                            "│4                 │",
                            "│5                 │",
                            "│6                 │",
                            "└──────────────────┘",
                        ],
                        2
                    );
                }

                #[test]
                fn 文字列の最後尾をいくつか描画できる() {
                    render_test!(
                        TERMINAL_WIDTH,
                        TERMINAL_HEIGHT,
                        vec![
                            "abcdefghijklmnopqrstuvwxyz",
                            "01234567890123456789",
                            "hello world",
                            "0",
                            "1",
                            "2",
                            "3",
                            "4",
                            "5",
                            "6",
                            "7",
                            "8",
                            "9",
                            "10",
                        ],
                        vec![
                            "┌──────────────────┐",
                            "│7                 │",
                            "│8                 │",
                            "│9                 │",
                            "│10                │",
                            "│                  │",
                            "│                  │",
                            "│                  │",
                            "│                  │",
                            "└──────────────────┘",
                        ],
                        10
                    );
                }
            }
        }

        mod 横スクロール {
            use super::*;

            mod 右にスクロール {
                use super::*;

                macro_rules! render_test {
                    ($terminal_width:expr, $terminal_height:expr, $lines:expr, $expected:expr, $scroll:literal) => {{
                        let (mut terminal, area) =
                            setup_terminal($terminal_width, $terminal_height);

                        let lines = $lines;

                        let styled_graphemes = lines.styled_graphemes();

                        let lines: Vec<_> = styled_graphemes
                            .iter()
                            .enumerate()
                            .map(|(i, sg)| WrappedLine::new(i, sg))
                            .collect();

                        let render = Render {
                            block: Block::default().borders(Borders::ALL),
                            lines: &lines,
                            scroll: Scroll { x: $scroll, y: 0 },
                            ..Default::default()
                        };

                        terminal
                            .draw(|f| {
                                f.render_widget(render, area);
                            })
                            .unwrap();

                        let expected = Buffer::with_lines($expected);

                        terminal.backend().assert_buffer(&expected);
                    }};
                }

                #[test]
                fn 文字列の範囲外にスクロールしたとき何も描画しない() {
                    render_test!(
                        TERMINAL_WIDTH,
                        TERMINAL_HEIGHT,
                        vec![
                            "abcdefghijklmnopqrstuvwxyz",
                            "01234567890123456789",
                            "hello world",
                        ],
                        vec![
                            "┌──────────────────┐",
                            "│                  │",
                            "│                  │",
                            "│                  │",
                            "│                  │",
                            "│                  │",
                            "│                  │",
                            "│                  │",
                            "│                  │",
                            "└──────────────────┘",
                        ],
                        30
                    );
                }

                #[test]
                fn 指定した数だけスクロールする() {
                    // cSpell:ignore cdefghijklmnopqrst
                    render_test!(
                        TERMINAL_WIDTH,
                        TERMINAL_HEIGHT,
                        vec![
                            "abcdefghijklmnopqrstuvwxyz",
                            "01234567890123456789",
                            "hello world",
                            "0",
                            "1",
                            "2",
                            "3",
                            "4",
                            "5",
                            "6",
                            "7",
                            "8",
                            "9",
                            "10",
                        ],
                        vec![
                            "┌──────────────────┐",
                            "│cdefghijklmnopqrst│",
                            "│234567890123456789│",
                            "│llo world         │",
                            "│                  │",
                            "│                  │",
                            "│                  │",
                            "│                  │",
                            "│                  │",
                            "└──────────────────┘",
                        ],
                        2
                    )
                }

                #[test]
                fn 行頭で全角文字を表示する幅が足りないとき不等号を表示する() {
                    render_test!(
                        TERMINAL_WIDTH + 1,
                        TERMINAL_HEIGHT,
                        vec![
                            "あいうえおかきくけこ",
                            "アイウエオカキクケコ",
                            "ｱｲｳｴｵｶｷｸｹｺ",
                            "一二三四五六七八九十",
                        ],
                        vec![
                            "┌───────────────────┐",
                            "│<いうえおかきくけこ│",
                            "│<イウエオカキクケコ│",
                            "│ｲｳｴｵｶｷｸｹｺ          │",
                            "│<二三四五六七八九十│",
                            "│                   │",
                            "│                   │",
                            "│                   │",
                            "│                   │",
                            "└───────────────────┘",
                        ],
                        1
                    )
                }

                #[test]
                fn 行末で全角文字を表示する幅が足りないとき不等号を表示する() {
                    render_test!(
                        TERMINAL_WIDTH + 1,
                        TERMINAL_HEIGHT,
                        vec![
                            "あいうえおかきくけこ",
                            "アイウエオカキクケコ",
                            "ｱｲｳｴｵｶｷｸｹｺ",
                            "一二三四五六七八九十",
                        ],
                        vec![
                            "┌───────────────────┐",
                            "│あいうえおかきくけ>│",
                            "│アイウエオカキクケ>│",
                            "│ｱｲｳｴｵｶｷｸｹｺ         │",
                            "│一二三四五六七八九>│",
                            "│                   │",
                            "│                   │",
                            "│                   │",
                            "│                   │",
                            "└───────────────────┘",
                        ],
                        0
                    )
                }
            }

            mod line_iterator {
                use super::*;

                mod 開始位置 {
                    use super::*;
                    use pretty_assertions::assert_eq;

                    #[test]
                    fn 開始位置が0のとき0を返す() {
                        // cSpell:ignore ijklmnopqrstuvwxyz
                        let line = "abあいうえおijklmnopqrstuvwxyz".styled_graphemes();

                        let actual = LineIterator::start(&line, 0);

                        assert_eq!((0, 0), actual)
                    }

                    #[test]
                    fn スクロール位置が文字の区切りと一致するとき文字のインデックスを返す() {
                        let line = "abあいうえおijklmnopqrstuvwxyz".styled_graphemes();

                        let actual = LineIterator::start(&line, 12);
                        assert_eq!((7, 12), actual);
                    }

                    #[test]
                    fn スクロール位置が全角文字の中間のときその文字のインデックスを返す() {
                        let line = "abあいうえおijklmnopqrstuvwxyz".styled_graphemes();

                        let actual = LineIterator::start(&line, 3);
                        assert_eq!((2, 2), actual);
                    }
                }

                mod iteration {
                    use super::*;
                    use pretty_assertions::assert_eq;

                    #[test]
                    fn 右スクロール分ずらしたところから値を返す() {
                        let line = "abcdefghijklmnopqrstuvwxyz".styled_graphemes();

                        let mut iter = LineIterator::new(&line, 3, 10);

                        assert_eq!(
                            iter.next(),
                            Some(&StyledGrapheme::new(line[3].symbol(), Style::default()))
                        );
                    }

                    #[test]
                    fn 一行分の文字幅が描画の幅よりも小さいとき一行分返す() {
                        let line = "abcdefghijklmnopqrstuvwxyz".styled_graphemes();

                        let iter = LineIterator::new(&line, 3, 34);

                        assert_eq!(
                            iter.last(),
                            Some(&StyledGrapheme::new(line[25].symbol(), Style::default()))
                        );
                    }

                    #[test]
                    fn 一行分の文字幅が描画の幅よりも長いとき描画できる分だけ値を返す() {
                        let line = "abcdefghijklmnopqrstuvwxyz".styled_graphemes();

                        let iter = LineIterator::new(&line, 3, 20);

                        assert_eq!(
                            iter.last(),
                            Some(&StyledGrapheme::new(line[22].symbol(), Style::default()))
                        );
                    }
                }

                mod 全角文字を含む場合のパディング {
                    use super::*;
                    use pretty_assertions::assert_eq;

                    #[test]
                    fn 行頭に全角文字を表示できるとき全角文字を返す() {
                        let line = "アイウエオかきくけこ".styled_graphemes();

                        let iter = LineIterator::new(&line, 0, 30);

                        let actual: String = iter.map(|sg| sg.symbol()).collect();

                        let expected = "アイウエオかきくけこ".to_string();

                        assert_eq!(actual, expected);
                    }

                    #[test]
                    ///  あああああああああああああ
                    /// |ああああああああああああ>|
                    fn 行末で全角文字を表示する幅が足りないとき不等号を返す() {
                        let line = "アイウエオかきくけこ".styled_graphemes();

                        let iter = LineIterator::new(&line, 4, 15);

                        let actual: String = iter.map(|sg| sg.symbol()).collect();

                        let expected = "ウエオかきくけ>".to_string();

                        assert_eq!(actual, expected);
                    }

                    #[test]
                    /// あああああああああああああ|
                    /// |<ああああああああああああ|
                    fn 行頭で全角文字を表示する幅が足りないとき不等号を返す() {
                        let line = "アイウエオかきくけこ".styled_graphemes();

                        let iter = LineIterator::new(&line, 3, 30);

                        let actual: String = iter.map(|sg| sg.symbol()).collect();

                        let expected = "<ウエオかきくけこ".to_string();

                        assert_eq!(actual, expected);
                    }
                }
            }
        }
    }
}
