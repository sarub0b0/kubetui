use super::{
    styled_graphemes::{StyledGrapheme, StyledGraphemes},
    wrap::WrapTrait,
};
use crate::tui_wrapper::widget::LiteralItem;
use std::ops::Range;
use tui::style::{Modifier, Style};

use search::Search;

pub mod inner {
    use std::ops::Range;

    use tui::style::{Modifier, Style};

    use crate::tui_wrapper::widget::{
        text::{
            styled_graphemes::{StyledGrapheme, StyledGraphemes},
            wrap::WrapTrait,
        },
        LiteralItem,
    };

    use super::search::Search;

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
        index: usize,
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
    }

    type Graphemes = Vec<StyledGrapheme>;
    type Wrappers<'a> = Vec<*const [StyledGrapheme]>;

    impl TextItem {
        pub fn new(literal_item: Vec<LiteralItem>, wrap_width: Option<usize>) -> Self {
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

            let mut lines = Vec::new();
            let mut wrapped_lines = Vec::new();
            let mut line_num = 0;
            graphemes_list
                .into_iter()
                .zip(wrappers_list.into_iter())
                .zip(literal_item.into_iter())
                .enumerate()
                .for_each(|(i, ((graphemes, wrapped), literal_item))| {
                    let wrapped_len = wrapped.len();

                    let line = Line {
                        line_index: i,
                        line_number: line_num,
                        literal_item,
                        graphemes,
                        wrapped_lines: line_num..(line_num + wrapped_len),
                    };

                    lines.push(line);

                    let new_wrapped_lines: Vec<WrappedLine> = wrapped
                        .into_iter()
                        .map(|w| WrappedLine {
                            line_index: lines.len() - 1,
                            slice_ptr: w,
                        })
                        .collect();

                    wrapped_lines.extend(new_wrapped_lines);

                    line_num += wrapped_len;
                });

            Self {
                lines,
                wrapped_lines,
                highlights: None,
                wrap_width,
            }
        }

        pub fn update(&mut self, item: Vec<LiteralItem>) {
            let wrap_width = self.wrap_width;
            let highlights = self.highlights.clone();

            let mut new = Self::new(item, wrap_width);

            if let Some(highlights) = highlights {
                new.highlight(&highlights.word);
            }

            *self = new;
        }

        pub fn push(&mut self, item: LiteralItem) {
            let graphemes = item.item.styled_graphemes();

            let wrapped_lines_len = self.wrapped_lines.len();

            #[allow(clippy::needless_collect)]
            let wrappers: Wrappers = graphemes
                .wrap(self.wrap_width)
                .map(|w| w as *const [StyledGrapheme])
                .collect();

            let wrapped_lines: Vec<WrappedLine> = wrappers
                .into_iter()
                .map(|w| WrappedLine {
                    line_index: self.lines.len(),
                    slice_ptr: w,
                })
                .collect();

            let line = Line {
                line_index: self.lines.len(),
                line_number: wrapped_lines_len,
                literal_item: item,
                graphemes,
                wrapped_lines: wrapped_lines_len..(wrapped_lines_len + wrapped_lines.len()),
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
            let wrapped_lines_len = self.wrapped_lines.len();
            let item_len = item.len();

            let graphemes_list: Vec<Graphemes> = item
                .iter()
                .map(|item| item.item.styled_graphemes())
                .collect();

            #[allow(clippy::needless_collect)]
            let wrappers_list: Vec<Wrappers> = graphemes_list
                .iter()
                .map(|g| {
                    g.wrap(self.wrap_width)
                        .map(|w| w as *const [StyledGrapheme])
                        .collect()
                })
                .collect();

            let mut line_num = wrapped_lines_len;
            graphemes_list
                .into_iter()
                .zip(wrappers_list.into_iter())
                .zip(item.into_iter())
                .for_each(|((graphemes, wrapped), literal_item)| {
                    let wrapped_len = wrapped.len();

                    let new_wrapped_lines: Vec<WrappedLine> = wrapped
                        .into_iter()
                        .map(|w| WrappedLine {
                            line_index: line_num,
                            slice_ptr: w,
                        })
                        .collect();

                    let line = Line {
                        line_index: self.lines.len(),
                        line_number: line_num,
                        literal_item,
                        graphemes,
                        wrapped_lines: line_num..(line_num + wrapped_len),
                    };

                    self.lines.push(line);

                    self.wrapped_lines.extend(new_wrapped_lines);

                    line_num += wrapped_len;
                });

            if let Some(highlights) = &mut self.highlights {
                let lines_len = self.lines.len();
                let lines = &mut self.lines[(lines_len - item_len)..lines_len];

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
                    index: 0,
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
                    .zip(hl.styles.iter())
                    .for_each(|(gs, style)| {
                        *gs.style_mut() = style.add_modifier(Modifier::REVERSED)
                    });
            }
        }

        fn highlight_color(&mut self, index: usize) -> Option<usize> {
            if let Some(highlights) = &mut self.highlights {
                let hl = &highlights.item[index];

                let line = &mut self.lines[hl.line_index];
                let graphemes = &mut line.graphemes[hl.range.clone()];

                graphemes
                    .iter_mut()
                    .for_each(|gs| *gs.style_mut() = gs.style().add_modifier(Modifier::SLOW_BLINK));

                highlights.index = index;

                Some(hl.line_number)
            } else {
                None
            }
        }

        pub fn select_nearest_highlight(&mut self, scroll_index: usize) -> Option<usize> {
            let nearest_index = if let Some(highlights) = &mut self.highlights {
                let nearest_index = highlights
                    .item
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, hl)| hl.line_number.abs_diff(scroll_index))
                    .map(|(i, _)| i)
                    .unwrap();

                Some(nearest_index)
            } else {
                None
            };

            if let Some(index) = nearest_index {
                self.highlight_color(index)
            } else {
                None
            }
        }

        pub fn select_next_highlight(&mut self) -> Option<usize> {
            if let Some(highlights) = &mut self.highlights {
                let index = highlights.index;

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
                let index = highlights.index;

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
                (highlights.index + 1, highlights.item.len())
            } else {
                (0, 0)
            }
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

            self.wrapped_lines.clear();
            let mut line_num = 0;
            self.lines
                .iter_mut()
                .zip(wrappers_list.into_iter())
                .enumerate()
                .for_each(|(i, (line, wrapped))| {
                    let wrapped_len = wrapped.len();

                    line.line_number = line_num;
                    line.wrapped_lines = line_num..(line_num + wrapped_len);

                    let new_wrapped_lines: Vec<WrappedLine> = wrapped
                        .into_iter()
                        .map(|w| WrappedLine {
                            line_index: i,
                            slice_ptr: w,
                        })
                        .collect();

                    self.wrapped_lines.extend(new_wrapped_lines);

                    line_num += wrapped_len;
                });

            if let Some(highlights) = &mut self.highlights {
                highlights.item.iter_mut().for_each(|hl| {
                    let line = &self.lines[hl.line_index];

                    let start_index = hl.range.start;

                    let mut line_number = line.line_number;

                    let mut grapheme_len = 0;

                    let wrapped_lines = &self.wrapped_lines[line.wrapped_lines.clone()];

                    wrapped_lines.iter().for_each(|w| {
                        grapheme_len += w.line().len();
                        if grapheme_len < start_index {
                            line_number += 1;
                        }
                    });

                    hl.line_number = line_number;
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
                                *i.style_mut() = i.style().add_modifier(Modifier::REVERSED);
                                ret
                            })
                            .collect();

                        let start_index = range.start;

                        let mut line_number = self.line_number;

                        let mut grapheme_len = 0;

                        wrapped_lines.iter().for_each(|w| {
                            grapheme_len += w.line().len();
                            if grapheme_len < start_index {
                                line_number += 1;
                            }
                        });

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
        use super::*;

        #[test]
        fn new() {
            let item: Vec<LiteralItem> = vec![
                "0123456789".into(),
                "0123456789".into(),
                "0123456789".into(),
            ];

            let text = TextItem::new(item, None);
        }
    }
}

