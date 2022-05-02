use crossterm::event::{KeyEvent, MouseEvent};
use tui::{
    backend::Backend,
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::StyledGrapheme,
    widgets::{Block, Widget},
    Frame,
};
use unicode_width::UnicodeWidthStr;

use crate::tui_wrapper::event::EventResult;

use super::{config::WidgetConfig, Item, RenderTrait, SelectedItem, WidgetTrait};

struct Text {
    id: String,
    widget_config: WidgetConfig,
    chunk: Rect,
}

impl WidgetTrait for Text {
    fn id(&self) -> &str {
        &self.id
    }

    fn widget_config(&self) -> &WidgetConfig {
        &self.widget_config
    }

    fn widget_config_mut(&mut self) -> &mut WidgetConfig {
        &mut self.widget_config
    }

    fn focusable(&self) -> bool {
        todo!()
    }

    fn widget_item(&self) -> Option<SelectedItem> {
        todo!()
    }

    fn chunk(&self) -> Rect {
        self.chunk
    }

    fn select_index(&mut self, _: usize) {
        todo!()
    }

    fn select_next(&mut self, _: usize) {
        todo!()
    }

    fn select_prev(&mut self, _: usize) {
        todo!()
    }

    fn select_first(&mut self) {
        todo!()
    }

    fn select_last(&mut self) {
        todo!()
    }

    fn append_widget_item(&mut self, _: Item) {
        todo!()
    }

    fn update_widget_item(&mut self, _: Item) {
        todo!()
    }

    fn on_mouse_event(&mut self, _: MouseEvent) -> EventResult {
        todo!()
    }

    fn on_key_event(&mut self, _: KeyEvent) -> EventResult {
        todo!()
    }

    fn update_chunk(&mut self, _: Rect) {
        todo!()
    }

    fn clear(&mut self) {
        todo!()
    }
}

impl RenderTrait for Text {
    fn render<B>(&mut self, _f: &mut Frame<'_, B>, _: bool)
    where
        B: Backend,
    {
        todo!()
    }
}

mod styled_graphemes {
    use tui::{style::Style, text::StyledGrapheme};
    use unicode_segmentation::UnicodeSegmentation;

    use crate::{
        ansi::{AnsiEscapeSequence, TextParser},
        tui_wrapper::widget::ansi_color::SGR,
    };

    pub trait StyledGraphemes {
        fn styled_graphemes(&self) -> Vec<StyledGrapheme<'_>>;
    }

    impl StyledGraphemes for String {
        fn styled_graphemes(&self) -> Vec<StyledGrapheme<'_>> {
            styled_graphemes(self)
        }
    }

    impl StyledGraphemes for &'static str {
        fn styled_graphemes(&self) -> Vec<StyledGrapheme<'static>> {
            styled_graphemes(self)
        }
    }

    /// 一文字単位でスタイルを適用したリストを返す
    pub fn styled_graphemes(s: &str) -> Vec<StyledGrapheme<'_>> {
        let mut style = Style::default();

        s.ansi_parse()
            .filter_map(|p| match p.ty {
                AnsiEscapeSequence::Chars => Some(StyledGrapheme {
                    symbol: p.chars,
                    style,
                }),
                AnsiEscapeSequence::SelectGraphicRendition(sgr) => {
                    style = SGR::from(sgr).into();
                    None
                }
                _ => None,
            })
            .flat_map(|sg| {
                sg.symbol
                    .graphemes(true)
                    .map(|g| StyledGrapheme {
                        symbol: g,
                        style: sg.style,
                    })
                    .collect::<Vec<StyledGrapheme<'_>>>()
            })
            .collect()
    }
}

/// 文字列を描画するためのモジュール
/// - 渡された１行ずつのデータを描画する
/// - 渡された縦横スクロールの位置をもとに描画位置を決定する
///
/// 考慮しないこと
/// - 折り返しする・しないの制御
/// - スクロールをする・しないの制御
///
/// このモジュールではステートを持たないこととし、
/// 上位のレイヤーでスクロールの位置や折り返しを管理すること
mod render {

    use tui::style::Modifier;

    #[cfg(not(test))]
    use tui::style::Color;

    use super::*;

    #[derive(Debug, Default, Clone, Copy)]
    struct Scroll {
        x: usize,
        y: usize,
    }

    #[derive(Debug, Default, Clone)]
    struct Render<'a> {
        block: Block<'a>,
        lines: &'a [&'a [StyledGrapheme<'a>]],
        scroll: Scroll,
    }

