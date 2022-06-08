mod render;
mod styled_graphemes;
mod wrap;

use std::rc::Rc;

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
    item::TextItem,
    render::{Render, Scroll},
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
    pub fn scroll_right(&mut self, i: usize) {
        if self.wrap {
            return;
        }

        self.scroll.x = self.scroll.x.saturating_add(i);
    }

    pub fn scroll_left(&mut self, i: usize) {
        if self.wrap {
            return;
        }

        self.scroll.x = self.scroll.x.saturating_sub(i);
    }

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
        self.inner_chunk = self.widget_config.block().inner(chunk);

        if self.wrap {
            self.item.rewrap(self.inner_chunk.width as usize);
        };
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
        let block = if let Some(block_injection) = &self.block_injection {
            (block_injection)(&*self, selected)
        } else {
            self.widget_config
                .render_block_with_title(self.focusable() && selected)
        };

        let wrapped_lines = self.item.wrapped();

        let lines: Vec<&[StyledGrapheme<'_>]> = wrapped_lines
            .iter()
            .map(|wrapped| wrapped.line.as_ref())
            .collect();

        let r = Render::builder()
            .block(block)
            .lines(&lines)
            .scroll(self.scroll)
            .build();

        f.render_widget(r, self.chunk);
    }
}

mod item {
    use super::{search::Search, styled_graphemes::StyledGraphemes, wrap::WrapTrait};
    use crate::tui_wrapper::widget::LiteralItem;
    use std::{borrow::Cow, ops::Range};
    use tui::{
        style::{Color, Modifier, Style},
        text::StyledGrapheme,
    };

    #[derive(Debug, Default)]
    struct Highlights {
        word: String,

        /// wordにマッチする場所に関するデータ
        item: Vec<Highlight>,

        /// 選択しているインデックス
        /// Reverseではなく背景色を変える
        index: usize,
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
                    highlights.item.extend(hls);
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
                    .flat_map(|g| g.wrap(self.wrap_width))
                    .collect();

                std::mem::transmute(wrapped)
            };

            if let Some(highlights) = &mut self.highlights {
                let hls: Vec<Highlight> = graphemes
                    .iter_mut()
                    .filter_map(|g| g.highlight_word(&highlights.word))
                    .flatten()
                    .collect();

                highlights.item.extend(hls);
            }

