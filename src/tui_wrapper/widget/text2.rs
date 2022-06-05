use std::{borrow::Cow, rc::Rc};

use crossterm::event::{KeyEvent, MouseEvent};
use derivative::*;
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

use self::{
    item::{TextItem, WrappedLine},
    render::{Render, Scroll},
    styled_graphemes::StyledGraphemes,
};

use super::{config::WidgetConfig, Item, LiteralItem, RenderTrait, SelectedItem, WidgetTrait};

type RenderBlockInjection = Rc<dyn Fn(&Text, bool) -> Block<'static>>;

#[derive(Derivative)]
#[derivative(Debug, Default)]
pub struct TextBuilder {
    id: String,
    widget_config: WidgetConfig,
    item: Vec<LiteralItem>,
    wrap: bool,
    follow: bool,
    #[derivative(Debug = "ignore")]
    block_injection: Option<RenderBlockInjection>,
}

impl TextBuilder {
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    pub fn widget_config(mut self, widget_config: WidgetConfig) -> Self {
        self.widget_config = widget_config;
        self
    }

    pub fn item(mut self, item: impl Into<Vec<LiteralItem>>) -> Self {
        self.item = item.into();
        self
    }

    pub fn wrap(mut self) -> Self {
        self.wrap = true;
        self
    }

    pub fn follow(mut self) -> Self {
        self.follow = true;
        self
    }

    pub fn build(self) -> Text<'static> {
        let ret = Text {
            id: self.id,
            widget_config: self.widget_config,
            item: TextItem::new(self.item, None),
            wrap: self.wrap,
            follow: self.follow,
            ..Default::default()
        };

        ret
    }
}

#[derive(Derivative)]
#[derivative(Debug, Default)]
pub struct Text<'a> {
    id: String,
    widget_config: WidgetConfig,
    item: TextItem<'a>,
    chunk: Rect,
    inner_chunk: Rect,
    wrap: bool,
    follow: bool,
    scroll: Scroll,
    #[derivative(Debug = "ignore")]
    block_injection: Option<RenderBlockInjection>,
}

impl Text<'_> {
    pub fn builder() -> TextBuilder {
        Default::default()
    }
}

/// ワード検索機能
///
/// # Features
///
/// - マッチした文字列をハイライト
/// - マッチした文字列に移動
/// - 検索モード終了時にハイライトを削除
impl Text<'_> {
    pub fn search(&mut self, word: &str) {
        self.item.highlight(word);
    }

    pub fn search_next(&mut self) {
        todo!()
    }

    pub fn search_prev(&mut self) {
        todo!()
    }

    pub fn search_cancel(&mut self) {
        self.item.clear_highlight();
    }
}

impl Text<'_> {
    pub fn scroll_y_last_index(&self) -> usize {
        self.item
            .wrapped()
            .len()
            .saturating_sub(self.inner_chunk.height as usize)
    }
}

impl<'a> WidgetTrait for Text<'_> {
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
        true
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

    fn select_next(&mut self, i: usize) {
        self.scroll.y = self
            .scroll
            .y
            .saturating_add(i)
            .min(self.scroll_y_last_index());
    }

    fn select_prev(&mut self, i: usize) {
        self.scroll.y = self.scroll.y.saturating_sub(i)
    }

    fn select_first(&mut self) {
        self.scroll.y = 0;
    }