    pub struct RenderBuilder<'a>(Render<'a>);

    impl<'a> RenderBuilder<'a> {
        fn block(mut self, block: Block<'a>) -> Self {
            self.0.block = block;
            self
        }

        fn lines(mut self, lines: &'a [&'a [StyledGrapheme<'a>]]) -> Self {
            self.0.lines = lines;
            self
        }

        fn scroll(mut self, scroll: Scroll) -> Self {
            self.0.scroll = scroll;
            self
        }

        fn build(self) -> Render<'a> {
            self.0
        }
    }

    impl<'a> Render<'a> {
        fn builder() -> RenderBuilder<'a> {
            RenderBuilder(Render::default())
        }
    }

    impl Widget for Render<'_> {
        fn render(self, area: Rect, buf: &mut Buffer) {
            let text_area = self.block.inner(area);

            self.block.render(area, buf);

            let start = self.scroll.y;
            let end = text_area.height as usize;

            for (y, line) in self.lines.iter().skip(start).take(end).enumerate() {
                let mut x = 0;

                let iter = LineIterator::new(line, self.scroll.x, text_area.width as usize);

                for StyledGrapheme { symbol, style } in iter {
                    buf.get_mut(text_area.left() + x as u16, text_area.top() + y as u16)
                        .set_symbol(symbol)
                        .set_style(*style);

                    x += symbol.width()
                }
            }
        }
    }

    #[derive(Debug, Default)]
    struct LineIterator<'a> {
        /// 一行分のStyledGraphemeの配列の参照
        line: &'a [StyledGrapheme<'a>],

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

    #[cfg(not(test))]
    const RENDER_LEFT_PADDING: StyledGrapheme<'static> = StyledGrapheme {
        symbol: "<",
        style: Style {
            fg: Some(Color::Gray),
            bg: None,
            add_modifier: Modifier::empty(),
            sub_modifier: Modifier::empty(),
        },
    };

    #[cfg(not(test))]
    const RENDER_RIGHT_PADDING: StyledGrapheme<'static> = StyledGrapheme {
        symbol: ">",
        style: Style {
            fg: Some(Color::Gray),
            bg: None,
            add_modifier: Modifier::empty(),
            sub_modifier: Modifier::empty(),
        },
    };

    #[cfg(test)]
    const RENDER_LEFT_PADDING: StyledGrapheme<'static> = StyledGrapheme {
        symbol: "<",
        style: Style {
            fg: None,
            bg: None,
            add_modifier: Modifier::empty(),
            sub_modifier: Modifier::empty(),
        },
    };

    #[cfg(test)]
    const RENDER_RIGHT_PADDING: StyledGrapheme<'static> = StyledGrapheme {
        symbol: ">",
        style: Style {
            fg: None,
            bg: None,
            add_modifier: Modifier::empty(),
            sub_modifier: Modifier::empty(),
        },
    };

    impl<'a> LineIterator<'a> {
        fn new(line: &'a [StyledGrapheme<'a>], scroll: usize, width: usize) -> Self {
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

        fn start(line: &'a [StyledGrapheme<'a>], scroll: usize) -> (usize, usize) {
            let mut sum = 0;
            let mut i = 0;
            for sg in line {
                if scroll < sum + sg.symbol.width() {
                    break;
                }

                sum += sg.symbol.width();
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
        type Item = &'a StyledGrapheme<'a>;

        fn next(&mut self) -> Option<Self::Item> {
            if self.line.len() <= self.n {
                return None;
            }

            let sg = &self.line[self.n];
            self.sum_width += sg.symbol.width();

            if sg.symbol.width() == 2
                && (self.sum_width + self.sum_width_offset).saturating_sub(self.scroll) == 1
            {
                self.n += 1;
                self.sum_width -= 1;
                return Some(&RENDER_LEFT_PADDING);
            }

            if self.sum_width <= self.render_width {
                self.n += 1;
                Some(sg)
            } else if sg.symbol.width() == 2
                && (self.sum_width).saturating_sub(self.render_width) == 1
            {
                self.n += 1;
                self.sum_width -= 1;
                Some(&RENDER_RIGHT_PADDING)
            } else {
                None
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        use tui::{backend::TestBackend, buffer::Buffer, layout::Rect, widgets::Borders, Terminal};
        use unicode_segmentation::UnicodeSegmentation;

        const TERMINAL_WIDTH: u16 = 20;
        const TERMINAL_HEIGHT: u16 = 10;

        trait StyledGraphemes<'a> {
            fn styled_graphemes(&self) -> Vec<StyledGrapheme<'a>>;
        }

        impl<'a> StyledGraphemes<'a> for &'a str {
            fn styled_graphemes(&self) -> Vec<StyledGrapheme<'a>> {
                self.graphemes(true)
                    .map(|g| StyledGrapheme {
                        symbol: g,
                        style: Style::default(),
                    })
                    .collect::<Vec<_>>()
            }
        }

        trait VecStyledGraphemes<'a> {
            fn styled_graphemes(&self) -> Vec<Vec<StyledGrapheme<'a>>>;
        }

        impl<'a> VecStyledGraphemes<'a> for Vec<&'a str> {
            fn styled_graphemes(&self) -> Vec<Vec<StyledGrapheme<'a>>> {
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

                macro_rules! test {
                    ($terminal_width:expr, $terminal_height:expr, $lines:expr, $expected:expr) => {{
                        let (mut terminal, area) =
                            setup_terminal($terminal_width, $terminal_height);

                        let lines = $lines;

                        let styled_graphemes = lines.styled_graphemes();

                        let lines = styled_graphemes.iter().map(|l| &l[..]).collect::<Vec<_>>();

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
                    test!(
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
                    test!(
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
                    test!(
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
            }
        }

        mod スクロール {
            use super::*;

            mod 縦スクロール {
                use super::*;

                mod 下にスクロール {
                    use super::*;

                    macro_rules! test {
                        ($terminal_width:expr, $terminal_height:expr, $lines:expr, $expected:expr, $scroll:literal) => {{
                            let (mut terminal, area) =
                                setup_terminal($terminal_width, $terminal_height);

                            let lines = $lines;

                            let styled_graphemes = lines.styled_graphemes();

                            let lines = styled_graphemes.iter().map(|l| &l[..]).collect::<Vec<_>>();

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
                        test!(
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
                        test!(
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
                        test!(
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

                    macro_rules! test {
                        ($terminal_width:expr, $terminal_height:expr, $lines:expr, $expected:expr, $scroll:literal) => {{
                            let (mut terminal, area) =
                                setup_terminal($terminal_width, $terminal_height);

                            let lines = $lines;

                            let styled_graphemes = lines.styled_graphemes();

                            let lines = styled_graphemes.iter().map(|l| &l[..]).collect::<Vec<_>>();

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
                        test!(
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
                        test!(
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
                        test!(
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
                        test!(
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
                                Some(&StyledGrapheme {
                                    symbol: "d",
                                    style: Style::default()
                                })
                            );
                        }

                        #[test]
                        fn 一行分の文字幅が描画の幅よりも小さいとき一行分返す() {
                            let line = "abcdefghijklmnopqrstuvwxyz".styled_graphemes();

                            let iter = LineIterator::new(&line, 3, 34);

                            assert_eq!(
                                iter.last(),
                                Some(&StyledGrapheme {
                                    symbol: "z",
                                    style: Style::default()
                                })
                            );
                        }

                        #[test]
                        fn 一行分の文字幅が描画の幅よりも長いとき描画できる分だけ値を返す() {
                            let line = "abcdefghijklmnopqrstuvwxyz".styled_graphemes();

                            let iter = LineIterator::new(&line, 3, 20);

                            assert_eq!(
                                iter.last(),
                                Some(&StyledGrapheme {
                                    symbol: "w",
                                    style: Style::default()
                                })
                            );
                        }
                    }

                    mod 全角文字を含む場合のパディング {
                        use super::*;
                        use pretty_assertions::assert_eq;

                        macro_rules! sg {
                            ($expr:expr) => {
                                styled_grapheme!($expr)
                            };
                        }

                        macro_rules! styled_grapheme {
                            ($symbol:expr, $style:expr) => {
                                StyledGrapheme {
                                    symbol: $symbol,
                                    style: $style,
                                }
                            };

                            ($symbol:expr) => {
                                styled_grapheme!($symbol, Style::default())
                            };
                        }

                        macro_rules! expected {
                            ($($value:tt)*) => {
                                vec![
                                    $($value)*
                                ]
                            };
                        }

                        #[test]
                        fn 行頭に全角文字を表示できるとき全角文字を返す() {
                            let line = "アイウエオかきくけこ".styled_graphemes();

                            let iter = LineIterator::new(&line, 0, 30);

                            let actual: Vec<StyledGrapheme> = iter.cloned().collect();

                            assert_eq!(
                                expected!(
                                    sg!("ア"),
                                    sg!("イ"),
                                    sg!("ウ"),
                                    sg!("エ"),
                                    sg!("オ"),
                                    sg!("か"),
                                    sg!("き"),
                                    sg!("く"),
                                    sg!("け"),
                                    sg!("こ"),
                                ),
                                actual,
                            );
                        }

                        #[test]
                        ///  あああああああああああああ
                        /// |ああああああああああああ>|
                        fn 行末で全角文字を表示する幅が足りないとき不等号を返す() {
                            let line = "アイウエオかきくけこ".styled_graphemes();

                            let iter = LineIterator::new(&line, 4, 15);

                            let actual: Vec<StyledGrapheme> = iter.cloned().collect();

                            assert_eq!(
                                expected!(
                                    sg!("ウ"),
                                    sg!("エ"),
                                    sg!("オ"),
                                    sg!("か"),
                                    sg!("き"),
                                    sg!("く"),
                                    sg!("け"),
                                    sg!(">"),
                                ),
                                actual,
                            );
                        }

                        #[test]
                        /// あああああああああああああ|
                        /// |<ああああああああああああ|
                        fn 行頭で全角文字を表示する幅が足りないとき不等号を返す() {
                            let line = "アイウエオかきくけこ".styled_graphemes();

                            let iter = LineIterator::new(&line, 3, 30);

                            let actual: Vec<StyledGrapheme> = iter.cloned().collect();

                            assert_eq!(
                                expected!(
                                    sg!("<"),
                                    sg!("ウ"),
                                    sg!("エ"),
                                    sg!("オ"),
                                    sg!("か"),
                                    sg!("き"),
                                    sg!("く"),
                                    sg!("け"),
                                    sg!("こ"),
                                ),
                                actual,
                            );
                        }
                    }
                }
            }
        }
    }
}
