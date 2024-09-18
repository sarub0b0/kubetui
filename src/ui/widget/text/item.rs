use super::wrap::WrapTrait;
use crate::ui::widget::{
    styled_graphemes::{StyledGrapheme, StyledGraphemes},
    LiteralItem,
};
use ratatui::style::{Color, Modifier, Style};
use std::ops::Range;

use search::Search;

#[inline]
fn highlight_style() -> Style {
    Style::default().add_modifier(Modifier::REVERSED)
}

#[inline]
fn selected_highlight_style() -> Style {
    Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::REVERSED)
}

#[derive(Debug, Clone, PartialEq)]
pub struct Highlight {
    /// Graphemesのインデックス
    ///
    /// ハイライトを戻すときに使う
    line_index: usize,

    /// ハイライト箇所の範囲
    range: Range<usize>,

    /// ハイライト前のスタイル
    styles: Vec<Style>,

    /// スクロールに使う
    line_number: usize,
}

#[derive(Debug, Clone, Default)]
struct Highlights {
    /// 検索ワード
    word: String,

    /// wordにマッチする場所に関するデータ
    item: Vec<Highlight>,

    /// 選択しているインデックス
    selected_index: usize,
}

#[derive(Debug, Default)]
pub struct TextItem {
    /// 1行分のgraphemesに分割した文字列リスト
    lines: Vec<Line>,

    /// 折り返しを考慮した描画のためのデータリスト
    /// item設定時に生成される
    wrapped_lines: Vec<WrappedLine>,

    /// ハイライト情報
    /// - ハイライト箇所の復旧に使用
    /// - ハイライト箇所へのジャンプに使用
    highlights: Option<Highlights>,

    /// 折り返しサイズ
    wrap_width: Option<usize>,

    /// 1行あたりの最大文字数
    max_chars: usize,
}

type Graphemes = Vec<StyledGrapheme>;
type Wrappers<'a> = Vec<*const [StyledGrapheme]>;

impl TextItem {
    pub fn new(literal_item: Vec<LiteralItem>, wrap_width: Option<usize>) -> Self {
        let (lines, wrapped_lines) = Self::new_or_extend(literal_item, wrap_width, 0, 0);

        let wrapped_lines: Vec<_> = wrapped_lines.into_iter().flatten().collect();

        let max_chars = wrapped_lines
            .iter()
            .map(|l| l.line().len())
            .max()
            .unwrap_or_default();

        Self {
            lines,
            wrapped_lines,
            highlights: None,
            wrap_width,
            max_chars,
        }
    }

    pub fn update(&mut self, item: Vec<LiteralItem>) {
        let wrap_width = self.wrap_width;
        let highlights = self.highlights.clone();

        let mut new = Self::new(item, wrap_width);

        if let Some(highlights) = highlights {
            let prev_line_number = highlights.item[highlights.selected_index].line_number;

            new.highlight(&highlights.word);

            new.select_nearest_highlight(prev_line_number);
        }

        *self = new;
    }

    pub fn push(&mut self, item: LiteralItem) {
        let graphemes = item.item.styled_graphemes();

        let line_number = self.wrapped_lines.len();

        #[allow(clippy::needless_collect)]
        let wrappers: Wrappers = graphemes
            .wrap(self.wrap_width)
            .map(|w| w as *const [StyledGrapheme])
            .collect();

        let line_index = self.lines.len();

        let wrapped_lines: Vec<WrappedLine> = wrappers
            .into_iter()
            .map(|w| WrappedLine {
                line_index,
                slice_ptr: w,
            })
            .collect();

        self.max_chars = wrapped_lines
            .iter()
            .map(|l| l.line().len())
            .max()
            .unwrap_or_default()
            .max(self.max_chars);

        let line = Line {
            line_index,
            line_number,
            literal_item: item,
            graphemes,
            wrapped_lines: line_number..(line_number + wrapped_lines.len()),
        };

        self.lines.push(line);
        self.wrapped_lines.extend(wrapped_lines);

        if let Some(highlights) = &mut self.highlights {
            let pushed_line_index = self.lines.len() - 1;
            let line = &mut self.lines[pushed_line_index];

            if let Some(hls) = line.highlight_word(
                &highlights.word,
                &self.wrapped_lines[line.wrapped_lines.clone()],
            ) {
                highlights.item.extend(hls);
            }
        }
    }