    fn select_last(&mut self) {
        self.scroll.y = self.scroll_y_last_index();
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

    fn update_chunk(&mut self, chunk: Rect) {
        self.chunk = chunk;
    }

    fn clear(&mut self) {
        todo!()
    }
}

impl RenderTrait for Text<'_> {
    fn render<B>(&mut self, f: &mut Frame<'_, B>, selected: bool)
    where
        B: Backend,
    {
        let block = self.widget_config.render_block_with_title(selected);

        let item = vec![
            WrappedLine {
                index: 0,
                line: Cow::from("0123456789".styled_graphemes()),
            },
            WrappedLine {
                index: 1,
                line: Cow::from("0123456789".styled_graphemes()),
            },
        ];

        let lines: Vec<&[StyledGrapheme<'_>]> =
            item.iter().map(|wrapped| wrapped.line.as_ref()).collect();

        let r = Render::builder().block(block).lines(&lines).build();

        f.render_widget(r, self.chunk);
    }
}

mod styled_graphemes {

    use tui::{style::Style, text::StyledGrapheme};
    use unicode_segmentation::UnicodeSegmentation;

    use crate::{
        ansi::{AnsiEscapeSequence, TextParser},
        tui_wrapper::widget::ansi_color::Sgr,
    };

    pub trait StyledGraphemes {
        fn styled_graphemes(&self) -> Vec<StyledGrapheme<'_>>;
        fn styled_graphemes_symbols(&self) -> Vec<&'_ str>;
    }

    impl StyledGraphemes for String {
        fn styled_graphemes(&self) -> Vec<StyledGrapheme<'_>> {
            styled_graphemes(self)
        }

        fn styled_graphemes_symbols(&self) -> Vec<&'_ str> {
            styled_graphemes_symbols(self)
        }
    }

    impl StyledGraphemes for &String {
        fn styled_graphemes(&self) -> Vec<StyledGrapheme<'_>> {
            styled_graphemes(self)
        }

        fn styled_graphemes_symbols(&self) -> Vec<&'_ str> {
            styled_graphemes_symbols(self)
        }
    }

    impl StyledGraphemes for &str {
        fn styled_graphemes(&self) -> Vec<StyledGrapheme<'_>> {
            styled_graphemes(self)
        }

        fn styled_graphemes_symbols(&self) -> Vec<&'_ str> {
            styled_graphemes_symbols(self)
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
                    style = Sgr::from(sgr).into();
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

    pub fn styled_graphemes_symbols(s: &str) -> Vec<&'_ str> {
        s.ansi_parse()
            .filter_map(|p| match p.ty {
                AnsiEscapeSequence::Chars => Some(p.chars),
                _ => None,
            })
            .flat_map(|chars| chars.graphemes(true).collect::<Vec<_>>())
            .collect()
    }
}

mod item {
    use super::{search::Search, styled_graphemes::StyledGraphemes, wrap::WrapTrait};
    use crate::tui_wrapper::widget::LiteralItem;
    use std::{borrow::Cow, cell::RefCell, ops::Range, pin::Pin, rc::Rc};
    use tui::{
        style::{Modifier, Style},
        text::StyledGrapheme,
    };

    #[derive(Debug, Default)]
    struct Highlights {
        word: String,
        item: Vec<Highlight>,
    }

    #[derive(Debug, Default)]
    pub struct TextItem<'a> {
        item: Vec<LiteralItem>,
        /// graphemesに分割した文字列リスト
        graphemes: Vec<Graphemes<'a>>,

        /// 折り返しを考慮した描画のためのデータリスト
        /// item設定時に生成される
        wrapped: Vec<WrappedLine<'a>>,

        /// ハイライト情報
        /// - ハイライト箇所の復旧に使用
        /// - ハイライト箇所へのジャンプに使用
        highlights: Option<Highlights>,

