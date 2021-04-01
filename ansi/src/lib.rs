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
    EraseLIne(u16),
    ScrollUp(u16),
    ScrollDown(u16),
    HorizontalVerticalPos(u16, u16),
    SelectGraphicRendition(Vec<u8>),
    AuxPortOn,
    AuxPortOff,
    DeviceStatusReport,
    SaveCurrentCursorPos,
    RestoreCurrentCursorPos,
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
    use super::*;
    use pretty_assertions::assert_eq;
}