    pub fn extend(&mut self, item: Vec<LiteralItem>) {
        let extend_len = item.len();

        let (lines, wrapped_lines) = Self::new_or_extend(
            item,
            self.wrap_width,
            self.wrapped_lines.len(),
            self.lines.len(),
        );

        self.max_chars = wrapped_lines
            .iter()
            .flatten()
            .map(|l| l.line().len())
            .max()
            .unwrap_or_default()
            .max(self.max_chars);

        self.lines.extend(lines);
        self.wrapped_lines
            .extend(wrapped_lines.into_iter().flatten());

        if let Some(highlights) = &mut self.highlights {
            let lines_len = self.lines.len();
            let lines = &mut self.lines[(lines_len - extend_len)..];

            let hls: Vec<Highlight> = lines
                .iter_mut()
                .filter_map(|line| {
                    line.highlight_word(
                        &highlights.word,
                        &self.wrapped_lines[line.wrapped_lines.clone()],
                    )
                })
                .flatten()
                .collect();

            highlights.item.extend(hls);
        }
    }

    /// Vec<LiteralItem>からLine, WrappedLineを生成する
    /// extendにも対応できるようインデックスに関する引数を追加している
    ///
    /// start_line_number: 新しく作成されるLineの開始行番号
    /// lines_len: 既存のLineの長さ
    fn new_or_extend(
        literal_item: Vec<LiteralItem>,
        wrap_width: Option<usize>,
        start_line_number: usize,
        lines_len: usize,
    ) -> (Vec<Line>, Vec<Vec<WrappedLine>>) {
        let graphemes_list: Vec<Graphemes> = literal_item
            .iter()
            .map(|item| item.item.styled_graphemes())
            .collect();

        #[allow(clippy::needless_collect)]
        let wrappers_list: Vec<Wrappers> = graphemes_list
            .iter()
            .map(|g| {
                g.wrap(wrap_width)
                    .map(|w| w as *const [StyledGrapheme])
                    .collect()
            })
            .collect();

        let item_len = literal_item.len();

        let mut lines = Vec::with_capacity(item_len);
        let mut wrapped_lines = Vec::with_capacity(item_len);

        let mut line_number = start_line_number;

        graphemes_list
            .into_iter()
            .zip(wrappers_list)
            .zip(literal_item)
            .enumerate()
            .for_each(|(i, ((graphemes, wrapped), literal_item))| {
                let wrapped_len = wrapped.len();
                let line_index = lines_len + i;

                let new_wrapped_lines: Vec<WrappedLine> = wrapped
                    .into_iter()
                    .map(|w| WrappedLine {
                        line_index,
                        slice_ptr: w,
                    })
                    .collect();

                let line = Line {
                    line_index,
                    line_number,
                    literal_item,
                    graphemes,
                    wrapped_lines: line_number..(line_number + wrapped_len),
                };

                lines.push(line);
                wrapped_lines.push(new_wrapped_lines);

                line_number += wrapped_len;
            });

        (lines, wrapped_lines)
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    pub fn max_chars(&self) -> usize {
        self.max_chars
    }
}

impl TextItem {
    pub fn highlight(&mut self, word: &str) {
        self.clear_highlight();

        let highlight_words: Vec<_> = self
            .lines
            .iter_mut()
            .filter_map(|line| {
                line.highlight_word(word, &self.wrapped_lines[line.wrapped_lines.clone()])
            })
            .flatten()
            .collect();

        if !highlight_words.is_empty() {
            let highlights = Highlights {
                word: word.to_string(),
                item: highlight_words,
                selected_index: 0,
            };

            self.highlights = Some(highlights);
        }
    }

    pub fn clear_highlight(&mut self) {
        if let Some(highlights) = &mut self.highlights {
            highlights.item.iter().for_each(|hl| {
                let line = &mut self.lines[hl.line_index];
                line.clear_highlight(hl.range.clone(), &hl.styles);
            });
        }

        self.highlights = None;
    }