        /// 折り返しサイズ
        wrap_width: Option<usize>,
    }

    /// WARNING:
    /// unsafeを用いて仮実装
    /// itemが変更されるとき、graphemes, wrapped, highlight_wordsも再生成すること
    impl TextItem<'_> {
        pub fn new(item: Vec<LiteralItem>, wrap_width: Option<usize>) -> Self {
            let graphemes: Vec<Graphemes> = unsafe {
                let graphemes: Vec<Graphemes> = item
                    .iter()
                    .enumerate()
                    .map(|(i, literal)| Graphemes::new(i, literal))
                    .collect();

                std::mem::transmute(graphemes)
            };

            let wrapped: Vec<WrappedLine> = unsafe {
                let wrapped: Vec<WrappedLine> = graphemes
                    .iter()
                    .enumerate()
                    .flat_map(|(i, g)| {
                        g.item
                            .wrap(wrap_width)
                            .map(|w| WrappedLine {
                                index: i,
                                line: Cow::Borrowed(w),
                            })
                            .collect::<Vec<WrappedLine>>()
                    })
                    .collect();

                std::mem::transmute(wrapped)
            };

            Self {
                item,
                graphemes,
                wrap_width,
                wrapped,
                highlights: None,
            }
        }

        pub fn push(&mut self, item: LiteralItem) {
            let mut graphemes: Graphemes =
                unsafe { std::mem::transmute(Graphemes::new(self.graphemes.len(), &item)) };

            let wrapped: Vec<WrappedLine> = unsafe {
                let wrapped: Vec<WrappedLine> = graphemes.wrap(self.wrap_width);

                std::mem::transmute(wrapped)
            };

            if let Some(highlights) = &mut self.highlights {
                if let Some(hls) = graphemes.highlight_word(&highlights.word) {
                    highlights.item.push(hls);
                }
            }

            self.item.push(item);
            self.graphemes.push(graphemes);
            self.wrapped.extend(wrapped);
        }

        pub fn extend(&mut self, item: Vec<LiteralItem>) {
            let mut graphemes: Vec<Graphemes> = unsafe {
                let graphemes: Vec<Graphemes> = item
                    .iter()
                    .enumerate()
                    .map(|(i, literal)| Graphemes::new(i + self.graphemes.len(), literal))
                    .collect();

                std::mem::transmute(graphemes)
            };

            let wrapped: Vec<WrappedLine> = unsafe {
                let wrapped: Vec<WrappedLine> = graphemes
                    .iter()
                    .enumerate()
                    .flat_map(|(i, g)| g.wrap(self.wrap_width))
                    .collect();

                std::mem::transmute(wrapped)
            };

            if let Some(highlights) = &mut self.highlights {
                let hls: Vec<Highlight> = graphemes
                    .iter_mut()
                    .filter_map(|g| g.highlight_word(&highlights.word))
                    .collect();

                highlights.item.extend(hls);
            }

            self.item.extend(item);
            self.graphemes.extend(graphemes);
            self.wrapped.extend(wrapped);
        }

        pub fn highlight(&mut self, word: &str) {
            let highlight_words: Vec<_> = self
                .graphemes
                .iter_mut()
                .filter_map(|g| g.highlight_word(word))
                .collect();

            if !highlight_words.is_empty() {
                let highlights = Highlights {
                    word: word.to_string(),
                    item: highlight_words,
                };

                self.highlights = Some(highlights);
            }
        }

        pub fn clear_highlight(&mut self) {
            if let Some(highlights) = &mut self.highlights {
                highlights.item.iter().for_each(|hl| {
                    let graphemes = &mut self.graphemes[hl.index];
                    graphemes.clear_highlight(&hl.item);
                });
            }

            self.highlights = None;
        }
    }

    impl<'a> TextItem<'a> {
        pub fn wrapped(&self) -> &[WrappedLine<'a>] {
            &self.wrapped
        }
    }

    /// Itemを折り返しとハイライトを考慮した構造体
    #[derive(Debug, Default, PartialEq)]
    pub struct WrappedLine<'a> {
        /// on_select時に渡すLiteralItemのインデックス = Item.itemのインデックス
        pub index: usize,

        /// 折り返しを計算した結果、表示する文字列データ
        pub line: Cow<'a, [StyledGrapheme<'a>]>,
    }
    #[derive(Debug, Default, PartialEq)]
    pub struct HighlightItem {
        /// ハイライト箇所の範囲
        range: Range<usize>,

        /// ハイライト前のスタイル
        item: Vec<Style>,
    }

    #[derive(Debug, Default)]
    pub struct Highlight {
        /// 行番号
        index: usize,

        /// ハイライトリスト
        item: Vec<HighlightItem>,
    }

    /// LiteralItem から Vec<StyledGrapheme> に変換する
    ///
    /// 文字列をパースしてスタイルを適用する
    #[derive(Debug, PartialEq)]
    pub struct Graphemes<'a> {
        /// 行番号
        index: usize,
        /// １行分の文字列
        item: Vec<StyledGrapheme<'a>>,
    }

    impl<'a> Graphemes<'a> {
        pub fn new(index: usize, literal: &'a LiteralItem) -> Self {
            Self {
                index,
                item: literal.item.styled_graphemes(),
            }
        }

        pub fn highlight_word(&mut self, word: &str) -> Option<Highlight> {
            let word = word.styled_graphemes_symbols();

            if let Some(ranges) = self.item.search(&word) {
                let item: Vec<HighlightItem> = ranges
                    .iter()
                    .cloned()
                    .map(|range| {
                        let item = self.item[range.clone()]
                            .iter_mut()
                            .map(|i| {
                                let ret = i.style;
                                i.style = i.style.add_modifier(Modifier::REVERSED);
                                ret
                            })
                            .collect();

                        HighlightItem { range, item }
                    })
                    .collect();

                Some(Highlight {
                    index: self.index,
                    item,
                })
            } else {
                None
            }
        }

        pub fn clear_highlight(&mut self, item: &[HighlightItem]) {
            item.iter().for_each(|HighlightItem { range, item }| {
                let i = &mut self.item[range.clone()];
                i.iter_mut().zip(item.iter()).for_each(|(l, r)| {
                    l.style = *r;
                });
            })
        }

        pub fn wrap(&self, wrap_width: Option<usize>) -> Vec<WrappedLine> {
            self.item
                .wrap(wrap_width)
                .map(|w| WrappedLine {
                    index: self.index,
                    line: Cow::Borrowed(w),
                })
                .collect()
        }
    }

    #[cfg(test)]
    mod tests {
        use pretty_assertions::assert_eq;
        use tui::style::{Modifier, Style};

        use super::*;

        mod text_item {
            use pretty_assertions::assert_eq;

            use super::*;

            #[test]
            fn new() {
                let item = LiteralItem::new("0123456789", None);
                let item = TextItem::new(vec![item], Some(5));

                let actual = item.wrapped;

                let expected = vec![
                    WrappedLine {
                        index: 0,
                        line: Cow::Owned("01234".styled_graphemes()),
                    },
                    WrappedLine {
                        index: 0,
                        line: Cow::Owned("56789".styled_graphemes()),
                    },
                ];

                assert_eq!(actual, expected);
            }

            #[test]
            fn push() {
                let item = LiteralItem::new("0123456789", None);
                let mut item = TextItem::new(vec![item], Some(5));
                item.push(LiteralItem::new("0123456789", None));

                let actual = item.wrapped;

                let expected = vec![
                    WrappedLine {
                        index: 0,
                        line: Cow::Owned("01234".styled_graphemes()),
                    },
                    WrappedLine {
                        index: 0,
                        line: Cow::Owned("56789".styled_graphemes()),
                    },
                    WrappedLine {
                        index: 1,
                        line: Cow::Owned("01234".styled_graphemes()),
                    },
                    WrappedLine {
                        index: 1,
                        line: Cow::Owned("56789".styled_graphemes()),
                    },
                ];

                assert_eq!(actual, expected);
            }

            #[test]
            fn extend() {
                let item = LiteralItem::new("0123456789", None);
                let mut item = TextItem::new(vec![item], Some(5));
                item.extend(vec![
                    LiteralItem::new("0123456789", None),
                    LiteralItem::new("あいうえ", None),
                ]);

                let actual = item.wrapped;

                let expected = vec![
                    WrappedLine {
                        index: 0,
                        line: Cow::Owned("01234".styled_graphemes()),
                    },
                    WrappedLine {
                        index: 0,
                        line: Cow::Owned("56789".styled_graphemes()),
                    },
                    WrappedLine {
                        index: 1,
                        line: Cow::Owned("01234".styled_graphemes()),
                    },
                    WrappedLine {
                        index: 1,
                        line: Cow::Owned("56789".styled_graphemes()),
                    },
                    WrappedLine {
                        index: 2,
                        line: Cow::Owned("あい".styled_graphemes()),
                    },
                    WrappedLine {
                        index: 2,
                        line: Cow::Owned("うえ".styled_graphemes()),
                    },
                ];

                assert_eq!(actual, expected);
            }

            #[test]
            fn highlight() {
                let mut item = TextItem::new(
                    vec![
                        LiteralItem::new("hello world", None),
                        LiteralItem::new("hoge world", None),
                    ],
                    Some(5),
                );

                item.highlight("world");

                let actual = item.graphemes;

                let mut expected_1 = "hello world".styled_graphemes();
                expected_1[6].style = expected_1[6].style.add_modifier(Modifier::REVERSED);
                expected_1[7].style = expected_1[7].style.add_modifier(Modifier::REVERSED);
                expected_1[8].style = expected_1[8].style.add_modifier(Modifier::REVERSED);
                expected_1[9].style = expected_1[9].style.add_modifier(Modifier::REVERSED);
                expected_1[10].style = expected_1[10].style.add_modifier(Modifier::REVERSED);

                let mut expected_2 = "hoge world".styled_graphemes();
                expected_2[5].style = expected_2[5].style.add_modifier(Modifier::REVERSED);
                expected_2[6].style = expected_2[6].style.add_modifier(Modifier::REVERSED);
                expected_2[7].style = expected_2[7].style.add_modifier(Modifier::REVERSED);
                expected_2[8].style = expected_2[8].style.add_modifier(Modifier::REVERSED);
                expected_2[9].style = expected_2[9].style.add_modifier(Modifier::REVERSED);

                let expected = vec![
                    Graphemes {
                        index: 0,
                        item: expected_1,
                    },
                    Graphemes {
                        index: 1,
                        item: expected_2,
                    },
                ];

                assert_eq!(actual, expected);
            }

            #[test]
            fn clear_highlight() {
                let mut item = TextItem::new(
                    vec![
                        LiteralItem::new("hello world", None),
                        LiteralItem::new("hoge world", None),
                    ],
                    Some(5),
                );

                item.highlight("world");
                item.clear_highlight();

                let actual = item.graphemes;

                let expected_1 = "hello world".styled_graphemes();
                let expected_2 = "hoge world".styled_graphemes();
                let expected = vec![
                    Graphemes {
                        index: 0,
                        item: expected_1,
                    },
                    Graphemes {
                        index: 1,
                        item: expected_2,
                    },
                ];

                assert_eq!(actual, expected);
            }
        }

        mod graphemes {
            use pretty_assertions::assert_eq;

            use super::*;

            #[test]
            fn 指定された文字列にマッチするときその文字列を退避してハイライトする() {
                let item = LiteralItem {
                    item: "hello world".to_string(),
                    ..Default::default()
                };
                let mut item = Graphemes::new(0, &item);

                let highlight_words = item.highlight_word("hello").unwrap();

                assert_eq!(
                    highlight_words.item,
                    vec![HighlightItem {
                        range: 0..5,
                        item: vec![
                            Style::default(),
                            Style::default(),
                            Style::default(),
                            Style::default(),
                            Style::default(),
                        ],
                    }]
                );

                assert_eq!(
                    item.item,
                    vec![
                        StyledGrapheme {
                            symbol: "h",
                            style: Style::default().add_modifier(Modifier::REVERSED),
                        },
                        StyledGrapheme {
                            symbol: "e",
                            style: Style::default().add_modifier(Modifier::REVERSED),
                        },
                        StyledGrapheme {
                            symbol: "l",
                            style: Style::default().add_modifier(Modifier::REVERSED),
                        },
                        StyledGrapheme {
                            symbol: "l",
                            style: Style::default().add_modifier(Modifier::REVERSED),
                        },
                        StyledGrapheme {
                            symbol: "o",
                            style: Style::default().add_modifier(Modifier::REVERSED),
                        },
                        StyledGrapheme {
                            symbol: " ",
                            style: Style::default(),
                        },
                        StyledGrapheme {
                            symbol: "w",
                            style: Style::default(),
                        },
                        StyledGrapheme {
                            symbol: "o",
                            style: Style::default(),
                        },
                        StyledGrapheme {
                            symbol: "r",
                            style: Style::default(),
                        },
                        StyledGrapheme {
                            symbol: "l",
                            style: Style::default(),
                        },
                        StyledGrapheme {
                            symbol: "d",
                            style: Style::default(),
                        },
                    ],
                );
            }

            #[test]
            fn 指定された文字列にマッチしないときハイライトしない() {
                let item = LiteralItem {
                    item: "hello world".to_string(),
                    ..Default::default()
                };
                let mut item = Graphemes::new(0, &item);

                let highlight_words = item.highlight_word("hoge");

                assert_eq!(highlight_words.is_none(), true);

                assert_eq!(
                    item.item,
                    vec![
                        StyledGrapheme {
                            symbol: "h",
                            style: Style::default(),
                        },
                        StyledGrapheme {
                            symbol: "e",
                            style: Style::default(),
                        },
                        StyledGrapheme {
                            symbol: "l",
                            style: Style::default(),
                        },
                        StyledGrapheme {
                            symbol: "l",
                            style: Style::default(),
                        },
                        StyledGrapheme {
                            symbol: "o",
                            style: Style::default(),
                        },
                        StyledGrapheme {
                            symbol: " ",
                            style: Style::default(),
                        },
                        StyledGrapheme {
                            symbol: "w",
                            style: Style::default(),
                        },
                        StyledGrapheme {
                            symbol: "o",
                            style: Style::default(),
                        },
                        StyledGrapheme {
                            symbol: "r",
                            style: Style::default(),
                        },
                        StyledGrapheme {
                            symbol: "l",
                            style: Style::default(),
                        },
                        StyledGrapheme {
                            symbol: "d",
                            style: Style::default(),
                        },
                    ],
                );
            }

            #[test]
            fn ハイライトを削除したときスタイルをもとに戻す() {
                let item = LiteralItem {
                    // cSpell: disable-next-line
                    item: "\x1b[31mhello\x1b[0m world".to_string(),
                    ..Default::default()
                };
                let mut item = Graphemes::new(0, &item);

                let highlight = item.highlight_word("hello").unwrap();

                item.clear_highlight(&highlight.item);

                assert_eq!(
                    item.item,
                    vec![
                        StyledGrapheme {
                            symbol: "h",
                            style: Style::default().fg(tui::style::Color::Red),
                        },
                        StyledGrapheme {
                            symbol: "e",
                            style: Style::default().fg(tui::style::Color::Red),
                        },
                        StyledGrapheme {
                            symbol: "l",
                            style: Style::default().fg(tui::style::Color::Red),
                        },
                        StyledGrapheme {
                            symbol: "l",
                            style: Style::default().fg(tui::style::Color::Red),
                        },
                        StyledGrapheme {
                            symbol: "o",
                            style: Style::default().fg(tui::style::Color::Red),
                        },
                        StyledGrapheme {
                            symbol: " ",
                            style: Style::reset(),
                        },
                        StyledGrapheme {
                            symbol: "w",
                            style: Style::reset(),
                        },
                        StyledGrapheme {
                            symbol: "o",
                            style: Style::reset(),
                        },
                        StyledGrapheme {
                            symbol: "r",
                            style: Style::reset(),
                        },
                        StyledGrapheme {
                            symbol: "l",
                            style: Style::reset(),
                        },
                        StyledGrapheme {
                            symbol: "d",
                            style: Style::reset(),
                        },
                    ],
                );
            }
        }
    }
}