            self.item.extend(item);
            self.graphemes.extend(graphemes);
            self.wrapped.extend(wrapped);
        }
    }

    impl<'a> TextItem<'a> {
        pub fn highlight(&mut self, word: &str) {
            let highlight_words: Vec<_> = self
                .graphemes
                .iter_mut()
                .filter_map(|g| g.highlight_word(word))
                .flatten()
                .collect();

            if !highlight_words.is_empty() {
                let highlights = Highlights {
                    word: word.to_string(),
                    item: highlight_words,
                    index: 0,
                };

                self.highlights = Some(highlights);
            }
        }

        pub fn clear_highlight(&mut self) {
            if let Some(highlights) = &mut self.highlights {
                highlights.item.iter().for_each(|hl| {
                    let graphemes = &mut self.graphemes[hl.index];
                    graphemes.clear_highlight(hl);
                });
            }

            self.highlights = None;
        }

        /// 指定したインデックスを選択
        fn select_index_highlight(&mut self, index: usize) -> Option<usize> {
            if let Some(highlights) = &mut self.highlights {
                highlights.index = index;

                let hl = &highlights.item[index];

                let graphemes = &mut self.graphemes[hl.index].item[hl.range.clone()];

                graphemes
                    .iter_mut()
                    .for_each(|gs| gs.style = gs.style.fg(Color::Yellow));

                Some(hl.index)
            } else {
                None
            }
        }

        /// 指定したインデックスに一番近い場所を選択
        pub fn select_nearest_highlight(&mut self, scroll_index: usize) -> Option<usize> {
            let nearest_index = if let Some(highlights) = &mut self.highlights {
                let nearest_index = highlights
                    .item
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, hl)| hl.index.abs_diff(scroll_index))
                    .map(|(i, _)| i)
                    .unwrap();

                Some(nearest_index)
            } else {
                None
            };

            if let Some(index) = nearest_index {
                self.select_index_highlight(index)
            } else {
                None
            }
        }

        /// 次のマッチ箇所をハイライト
        pub fn select_next_highlight(&mut self) -> Option<usize> {
            if let Some(highlights) = &mut self.highlights {
                let index = highlights.index;

                let hl = &highlights.item[index];

                let graphemes = &mut self.graphemes[hl.index].item[hl.range.clone()];

                graphemes
                    .iter_mut()
                    .zip(hl.item.iter())
                    .for_each(|(gs, style)| gs.style = style.add_modifier(Modifier::REVERSED));

                let index = (index + 1) % highlights.item.len();

                let hl = &highlights.item[index];

                highlights.index = index;

                let graphemes = &mut self.graphemes[hl.index].item[hl.range.clone()];

                graphemes
                    .iter_mut()
                    .zip(hl.item.iter())
                    .for_each(|(gs, style)| gs.style = gs.style.fg(Color::Yellow));

                return Some(hl.index);
            } else {
                None
            }
        }

        /// 前のマッチ箇所をハイライト
        pub fn select_prev_highlight(&mut self) -> Option<usize> {
            if let Some(highlights) = &mut self.highlights {
                let index = highlights.index;

                let hl = &highlights.item[index];

                let graphemes = &mut self.graphemes[hl.index].item[hl.range.clone()];

                graphemes
                    .iter_mut()
                    .zip(hl.item.iter())
                    .for_each(|(gs, style)| gs.style = style.add_modifier(Modifier::REVERSED));

                let index = if index == 0 {
                    highlights.item.len().saturating_sub(1)
                } else {
                    index.saturating_sub(1)
                };

                let hl = &highlights.item[index];

                highlights.index = index;

                let graphemes = &mut self.graphemes[hl.index].item[hl.range.clone()];

                graphemes
                    .iter_mut()
                    .for_each(|gs| gs.style = gs.style.fg(Color::Yellow));

                Some(hl.index)
            } else {
                None
            }
        }
    }

    impl<'a> TextItem<'a> {
        pub fn wrapped(&self) -> &[WrappedLine<'a>] {
            &self.wrapped
        }

        pub fn rewrap(&mut self, wrap_width: usize) {
            self.wrap_width = Some(wrap_width);

            let wrapped: Vec<WrappedLine> = unsafe {
                let wrapped: Vec<WrappedLine> = self
                    .graphemes
                    .iter()
                    .enumerate()
                    .flat_map(|(i, g)| {
                        g.item
                            .wrap(self.wrap_width)
                            .map(|w| WrappedLine {
                                index: i,
                                line: Cow::Borrowed(w),
                            })
                            .collect::<Vec<WrappedLine>>()
                    })
                    .collect();

                std::mem::transmute(wrapped)
            };

            self.wrapped = wrapped;
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
    pub struct Highlight {
        /// 行番号
        index: usize,

        /// ハイライト箇所の範囲
        range: Range<usize>,

        /// ハイライト前のスタイル
        item: Vec<Style>,
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

        pub fn highlight_word(&mut self, word: &str) -> Option<Vec<Highlight>> {
            let word = word.styled_graphemes_symbols();

            if let Some(ranges) = self.item.search(&word) {
                let ret: Vec<Highlight> = ranges
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

                        Highlight {
                            index: self.index,
                            range,
                            item,
                        }
                    })
                    .collect();

                Some(ret)
            } else {
                None
            }
        }

        pub fn clear_highlight(&mut self, item: &Highlight) {
            let Highlight {
                index: _,
                range,
                item,
            } = item;

            let i = &mut self.item[range.clone()];

            i.iter_mut().zip(item.iter()).for_each(|(l, r)| {
                l.style = *r;
            });
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
                    highlight_words,
                    vec![Highlight {
                        index: 0,
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

                item.clear_highlight(&highlight[0]);

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