    fn highlight_normal(&mut self, index: usize) {
        if let Some(highlights) = &mut self.highlights {
            let hl = &highlights.item[index];

            let line = &mut self.lines[hl.line_index];
            let graphemes = &mut line.graphemes[hl.range.clone()];

            graphemes
                .iter_mut()
                .for_each(|gs| *gs.style_mut() = highlight_style());
        }
    }

    fn highlight_color(&mut self, index: usize) -> Option<usize> {
        if let Some(highlights) = &mut self.highlights {
            let hl = &highlights.item[index];

            let line = &mut self.lines[hl.line_index];
            let graphemes = &mut line.graphemes[hl.range.clone()];

            graphemes
                .iter_mut()
                .for_each(|gs| *gs.style_mut() = selected_highlight_style());

            highlights.selected_index = index;

            Some(hl.line_number)
        } else {
            None
        }
    }

    pub fn select_nearest_highlight(&mut self, scroll_index: usize) -> Option<usize> {
        if let Some(highlights) = &mut self.highlights {
            let index = highlights.selected_index;

            let nearest_index = highlights
                .item
                .iter()
                .enumerate()
                .min_by_key(|(_, hl)| hl.line_number.abs_diff(scroll_index))
                .map(|(i, _)| i)
                .unwrap_or(0);

            self.highlight_normal(index);

            self.highlight_color(nearest_index)
        } else {
            None
        }
    }

    pub fn select_next_highlight(&mut self) -> Option<usize> {
        if let Some(highlights) = &mut self.highlights {
            let index = highlights.selected_index;

            let item_len = highlights.item.len();

            self.highlight_normal(index);

            let index = (index + 1) % item_len;

            self.highlight_color(index)
        } else {
            None
        }
    }

    pub fn select_prev_highlight(&mut self) -> Option<usize> {
        if let Some(highlights) = &mut self.highlights {
            let index = highlights.selected_index;

            let item_len = highlights.item.len();

            self.highlight_normal(index);

            let index = if index == 0 {
                item_len.saturating_sub(1)
            } else {
                index.saturating_sub(1)
            };

            self.highlight_color(index)
        } else {
            None
        }
    }

    pub fn highlight_status(&self) -> (usize, usize) {
        if let Some(highlights) = &self.highlights {
            (highlights.selected_index + 1, highlights.item.len())
        } else {
            (0, 0)
        }
    }

    pub fn highlight_selected_line_number(&self) -> Option<usize> {
        self.highlights
            .as_ref()
            .map(|h| h.item[h.selected_index].line_number)
    }
}

impl TextItem {
    pub fn wrapped_lines(&self) -> &[WrappedLine] {
        &self.wrapped_lines
    }

    pub fn rewrap(&mut self, wrap_width: usize) {
        self.wrap_width = Some(wrap_width);

        #[allow(clippy::needless_collect)]
        let wrappers_list: Vec<Wrappers> = self
            .lines
            .iter()
            .map(|line| {
                line.graphemes
                    .wrap(self.wrap_width)
                    .map(|w| w as *const [StyledGrapheme])
                    .collect()
            })
            .collect();

        let mut wrapped_lines = Vec::with_capacity(wrappers_list.len());
        let mut line_number = 0;
        self.lines
            .iter_mut()
            .zip(wrappers_list)
            .enumerate()
            .for_each(|(i, (line, wrapped))| {
                let wrapped_len = wrapped.len();

                line.line_number = line_number;
                line.wrapped_lines = line_number..(line_number + wrapped_len);

                let new_wrapped_lines: Vec<WrappedLine> = wrapped
                    .into_iter()
                    .map(|w| WrappedLine {
                        line_index: i,
                        slice_ptr: w,
                    })
                    .collect();

                wrapped_lines.push(new_wrapped_lines);

                line_number += wrapped_len;
            });

        self.wrapped_lines = wrapped_lines.into_iter().flatten().collect();

        if let Some(highlights) = &mut self.highlights {
            highlights.item.iter_mut().for_each(|hl| {
                let line = &self.lines[hl.line_index];

                hl.line_number = highlight_line_number(
                    hl.range.start,
                    &self.wrapped_lines[line.wrapped_lines.clone()],
                    line.line_number,
                );
            });
        }
    }
}

#[derive(Debug)]
struct Line {
    line_index: usize,
    /// 折り返しを考慮した１行目となる行番号
    line_number: usize,