mod search {
    use std::ops::Range;

    use tui::text::StyledGrapheme;

    pub trait Search {
        fn search(&self, word: &[&str]) -> Option<Vec<Range<usize>>>;
    }

    impl Search for Vec<&str> {
        fn search(&self, word: &[&str]) -> Option<Vec<Range<usize>>> {
            let mut match_list = Vec::new();

            let word_len = word.len();
            let line_len = self.len();
            for i in 0..line_len {
                let range = i..(i + word_len);

                if let Some(target) = self.get(range.clone()) {
                    if target == word {
                        match_list.push(range);
                    }
                }
            }

            if !match_list.is_empty() {
                Some(match_list)
            } else {
                None
            }
        }
    }

    impl<'a> Search for Vec<StyledGrapheme<'a>> {
        fn search(&self, word: &[&str]) -> Option<Vec<Range<usize>>> {
            let mut match_list = Vec::new();

            let word_len = word.len();
            let line_len = self.len();
            for i in 0..line_len {
                let range = i..(i + word_len);

                if let Some(target) = self.get(range.clone()) {
                    if target.iter().zip(word.iter()).all(|(t, w)| &t.symbol == w) {
                        match_list.push(range);
                    }
                }
            }

            if !match_list.is_empty() {
                Some(match_list)
            } else {
                None
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use crate::tui_wrapper::widget::text2::styled_graphemes::StyledGraphemes;

        use super::*;

        mod styled_graphemes {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn 指定ワードにマッチしたとき範囲のリストを返す() {
                let line = "hello world. hello world.".styled_graphemes();

                let word = "hello".styled_graphemes_symbols();

                let actual = line.search(&word);

                let expected = Some(vec![0..5, 13..18]);

                assert_eq!(actual, expected);
            }

            #[test]
            fn 指定ワードにマッチしないときnoneを返す() {
                let line = "hello world. hello world.".styled_graphemes();

                let word = "hogehoge".styled_graphemes_symbols();

                let actual = line.search(&word);

                let expected = None;

                assert_eq!(actual, expected);
            }
        }

        mod vec {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn 指定ワードにマッチしたとき範囲のリストを返す() {
                let line = "hello world. hello world.".styled_graphemes_symbols();

                let word = "hello".styled_graphemes_symbols();

                let actual = line.search(&word);

                let expected = Some(vec![0..5, 13..18]);

                assert_eq!(actual, expected);
            }

            #[test]
            fn 指定ワードにマッチしないときnoneを返す() {
                let line = "hello world. hello world.".styled_graphemes_symbols();

                let word = "hogehoge".styled_graphemes_symbols();

                let actual = line.search(&word);

                let expected = None;

                assert_eq!(actual, expected);
            }
        }
    }
}

mod wrap {