#[derive(Debug, Clone, Default)]
struct Highlights {
    /// 検索ワード
    word: String,

    /// wordにマッチする場所に関するデータ
    item: Vec<Highlight>,

    /// 選択しているインデックス
    /// Reverseではなく背景色を変える
    index: usize,
}

#[derive(Debug, Default)]
pub struct TextItem {
    item: Vec<LiteralItem>,
    /// graphemesに分割した文字列リスト
    graphemes: Vec<Graphemes>,

    /// 折り返しを考慮した描画のためのデータリスト
    /// item設定時に生成される
    wrapped: Vec<WrappedLine>,

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
impl TextItem {
    pub fn new(item: Vec<LiteralItem>, wrap_width: Option<usize>) -> Self {
        let graphemes: Vec<_> = item
            .iter()
            .enumerate()
            .map(|(i, literal)| Graphemes::new(i, literal))
            .collect();

        let wrapped: Vec<WrappedLine> = graphemes.iter().flat_map(|g| g.wrap(wrap_width)).collect();

        Self {
            item,
            graphemes,
            wrap_width,
            wrapped,
            highlights: None,
        }
    }

    pub fn update(&mut self, item: Vec<LiteralItem>) {
        let wrap_width = self.wrap_width;
        let highlights = self.highlights.clone();

        let mut new = Self::new(item, wrap_width);

        if let Some(highlights) = highlights {
            new.highlight(&highlights.word);
        }

        *self = new;
    }