    /// ベースとなる１行分の文字列データ
    ///
    /// この文字列のポインターを駆使していく
    #[allow(dead_code)]
    literal_item: LiteralItem,

    /// 目でみたときの１文字ずつに分割した配列
    graphemes: Vec<StyledGrapheme>,

    /// この１行を折り返したWrappedLineのスライス
    ///
    /// ワード検索でスクロール位置を割り出すのに使う
    /// TextItem.wrappedのポインターをもつ
    wrapped_lines: Range<usize>,
}

#[allow(clippy::derivable_impls)]
impl Default for Line {
    fn default() -> Self {
        Self {
            line_number: 0,
            literal_item: Default::default(),
            graphemes: Default::default(),
            wrapped_lines: Default::default(),
            line_index: 0,
        }
    }
}

impl Line {
    pub fn highlight_word(
        &mut self,
        word: &str,
        wrapped_lines: &[WrappedLine],
    ) -> Option<Vec<Highlight>> {
        let word = word.styled_graphemes_symbols();

        if let Some(ranges) = self.graphemes.search(&word) {
            let ret: Vec<Highlight> = ranges
                .iter()
                .cloned()
                .map(|range| {
                    let styles = self.graphemes[range.clone()]
                        .iter_mut()
                        .map(|i| {
                            let ret = *i.style();
                            *i.style_mut() = highlight_style();
                            ret
                        })
                        .collect();

                    let line_number =
                        highlight_line_number(range.start, wrapped_lines, self.line_number);

                    Highlight {
                        line_index: self.line_index,
                        range,
                        styles,
                        line_number,
                    }
                })
                .collect();

            Some(ret)
        } else {
            None
        }
    }

    pub fn clear_highlight(&mut self, range: Range<usize>, styles: &[Style]) {
        let i = &mut self.graphemes[range];

        i.iter_mut().zip(styles.iter()).for_each(|(l, r)| {
            *l.style_mut() = *r;
        });
    }
}

/// start_indexが含まれる行番号を求める
fn highlight_line_number(
    start_index: usize,
    wrapped_lines: &[WrappedLine],
    mut line_number: usize,
) -> usize {
    let mut grapheme_len = 0;

    for w in wrapped_lines {
        grapheme_len += w.line().len();
        if grapheme_len < start_index {
            line_number += 1;
        } else {
            break;
        }
    }

    line_number
}

#[derive(Debug)]
pub struct WrappedLine {
    /// Lineのptr
    /// 範囲選択された文字列を連結する際に１行で区切りたいために使用する
    line_index: usize,

    /// 折り返しを考慮したgraphemesのスライス
    /// 描画に使用する
    /// TextItem.graphemesのポインターをもつ
    slice_ptr: *const [StyledGrapheme],
}

impl WrappedLine {
    #[cfg(test)]
    pub fn new(line_index: usize, slice: &[StyledGrapheme]) -> Self {
        Self {
            line_index,
            slice_ptr: slice,
        }
    }

    #[inline]
    pub fn index(&self) -> usize {
        self.line_index
    }

    #[inline]
    pub fn line(&self) -> &[StyledGrapheme] {
        unsafe { &*self.slice_ptr }
    }
}

#[allow(clippy::derivable_impls)]
impl Default for WrappedLine {
    fn default() -> Self {
        Self {
            line_index: 0,
            slice_ptr: &[],
        }
    }
}

#[cfg(test)]
mod tests {
    use ratatui::style::{Modifier, Style};

    use super::*;

    mod text_item {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn new() {
            let item = LiteralItem::new("0123456789", None);
            let item = TextItem::new(vec![item], Some(5));

