mod parser;
use parser::parse;

#[derive(Debug, PartialEq, Clone)]
pub enum AnsiEscapeSequence {
    Escape,
    CursorUp(u16),
    CursorDown(u16),
    CursorForward(u16),
    CursorBack(u16),
    CursorNextLine(u16),
    CursorPreviousLine(u16),
    CursorHorizontalAbs(u16),
    CursorPos(u16, u16),
    EraseDisplay(u16),
    EraseLine(u16),
    ScrollUp(u16),
    ScrollDown(u16),
    HorizontalVerticalPos(u16, u16),
    SelectGraphicRendition(Vec<u8>),
    AuxPortOn,
    AuxPortOff,
    DeviceStatusReport,
    SaveCurrentCursorPos,
    RestoreSavedCursorPos,
    CursorShow,
    CursorHide,
    SetMode(u8),
    ResetMode(u8),
}

#[derive(Debug, PartialEq)]
pub enum Text<'a> {
    Chars(&'a str),
    Escape(AnsiEscapeSequence),
}

trait TextParser {
    fn ansi_parse(&self) -> TextIterator;
}

pub struct TextIterator<'a>(&'a str);

impl TextParser for str {
    fn ansi_parse(&self) -> TextIterator {
        TextIterator(self)
    }
}

impl TextParser for String {
    fn ansi_parse(&self) -> TextIterator {
        TextIterator(self)
    }
}

impl<'a> Iterator for TextIterator<'a> {
    type Item = Text<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        use AnsiEscapeSequence::*;
        let s = self.0;

        if s.is_empty() {
            return None;
        }

        let find = s.find("\x1b");
        match find {
            Some(pos) => {
                if pos == 0 {
                    match parse(&s[pos..]) {
                        Ok(ret) => {
                            self.0 = ret.0;
                            Some(Text::Escape(ret.1))
                        }
                        Err(_) => Some(Text::Escape(Escape)),
                    }
                } else {
                    let ret = &s[..pos];
                    self.0 = &s[pos..];
                    Some(Text::Chars(ret))
                }
            }
            None => {
                let temp = s;
                self.0 = "";
                Some(Text::Chars(temp))
            }
        }
    }
}

#[cfg(test)]
mod parse_test {
    use super::AnsiEscapeSequence::*;
    use super::Text::*;
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn empty() {
        assert_eq!("".ansi_parse().next(), None);
    }

    #[test]
    fn text_only() {
        assert_eq!("text".ansi_parse().next(), Some(Chars("text")));
    }

    #[test]
    fn escape_only() {
        assert_eq!(
            "\x1b".ansi_parse().next(),
            Some(Text::Escape(AnsiEscapeSequence::Escape))
        );
    }

    #[test]
    fn escape_cursor_up() {
        assert_eq!(
            "\x1b[1A".ansi_parse().next(),
            Some(Text::Escape(CursorUp(1)))
        );
    }

    #[test]
    fn escape_cursor_up_and_cursor_down() {
        let mut iter = "\x1b[1A\x1b[1B".ansi_parse();
        assert_eq!(iter.next(), Some(Text::Escape(CursorUp(1))));
        assert_eq!(iter.next(), Some(Text::Escape(CursorDown(1))));
    }

    #[test]
    fn escape_color() {
        assert_eq!(
            "\x1b[1;2;3;4m".ansi_parse().next(),
            Some(Text::Escape(SelectGraphicRendition(vec![1, 2, 3, 4])))
        );
    }

    #[test]
    fn text_and_cursor_up() {
        let mut iter = "text\x1b[1A".ansi_parse();
        assert_eq!(iter.next(), Some(Chars("text")));
        assert_eq!(iter.next(), Some(Text::Escape(CursorUp(1))));
    }

    #[test]
    fn cursor_up_and_text() {
        let mut iter = "\x1b[1Atext".ansi_parse();
        assert_eq!(iter.next(), Some(Text::Escape(CursorUp(1))));
        assert_eq!(iter.next(), Some(Chars("text")));
    }

    #[test]
    fn text_and_cursor_up_and_text() {
        let mut iter = "text\x1b[1Atext".ansi_parse();
        assert_eq!(iter.next(), Some(Chars("text")));
        assert_eq!(iter.next(), Some(Text::Escape(CursorUp(1))));
        assert_eq!(iter.next(), Some(Chars("text")));
    }

    #[test]
    fn cursor_up_text_and_cursor_down() {
        let mut iter = "\x1b[1Atext\x1b[1B".ansi_parse();
        assert_eq!(iter.next(), Some(Text::Escape(CursorUp(1))));
        assert_eq!(iter.next(), Some(Chars("text")));
        assert_eq!(iter.next(), Some(Text::Escape(CursorDown(1))));
    }
}