    use tui::text::StyledGrapheme;
    use unicode_width::UnicodeWidthStr;

    #[derive(Debug)]
    pub struct Wrap<'a> {
        /// 折り返し計算をする文字列リスト
        line: &'a [StyledGrapheme<'a>],

        /// 折り返し幅
        wrap_width: Option<usize>,
    }

    pub trait WrapTrait {
        fn wrap(&self, wrap_width: Option<usize>) -> Wrap;
    }

    impl WrapTrait for Vec<StyledGrapheme<'_>> {
        fn wrap(&self, wrap_width: Option<usize>) -> Wrap {
            Wrap {
                line: self,
                wrap_width,
            }
        }
    }

    impl<'a> Iterator for Wrap<'a> {
        type Item = &'a [StyledGrapheme<'a>];
        fn next(&mut self) -> Option<Self::Item> {
            if self.line.is_empty() {
                return None;
            }

            if let Some(wrap_width) = self.wrap_width {
                let WrapResult { wrapped, remaining } = wrap(self.line, wrap_width);

                self.line = remaining;

                Some(wrapped)
            } else {
                let ret = self.line;

                self.line = &[];

                Some(ret)
            }
        }
    }

    #[derive(Debug, PartialEq)]
    struct WrapResult<'a> {
        wrapped: &'a [StyledGrapheme<'a>],
        remaining: &'a [StyledGrapheme<'a>],
    }

    fn wrap<'a>(line: &'a [StyledGrapheme<'a>], wrap_width: usize) -> WrapResult {
        let mut result = WrapResult {
            wrapped: line,
            remaining: &[],
        };

        let mut sum = 0;
        for (i, sg) in line.iter().enumerate() {
            let width = sg.symbol.width();

            if wrap_width < sum + width {
                result = WrapResult {
                    wrapped: &line[..i],
                    remaining: &line[i..],
                };
                break;
            }

            sum += width;
        }

        result
    }

    #[cfg(test)]
    mod tests {
        use pretty_assertions::assert_eq;

        use crate::tui_wrapper::widget::text2::styled_graphemes::StyledGraphemes;

        use super::*;

        #[test]
        fn 折り返しなしのときlinesを1行ずつ生成する() {
            let line = "abc".styled_graphemes();

            let actual = line.wrap(None).collect::<Vec<_>>();

            let expected = vec!["abc".styled_graphemes()];

            assert_eq!(actual, expected);
        }

        mod wrap {
            use super::*;

            use pretty_assertions::assert_eq;

            #[test]
            fn has_remaining() {
                let line: Vec<StyledGrapheme> = "0123456789".styled_graphemes();

                let result = wrap(&line, 5);

                assert_eq!(
                    result,
                    WrapResult {
                        wrapped: &line[..5],
                        remaining: &line[5..]
                    }
                );
            }

            #[test]
            fn no_remaining() {
                let line: Vec<StyledGrapheme> = "0123456789".styled_graphemes();

                let result = wrap(&line, 10);

                assert_eq!(
                    result,
                    WrapResult {
                        wrapped: &line,
                        remaining: &[]
                    }
                );
            }
        }

        mod 半角 {
            use super::*;

            use pretty_assertions::assert_eq;

            #[test]
            fn 折り返しのとき指定した幅に収まるリストを返す() {
                let line = "0123456789".styled_graphemes();

                let actual = line.wrap(Some(5)).collect::<Vec<_>>();

                let expected = vec!["01234".styled_graphemes(), "56789".styled_graphemes()];

                assert_eq!(actual, expected);
            }
        }

        mod 全角 {
            use super::*;

            use pretty_assertions::assert_eq;

            #[test]
            fn 折り返しのとき指定した幅に収まるリストを返す() {
                let line = "アイウエオかきくけこ".styled_graphemes();

                let actual = line.wrap(Some(11)).collect::<Vec<_>>();

                let expected = vec![
                    "アイウエオ".styled_graphemes(),
                    "かきくけこ".styled_graphemes(),
                ];

                assert_eq!(actual, expected);
            }
        }
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
    pub struct Scroll {
        pub x: usize,
        pub y: usize,
    }

    #[derive(Debug, Default, Clone)]
    pub struct Render<'a> {
        block: Block<'a>,
        lines: &'a [&'a [StyledGrapheme<'a>]],
        scroll: Scroll,
    }

    pub struct RenderBuilder<'a>(Render<'a>);

    impl<'a> RenderBuilder<'a> {
        pub fn block(mut self, block: Block<'a>) -> Self {
            self.0.block = block;
            self
        }

        pub fn lines(mut self, lines: &'a [&'a [StyledGrapheme<'a>]]) -> Self {
            self.0.lines = lines;
            self
        }

        pub fn scroll(mut self, scroll: Scroll) -> Self {
            self.0.scroll = scroll;
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