            let lines = item.lines;
            let wrapped_lines = item.wrapped_lines;

            assert_eq!(lines[0].line_index, 0);
            assert_eq!(lines[0].line_number, 0);
            assert_eq!(lines[0].wrapped_lines, 0..2);

            assert_eq!(wrapped_lines[0].line_index, 0);
            assert_eq!(wrapped_lines[0].slice_ptr, &lines[0].graphemes[..5]);
            assert_eq!(wrapped_lines[1].line_index, 0);
            assert_eq!(wrapped_lines[1].slice_ptr, &lines[0].graphemes[5..]);
        }

        #[test]
        fn push() {
            let item = LiteralItem::new("0123456789", None);
            let mut item = TextItem::new(vec![item], Some(5));
            item.push(LiteralItem::new("0123456789", None));

            let lines = item.lines;
            let wrapped_lines = item.wrapped_lines;

            assert_eq!(lines[0].line_index, 0);
            assert_eq!(lines[0].line_number, 0);
            assert_eq!(lines[0].wrapped_lines, 0..2);

            assert_eq!(lines[1].line_index, 1);
            assert_eq!(lines[1].line_number, 2);
            assert_eq!(lines[1].wrapped_lines, 2..4);

            assert_eq!(wrapped_lines[0].line_index, 0);
            assert_eq!(wrapped_lines[0].slice_ptr, &lines[0].graphemes[..5]);
            assert_eq!(wrapped_lines[1].line_index, 0);
            assert_eq!(wrapped_lines[1].slice_ptr, &lines[0].graphemes[5..]);
            assert_eq!(wrapped_lines[2].line_index, 1);
            assert_eq!(wrapped_lines[2].slice_ptr, &lines[1].graphemes[..5]);
            assert_eq!(wrapped_lines[3].line_index, 1);
            assert_eq!(wrapped_lines[3].slice_ptr, &lines[1].graphemes[5..]);
        }

