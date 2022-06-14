use tui::style::Style;
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    ansi::{AnsiEscapeSequence, TextParser},
    tui_wrapper::widget::ansi_color::Sgr,
};

#[derive(Debug, Clone, PartialEq)]
pub struct StyledGrapheme {
    pub(super) symbol: *const str,
    pub(super) style: Style,
}

impl StyledGrapheme {
    #[inline]
    pub fn symbol(&self) -> &str {
        unsafe { &*self.symbol }
    }

    #[inline]
    pub fn style(&self) -> &Style {
        &self.style
    }

    #[inline]
    pub fn style_mut(&mut self) -> &mut Style {
        &mut self.style
    }
}

pub trait StyledGraphemes {
    fn styled_graphemes(&self) -> Vec<StyledGrapheme>;
    fn styled_graphemes_symbols(&self) -> Vec<&str>;
}

impl StyledGraphemes for String {
    fn styled_graphemes(&self) -> Vec<StyledGrapheme> {
        styled_graphemes(self)
    }

    fn styled_graphemes_symbols(&self) -> Vec<&str> {
        styled_graphemes_symbols(self)
    }
}

impl StyledGraphemes for &String {
    fn styled_graphemes(&self) -> Vec<StyledGrapheme> {
        styled_graphemes(self)
    }

    fn styled_graphemes_symbols(&self) -> Vec<&str> {
        styled_graphemes_symbols(self)
    }
}

impl StyledGraphemes for &str {
    fn styled_graphemes(&self) -> Vec<StyledGrapheme> {
        styled_graphemes(self)
    }

    fn styled_graphemes_symbols(&self) -> Vec<&str> {
        styled_graphemes_symbols(self)
    }
}

/// 一文字単位でスタイルを適用したリストを返す
fn styled_graphemes(s: &str) -> Vec<StyledGrapheme> {
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
            sg.symbol()
                .graphemes(true)
                .map(|g| StyledGrapheme {
                    symbol: g,
                    style: sg.style,
                })
                .collect::<Vec<StyledGrapheme>>()
        })
        .collect()
}

fn styled_graphemes_symbols(s: &str) -> Vec<&'_ str> {
    s.ansi_parse()
        .filter_map(|p| match p.ty {
            AnsiEscapeSequence::Chars => Some(p.chars),
            _ => None,
        })
        .flat_map(|chars| chars.graphemes(true).collect::<Vec<_>>())
        .collect()
}
