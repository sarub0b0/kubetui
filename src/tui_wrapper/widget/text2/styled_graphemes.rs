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
fn styled_graphemes(s: &str) -> Vec<StyledGrapheme<'_>> {
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

fn styled_graphemes_symbols(s: &str) -> Vec<&'_ str> {
    s.ansi_parse()
        .filter_map(|p| match p.ty {
            AnsiEscapeSequence::Chars => Some(p.chars),
            _ => None,
        })
        .flat_map(|chars| chars.graphemes(true).collect::<Vec<_>>())
        .collect()
}