        #[test]
        fn extend() {
            let item = LiteralItem::new("0123456789", None);
            let mut item = TextItem::new(vec![item], Some(5));
            item.extend(vec![
                LiteralItem::new("0123456789", None),
                LiteralItem::new("あいうえ", None),
            ]);

            let lines = item.lines;
            let wrapped_lines = item.wrapped_lines;

            assert_eq!(lines[0].line_index, 0);
            assert_eq!(lines[0].line_number, 0);
            assert_eq!(lines[0].wrapped_lines, 0..2);

            assert_eq!(lines[1].line_index, 1);
            assert_eq!(lines[1].line_number, 2);
            assert_eq!(lines[1].wrapped_lines, 2..4);

            assert_eq!(lines[2].line_index, 2);
            assert_eq!(lines[2].line_number, 4);
            assert_eq!(lines[2].wrapped_lines, 4..6);

            assert_eq!(wrapped_lines[0].line_index, 0);
            assert_eq!(wrapped_lines[0].slice_ptr, &lines[0].graphemes[..5]);
            assert_eq!(wrapped_lines[1].line_index, 0);
            assert_eq!(wrapped_lines[1].slice_ptr, &lines[0].graphemes[5..]);
            assert_eq!(wrapped_lines[2].line_index, 1);
            assert_eq!(wrapped_lines[2].slice_ptr, &lines[1].graphemes[..5]);
            assert_eq!(wrapped_lines[3].line_index, 1);
            assert_eq!(wrapped_lines[3].slice_ptr, &lines[1].graphemes[5..]);
            assert_eq!(wrapped_lines[4].line_index, 2);
            assert_eq!(wrapped_lines[4].slice_ptr, &lines[2].graphemes[..2]);
            assert_eq!(wrapped_lines[5].line_index, 2);
            assert_eq!(wrapped_lines[5].slice_ptr, &lines[2].graphemes[2..]);
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

            let actual: Vec<(usize, Vec<Style>)> = item
                .lines
                .iter()
                .map(|line| {
                    (
                        line.line_index,
                        line.graphemes.iter().map(|i| i.style).collect(),
                    )
                })
                .collect();

            let expected = vec![
                (
                    0,
                    vec![
                        Default::default(),
                        Default::default(),
                        Default::default(),
                        Default::default(),
                        Default::default(),
                        Default::default(),
                        Style::default().add_modifier(Modifier::REVERSED),
                        Style::default().add_modifier(Modifier::REVERSED),
                        Style::default().add_modifier(Modifier::REVERSED),
                        Style::default().add_modifier(Modifier::REVERSED),
                        Style::default().add_modifier(Modifier::REVERSED),
                    ],
                ),
                (
                    1,
                    vec![
                        Default::default(),
                        Default::default(),
                        Default::default(),
                        Default::default(),
                        Default::default(),
                        Style::default().add_modifier(Modifier::REVERSED),
                        Style::default().add_modifier(Modifier::REVERSED),
                        Style::default().add_modifier(Modifier::REVERSED),
                        Style::default().add_modifier(Modifier::REVERSED),
                        Style::default().add_modifier(Modifier::REVERSED),
                    ],
                ),
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

            let actual: Vec<(usize, Vec<Style>)> = item
                .lines
                .iter()
                .map(|line| {
                    (
                        line.line_index,
                        line.graphemes.iter().map(|i| i.style).collect(),
                    )
                })
                .collect();

            let expected = vec![
                (
                    0,
                    vec![
                        Style::default(),
                        Style::default(),
                        Style::default(),
                        Style::default(),
                        Style::default(),
                        Style::default(),
                        Style::default(),
                        Style::default(),
                        Style::default(),
                        Style::default(),
                        Style::default(),
                    ],
                ),
                (
                    1,
                    vec![
                        Style::default(),
                        Style::default(),
                        Style::default(),
                        Style::default(),
                        Style::default(),
                        Style::default(),
                        Style::default(),
                        Style::default(),
                        Style::default(),
                        Style::default(),
                    ],
                ),
            ];

            assert_eq!(actual, expected);
        }

        mod max_chars {
            use super::*;
            use pretty_assertions::assert_eq;
            use rstest::rstest;

            #[rstest]
            #[case(["012345".into(), "0123456789".into()], None, 10)]
            #[case(["012345".into(), "0123456789".into()], Some(5), 5)]
            fn new(
                #[case] literal_item: [LiteralItem; 2],
                #[case] wrap_width: Option<usize>,
                #[case] expected: usize,
            ) {
                let literal_item: Vec<LiteralItem> = literal_item.into_iter().collect();

                let item = TextItem::new(literal_item, wrap_width);

                assert_eq!(item.max_chars(), expected);
            }

            #[rstest]
            #[case(("012345".into(), "0123456789".into()), None, 10)]
            #[case(("012345".into(), "0123456789".into()), Some(5), 5)]
            fn push(
                #[case] literal_item: (LiteralItem, LiteralItem),
                #[case] wrap_width: Option<usize>,
                #[case] expected: usize,
            ) {
                let mut item = TextItem::new(vec![literal_item.0], wrap_width);
                item.push(literal_item.1);

                assert_eq!(item.max_chars(), expected);
            }

            #[rstest]
            #[case("012345".into(), None, 10)]
            #[case("012345".into(), Some(3), 3)]
            fn extend(
                #[case] literal_item: LiteralItem,
                #[case] wrap_width: Option<usize>,
                #[case] expected: usize,
            ) {
                let mut item = TextItem::new(vec![literal_item], wrap_width);
                item.extend(vec![
                    LiteralItem::new("0123456789", None),
                    LiteralItem::new("あいうえ", None),
                ]);

                assert_eq!(item.max_chars(), expected);
            }

            #[rstest]
            fn new_empty() {
                let item = TextItem::new(vec![], None);

                assert_eq!(item.max_chars(), 0);
            }
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

            let graphemes = item.item.styled_graphemes();
            let wrapped_lines: Vec<WrappedLine> = graphemes
                .wrap(None)
                .map(|w| WrappedLine {
                    line_index: 0,
                    slice_ptr: w,
                })
                .collect();

            let mut line = Line {
                line_index: 0,
                line_number: 0,
                literal_item: item,
                graphemes,
                wrapped_lines: 0..1,
            };

            let highlight = line.highlight_word("hello", &wrapped_lines).unwrap();

            assert_eq!(
                highlight,
                vec![Highlight {
                    line_index: 0,
                    range: 0..5,
                    styles: vec![
                        Style::default(),
                        Style::default(),
                        Style::default(),
                        Style::default(),
                        Style::default(),
                    ],
                    line_number: 0,
                }]
            );

            let actual: Vec<Style> = line.graphemes.into_iter().map(|i| i.style).collect();

            assert_eq!(
                actual,
                vec![
                    Style::default().add_modifier(Modifier::REVERSED),
                    Style::default().add_modifier(Modifier::REVERSED),
                    Style::default().add_modifier(Modifier::REVERSED),
                    Style::default().add_modifier(Modifier::REVERSED),
                    Style::default().add_modifier(Modifier::REVERSED),
                    Style::default(),
                    Style::default(),
                    Style::default(),
                    Style::default(),
                    Style::default(),
                    Style::default(),
                ],
            );
        }

        #[test]
        fn 指定された文字列にマッチしないときハイライトしない() {
            let item = LiteralItem {
                item: "hello world".to_string(),
                ..Default::default()
            };

            let graphemes = item.item.styled_graphemes();
            let wrapped_lines: Vec<WrappedLine> = graphemes
                .wrap(None)
                .map(|w| WrappedLine {
                    line_index: 0,
                    slice_ptr: w,
                })
                .collect();

            let mut line = Line {
                line_index: 0,
                line_number: 0,
                literal_item: item,
                graphemes,
                wrapped_lines: 0..1,
            };

            let highlight = line.highlight_word("hoge", &wrapped_lines);

            assert_eq!(highlight.is_none(), true);

            let actual: Vec<Style> = line.graphemes.into_iter().map(|i| i.style).collect();

            assert_eq!(
                actual,
                vec![
                    Style::default(),
                    Style::default(),
                    Style::default(),
                    Style::default(),
                    Style::default(),
                    Style::default(),
                    Style::default(),
                    Style::default(),
                    Style::default(),
                    Style::default(),
                    Style::default(),
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

            let graphemes = item.item.styled_graphemes();
            let wrapped_lines: Vec<WrappedLine> = graphemes
                .wrap(None)
                .map(|w| WrappedLine {
                    line_index: 0,
                    slice_ptr: w,
                })
                .collect();

            let mut line = Line {
                line_index: 0,
                line_number: 0,
                literal_item: item,
                graphemes,
                wrapped_lines: 0..1,
            };

            let highlight = line.highlight_word("hello", &wrapped_lines).unwrap();

            line.clear_highlight(highlight[0].range.clone(), &highlight[0].styles);

            let actual: Vec<Style> = line.graphemes.into_iter().map(|i| i.style).collect();

            assert_eq!(
                actual,
                vec![
                    Style::default().fg(ratatui::style::Color::Red),
                    Style::default().fg(ratatui::style::Color::Red),
                    Style::default().fg(ratatui::style::Color::Red),
                    Style::default().fg(ratatui::style::Color::Red),
                    Style::default().fg(ratatui::style::Color::Red),
                    Style::default(),
                    Style::default(),
                    Style::default(),
                    Style::default(),
                    Style::default(),
                    Style::default(),
                ],
            );
        }
    }
}

mod search {
    use std::ops::Range;

    use crate::ui::widget::styled_graphemes::StyledGrapheme;

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

    impl Search for Vec<StyledGrapheme> {
        fn search(&self, word: &[&str]) -> Option<Vec<Range<usize>>> {
            let mut match_list = Vec::new();

            let word_len = word.len();
            let line_len = self.len();
            for i in 0..line_len {
                let range = i..(i + word_len);

                if let Some(target) = self.get(range.clone()) {
                    if target
                        .iter()
                        .zip(word.iter())
                        .all(|(t, w)| &t.symbol() == w)
                    {
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
        use crate::ui::widget::styled_graphemes::StyledGraphemes;

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