    pub fn push(&mut self, item: LiteralItem) {
        let mut graphemes: Graphemes = Graphemes::new(self.graphemes.len(), &item);

        let wrapped: Vec<WrappedLine> = graphemes.wrap(self.wrap_width);

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
        let mut graphemes: Vec<Graphemes> = item
            .iter()
            .enumerate()
            .map(|(i, literal)| Graphemes::new(i + self.graphemes.len(), literal))
            .collect();

        let wrapped: Vec<WrappedLine> = graphemes
            .iter()
            .flat_map(|g| g.wrap(self.wrap_width))
            .collect();

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

impl TextItem {
    pub fn highlight(&mut self, word: &str) {
        self.clear_highlight();

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

    /// Reverseのハイライト状態に戻す
    fn highlight_normal(&mut self, index: usize) {
        if let Some(highlights) = &mut self.highlights {
            let hl = &highlights.item[index];

            let graphemes = &mut self.graphemes[hl.index].item[hl.range.clone()];

            graphemes
                .iter_mut()
                .zip(hl.item.iter())
                .for_each(|(gs, style)| *gs.style_mut() = style.add_modifier(Modifier::REVERSED));
        }
    }

    /// 指定したインデックスを選択
    fn highlight_color(&mut self, index: usize) -> Option<usize> {
        if let Some(highlights) = &mut self.highlights {
            let hl = &highlights.item[index];

            let graphemes = &mut self.graphemes[hl.index].item[hl.range.clone()];

            graphemes
                .iter_mut()
                .for_each(|gs| *gs.style_mut() = gs.style().add_modifier(Modifier::SLOW_BLINK));

            highlights.index = index;

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
            self.highlight_color(index)
        } else {
            None
        }
    }

    /// 次のマッチ箇所をハイライト
    pub fn select_next_highlight(&mut self) -> Option<usize> {
        if let Some(highlights) = &mut self.highlights {
            let index = highlights.index;

            let item_len = highlights.item.len();

            self.highlight_normal(index);

            let index = (index + 1) % item_len;

            self.highlight_color(index)
        } else {
            None
        }
    }

    /// 前のマッチ箇所をハイライト
    pub fn select_prev_highlight(&mut self) -> Option<usize> {
        if let Some(highlights) = &mut self.highlights {
            let index = highlights.index;

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
            (highlights.index + 1, highlights.item.len())
        } else {
            (0, 0)
        }
    }
}

impl TextItem {
    pub fn wrapped(&self) -> &[WrappedLine] {
        &self.wrapped
    }

    pub fn rewrap(&mut self, wrap_width: usize) {
        self.wrap_width = Some(wrap_width);

        let wrapped: Vec<WrappedLine> = self
            .graphemes
            .iter()
            .flat_map(|g| g.wrap(self.wrap_width))
            .collect();

        self.wrapped = wrapped;
    }
}

/// Itemを折り返しとハイライトを考慮した構造体
#[derive(Debug, PartialEq)]
pub struct WrappedLine {
    /// on_select時に渡すLiteralItemのインデックス = Item.itemのインデックス
    index: usize,

    /// 折り返しを計算した結果、表示する文字列データ
    ptr: *const [StyledGrapheme],
}

impl WrappedLine {
    #[inline]
    pub fn index(&self) -> usize {
        self.index
    }

    #[inline]
    pub fn line(&self) -> &[StyledGrapheme] {
        unsafe { &*self.ptr }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Highlight {
    /// Graphemesのインデックス
    index: usize,

    /// ハイライト箇所の範囲
    range: Range<usize>,

    /// ハイライト前のスタイル
    item: Vec<Style>,
}

/// LiteralItem から Vec<StyledGrapheme> に変換する
///
/// 文字列をパースしてスタイルを適用する
#[derive(Debug, Clone, PartialEq)]
pub struct Graphemes {
    /// 行番号
    index: usize,
    /// １行分の文字列
    item: Vec<StyledGrapheme>,
}

impl Graphemes {
    pub fn new(index: usize, literal: &LiteralItem) -> Self {
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
                            let ret = *i.style();
                            *i.style_mut() = i.style().add_modifier(Modifier::REVERSED);
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
            *l.style_mut() = *r;
        });
    }

    pub fn wrap(&self, wrap_width: Option<usize>) -> Vec<WrappedLine> {
        self.item
            .wrap(wrap_width)
            .map(|w| WrappedLine {
                index: self.index,
                ptr: w,
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

            let graphemes = &item.graphemes[0];

            let expected_lines: Vec<*const _> = vec![&graphemes.item[..5], &graphemes.item[5..]];

            let expected = vec![
                WrappedLine {
                    index: 0,
                    ptr: expected_lines[0],
                },
                WrappedLine {
                    index: 0,
                    ptr: expected_lines[1],
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

            let expected_lines: Vec<*const _> = vec![
                &item.graphemes[0].item[..5],
                &item.graphemes[0].item[5..],
                &item.graphemes[1].item[..5],
                &item.graphemes[1].item[5..],
            ];

            let expected = vec![
                WrappedLine {
                    index: 0,
                    ptr: expected_lines[0],
                },
                WrappedLine {
                    index: 0,
                    ptr: expected_lines[1],
                },
                WrappedLine {
                    index: 1,
                    ptr: expected_lines[2],
                },
                WrappedLine {
                    index: 1,
                    ptr: expected_lines[3],
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

            let expected_lines: Vec<*const _> = vec![
                &item.graphemes[0].item[..5],
                &item.graphemes[0].item[5..],
                &item.graphemes[1].item[..5],
                &item.graphemes[1].item[5..],
                &item.graphemes[2].item[..2],
                &item.graphemes[2].item[2..],
            ];

            let expected = vec![
                WrappedLine {
                    index: 0,
                    ptr: expected_lines[0],
                },
                WrappedLine {
                    index: 0,
                    ptr: expected_lines[1],
                },
                WrappedLine {
                    index: 1,
                    ptr: expected_lines[2],
                },
                WrappedLine {
                    index: 1,
                    ptr: expected_lines[3],
                },
                WrappedLine {
                    index: 2,
                    ptr: expected_lines[4],
                },
                WrappedLine {
                    index: 2,
                    ptr: expected_lines[5],
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

            let actual: Vec<(usize, Vec<Style>)> = item
                .graphemes
                .iter()
                .map(|g| (g.index, g.item.iter().map(|i| i.style).collect()))
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
                .graphemes
                .iter()
                .map(|g| (g.index, g.item.iter().map(|i| i.style).collect()))
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

            let actual: Vec<Style> = item.item.into_iter().map(|i| i.style).collect();
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

            let mut item = Graphemes::new(0, &item);

            let highlight_words = item.highlight_word("hoge");

            assert_eq!(highlight_words.is_none(), true);

            let actual: Vec<Style> = item.item.into_iter().map(|i| i.style).collect();

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
            let mut item = Graphemes::new(0, &item);

            let highlight = item.highlight_word("hello").unwrap();

            item.clear_highlight(&highlight[0]);

            let actual: Vec<Style> = item.item.into_iter().map(|i| i.style).collect();

            assert_eq!(
                actual,
                vec![
                    Style::default().fg(tui::style::Color::Red),
                    Style::default().fg(tui::style::Color::Red),
                    Style::default().fg(tui::style::Color::Red),
                    Style::default().fg(tui::style::Color::Red),
                    Style::default().fg(tui::style::Color::Red),
                    Style::reset(),
                    Style::reset(),
                    Style::reset(),
                    Style::reset(),
                    Style::reset(),
                    Style::reset(),
                ],
            );
        }
    }
}

mod search {
    use std::ops::Range;

    use crate::tui_wrapper::widget::text::styled_graphemes::StyledGrapheme;

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
        use crate::tui_wrapper::widget::text::styled_graphemes::StyledGraphemes;

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
