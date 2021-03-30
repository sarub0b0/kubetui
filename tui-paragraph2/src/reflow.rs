// version 0.14をそのまま使用
// https://github.com/fdehau/tui-rs/blob/master/src/widgets/reflow.rs
//
use tui::text::StyledGrapheme;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// A state machine to pack styled symbols into lines.
/// Cannot implement it as Iterator since it yields slices of the internal buffer (need streaming
/// iterators for that).
pub trait LineComposer<'a> {
    fn next_line(&mut self) -> Option<(&[StyledGrapheme<'a>], u16)>;
}

/// A state machine that truncates overhanging lines.
pub struct LineTruncator<'a, 'b> {
    symbols: &'b mut dyn Iterator<Item = StyledGrapheme<'a>>,
    max_line_width: u16,
    current_line: Vec<StyledGrapheme<'a>>,
    /// Record the offet to skip render
    horizontal_offset: u16,
}

impl<'a, 'b> LineTruncator<'a, 'b> {
    pub fn new(
        symbols: &'b mut dyn Iterator<Item = StyledGrapheme<'a>>,
        max_line_width: u16,
    ) -> LineTruncator<'a, 'b> {
        LineTruncator {
            symbols,
            max_line_width,
            horizontal_offset: 0,
            current_line: vec![],
        }
    }
}

impl<'a, 'b> LineComposer<'a> for LineTruncator<'a, 'b> {
    fn next_line(&mut self) -> Option<(&[StyledGrapheme<'a>], u16)> {
        if self.max_line_width == 0 {
            return None;
        }

        self.current_line.truncate(0);
        let mut current_line_width = 0;

        let mut skip_rest = false;
        let mut symbols_exhausted = true;
        let mut horizontal_offset = self.horizontal_offset as usize;
        for StyledGrapheme { symbol, style } in &mut self.symbols {
            symbols_exhausted = false;

            // Ignore characters wider that the total max width.
            if symbol.width() as u16 > self.max_line_width {
                continue;
            }

            // Break on newline and discard it.
            if symbol == "\n" {
                break;
            }

            if current_line_width + symbol.width() as u16 > self.max_line_width {
                // Exhaust the remainder of the line.
                skip_rest = true;
                break;
            }

            let symbol = if horizontal_offset == 0 {
                symbol
            } else {
                let w = symbol.width();
                if w > horizontal_offset {
                    let t = trim_offset(symbol, horizontal_offset);
                    horizontal_offset = 0;
                    t
                } else {
                    horizontal_offset -= w;
                    ""
                }
            };
            current_line_width += symbol.width() as u16;
            self.current_line.push(StyledGrapheme { symbol, style });
        }

        if skip_rest {
            for StyledGrapheme { symbol, .. } in &mut self.symbols {
                if symbol == "\n" {
                    break;
                }
            }
        }

        if symbols_exhausted && self.current_line.is_empty() {
            None
        } else {
            Some((&self.current_line[..], current_line_width))
        }
    }
}

/// This function will return a str slice which start at specified offset.
/// As src is a unicode str, start offset has to be calculated with each character.
fn trim_offset(src: &str, mut offset: usize) -> &str {
    let mut start = 0;
    for c in UnicodeSegmentation::graphemes(src, true) {
        let w = c.width();
        if w <= offset {
            offset -= w;
            start += c.len();
        } else {
            break;
        }
    }
    &src[start..]
}
