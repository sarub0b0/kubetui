use nom::{
    branch::{alt, permutation},
    bytes::complete::tag,
    character::complete::{char, digit0, multispace0},
    multi::separated_list0,
    sequence::delimited,
    IResult,
};

use super::AnsiEscapeSequence::{self, *};

#[allow(unused_macros)]
macro_rules! func {
    ($name:ident, ($arg:ident) $token:block) => {
        fn $name($arg: &str) -> IResult<&str, AnsiEscapeSequence, &str> {
            $token
        }
    };
}

#[allow(unused_macros)]
macro_rules! csi_chars {
    ($name:ident, $expr:expr, $enum:ident) => {
        func!($name, (s) {
            let (s, _) = permutation((control_sequence, $expr))(s)?;
            Ok((s, $enum))
        });
    };
}

#[allow(unused_macros)]
macro_rules! csi_num {
    ($name:ident, $char:literal, $init:literal, $enum:ident) => {
        func!($name, (s) {
            let (s, n) = digit(s, $char)?;
            Ok((s, $enum(n.parse().unwrap_or($init))))
        });
    };
}

#[allow(unused_macros)]
macro_rules! csi_row_col{
    ($name:ident, $char:literal, $enum:ident) => {
        func!(
            $name, (s) {
                let (s, (n, m)) = expr_row_col(s, $char)?;
                Ok((s, $enum(n, m)))
            });
    }
}

fn escape(s: &str) -> IResult<&str, char> {
    char('\x1b')(s)
}

fn control_sequence(s: &str) -> IResult<&str, (char, char)> {
    permutation((escape, char('[')))(s)
}

// "", "n", "n;", ";m", "n;m"
// ( "" | NUM ( ";" ( NUM )? )? | ";" NUM ) CHAR
fn row_col(s: &str, close: char) -> IResult<&str, (u16, u16)> {
    let (s, nums): (&str, Vec<&str>) = delimited(
        control_sequence,
        delimited(multispace0, separated_list0(char(';'), digit0), multispace0),
        char(close),
    )(s)?;

    if nums.is_empty() {
        return Ok((s, (1, 1)));
    }

    if nums.len() == 1 {
        return Ok((s, (nums[0].parse::<u16>().unwrap_or(1), 1)));
    }

    Ok((
        s,
        (
            nums[0].parse::<u16>().unwrap_or(1),
            nums[1].parse::<u16>().unwrap_or(1),
        ),
    ))
}

fn graphic(s: &str) -> IResult<&str, AnsiEscapeSequence> {
    let (s, nums): (&str, Vec<&str>) = delimited(
        control_sequence,
        delimited(multispace0, separated_list0(char(';'), digit0), multispace0),
        char('m'),
    )(s)?;

    let nums = nums.iter().map(|s| s.parse().unwrap_or(0)).collect();
    Ok((s, SelectGraphicRendition(nums)))
}

fn digit(s: &str, c: char) -> IResult<&str, &str> {
    delimited(control_sequence, digit0, char(c))(s)
}

fn cursor_up(s: &str) -> IResult<&str, AnsiEscapeSequence> {
    let (s, n) = digit(s, 'A')?;
    Ok((s, CursorUp(n.parse().unwrap_or(1))))
}

fn cursor_down(s: &str) -> IResult<&str, AnsiEscapeSequence> {
    let (s, n) = digit(s, 'B')?;
    Ok((s, CursorDown(n.parse().unwrap_or(1))))
}

fn cursor_forward(s: &str) -> IResult<&str, AnsiEscapeSequence> {
    let (s, n) = digit(s, 'C')?;
    Ok((s, CursorForward(n.parse().unwrap_or(1))))
}

fn cursor_back(s: &str) -> IResult<&str, AnsiEscapeSequence> {
    let (s, n) = digit(s, 'D')?;
    Ok((s, CursorBack(n.parse().unwrap_or(1))))
}

fn cursor_next_line(s: &str) -> IResult<&str, AnsiEscapeSequence> {
    let (s, n) = digit(s, 'E')?;
    Ok((s, CursorNextLine(n.parse().unwrap_or(1))))
}

fn cursor_previous_line(s: &str) -> IResult<&str, AnsiEscapeSequence> {
    let (s, n) = digit(s, 'F')?;
    Ok((s, CursorPreviousLine(n.parse().unwrap_or(1))))
}

fn cursor_horizontal_absolute(s: &str) -> IResult<&str, AnsiEscapeSequence> {
    let (s, n) = digit(s, 'G')?;
    Ok((s, CursorHorizontalAbs(n.parse().unwrap_or(1))))
}

fn cursor_position(s: &str) -> IResult<&str, AnsiEscapeSequence> {
    let (s, (n, m)) = row_col(s, 'H')?;
    Ok((s, CursorPos(n, m)))
}

fn erase_display(s: &str) -> IResult<&str, AnsiEscapeSequence> {
    let (s, n) = digit(s, 'J')?;
    Ok((s, EraseDisplay(n.parse().unwrap_or(0))))
}

fn erase_line(s: &str) -> IResult<&str, AnsiEscapeSequence> {
    let (s, n) = digit(s, 'K')?;
    Ok((s, EraseLine(n.parse().unwrap_or(0))))
}

fn scroll_up(s: &str) -> IResult<&str, AnsiEscapeSequence> {
    let (s, n) = digit(s, 'S')?;
    Ok((s, ScrollUp(n.parse().unwrap_or(1))))
}

fn scroll_down(s: &str) -> IResult<&str, AnsiEscapeSequence> {
    let (s, n) = digit(s, 'T')?;
    Ok((s, ScrollDown(n.parse().unwrap_or(1))))
}

fn horizontal_vertical_pos(s: &str) -> IResult<&str, AnsiEscapeSequence> {
    let (s, (n, m)) = row_col(s, 'f')?;
    Ok((s, HorizontalVerticalPos(n, m)))
}

fn aux_port_on(s: &str) -> IResult<&str, AnsiEscapeSequence> {
    let (s, _) = permutation((control_sequence, tag("5i")))(s)?;
    Ok((s, AuxPortOn))
}

fn aux_port_off(s: &str) -> IResult<&str, AnsiEscapeSequence> {
    let (s, _) = permutation((control_sequence, tag("4i")))(s)?;
    Ok((s, AuxPortOff))
}

fn device_status_report(s: &str) -> IResult<&str, AnsiEscapeSequence> {
    let (s, _) = permutation((control_sequence, tag("6n")))(s)?;
    Ok((s, DeviceStatusReport))
}

fn save_current_cursor_pos(s: &str) -> IResult<&str, AnsiEscapeSequence> {
    let (s, _) = permutation((control_sequence, char('s')))(s)?;
    Ok((s, SaveCurrentCursorPos))
}

fn restore_saved_cursor_pos(s: &str) -> IResult<&str, AnsiEscapeSequence> {
    let (s, _) = permutation((control_sequence, char('u')))(s)?;
    Ok((s, RestoreSavedCursorPos))
}

fn cursor_show(s: &str) -> IResult<&str, AnsiEscapeSequence> {
    let (s, _) = permutation((control_sequence, tag("25h")))(s)?;
    Ok((s, CursorShow))
}

fn cursor_hide(s: &str) -> IResult<&str, AnsiEscapeSequence> {
    let (s, _) = permutation((control_sequence, tag("25l")))(s)?;
    Ok((s, CursorHide))
}

pub fn parse(s: &str) -> IResult<&str, AnsiEscapeSequence> {
    alt((
        // cursor
        cursor_up,
        cursor_down,
        cursor_forward,
        cursor_back,
        cursor_next_line,
        cursor_previous_line,
        cursor_horizontal_absolute,
        cursor_position,
        horizontal_vertical_pos,
        // common
        erase_display,
        erase_line,
        scroll_up,
        scroll_down,
        aux_port_on,
        aux_port_off,
        device_status_report,
        graphic,
        // private
        save_current_cursor_pos,
        restore_saved_cursor_pos,
        cursor_show,
        cursor_hide,
    ))(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    mod csi {
        use super::*;
        mod common {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn empty() {
                assert!(parse("").is_err());
            }

            #[test]
            fn escape() {
                assert!(parse("\x1b").is_err());
            }

            #[test]
            fn erase_display() {
                assert_eq!(parse("\x1b[1J"), Ok(("", EraseDisplay(1))));
            }

            #[test]
            fn erase_line() {
                assert_eq!(parse("\x1b[1K"), Ok(("", EraseLine(1))));
            }

            #[test]
            fn scroll_up() {
                assert_eq!(parse("\x1b[2S"), Ok(("", ScrollUp(2))));
            }

            #[test]
            fn scroll_down() {
                assert_eq!(parse("\x1b[2T"), Ok(("", ScrollDown(2))));
            }

            #[test]
            fn aux_port_on() {
                assert_eq!(parse("\x1b[5i"), Ok(("", AuxPortOn)));
            }

            #[test]
            fn aux_port_off() {
                assert_eq!(parse("\x1b[4i"), Ok(("", AuxPortOff)));
            }

            #[test]
            fn device_status_report() {
                assert_eq!(parse("\x1b[6n"), Ok(("", DeviceStatusReport)));
            }

            mod cursor {
                use super::*;
                use pretty_assertions::assert_eq;

                #[test]
                fn up() {
                    assert_eq!(parse("\x1b[1A"), Ok(("", CursorUp(1))));
                    assert_eq!(parse("\x1b[A"), Ok(("", CursorUp(1))));
                }

                #[test]
                fn down() {
                    assert_eq!(parse("\x1b[2B"), Ok(("", CursorDown(2))));
                }

                #[test]
                fn forward() {
                    assert_eq!(parse("\x1b[2C"), Ok(("", CursorForward(2))));
                }

                #[test]
                fn back() {
                    assert_eq!(parse("\x1b[2D"), Ok(("", CursorBack(2))));
                }

                #[test]
                fn next_line() {
                    assert_eq!(parse("\x1b[2E"), Ok(("", CursorNextLine(2))));
                }

                #[test]
                fn previous_line() {
                    assert_eq!(parse("\x1b[2F"), Ok(("", CursorPreviousLine(2))));
                }

                #[test]
                fn horizontal_absolute() {
                    assert_eq!(parse("\x1b[2G"), Ok(("", CursorHorizontalAbs(2))));
                }

                #[test]
                fn horizontal_vertical_pos() {
                    assert_eq!(parse("\x1b[2;2f"), Ok(("", HorizontalVerticalPos(2, 2))));
                }
                #[test]
                fn position() {
                    assert_eq!(parse("\x1b[H"), Ok(("", CursorPos(1, 1))));
                    assert_eq!(parse("\x1b[2H"), Ok(("", CursorPos(2, 1))));
                    assert_eq!(parse("\x1b[5;H"), Ok(("", CursorPos(5, 1))));
                    assert_eq!(parse("\x1b[2;5H"), Ok(("", CursorPos(2, 5))));
                    assert_eq!(parse("\x1b[;5H"), Ok(("", CursorPos(1, 5))));
                    assert_eq!(parse("\x1b[;H"), Ok(("", CursorPos(1, 1))));
                    assert_eq!(parse("\x1b[;;H"), Ok(("", CursorPos(1, 1))));
                }
            }
            mod graphic {
                use super::*;
                use pretty_assertions::assert_eq;

                #[test]
                fn modifier() {
                    assert_eq!(parse("\x1b[1m"), Ok(("", SelectGraphicRendition(vec![1]))));
                    assert_eq!(
                        parse("\x1b[1;m"),
                        Ok(("", SelectGraphicRendition(vec![1, 0])))
                    );
                }

                #[test]
                fn color_3_bit() {
                    assert_eq!(
                        parse("\x1b[30m"),
                        Ok(("", SelectGraphicRendition(vec![30])))
                    );
                }
                #[test]
                fn color_8_bit() {
                    assert_eq!(
                        parse("\x1b[38;5;200m"),
                        Ok(("", SelectGraphicRendition(vec![38, 5, 200])))
                    );
                }

                #[test]
                fn color_24_bit() {
                    assert_eq!(
                        parse("\x1b[1;38;2;20;20;20m"),
                        Ok(("", SelectGraphicRendition(vec![1, 38, 2, 20, 20, 20])))
                    );
                }
            }
        }
        mod private {
            use super::*;
            use pretty_assertions::assert_eq;

            #[test]
            fn save_current_cursor_pos() {
                assert_eq!(parse("\x1b[s"), Ok(("", SaveCurrentCursorPos)));
            }

            #[test]
            fn restore_saved_cursor_pos() {
                assert_eq!(parse("\x1b[u"), Ok(("", RestoreSavedCursorPos)));
            }

            #[test]
            fn cursor_show() {
                assert_eq!(parse("\x1b[25h"), Ok(("", CursorShow)));
            }

            #[test]
            fn cursor_hide() {
                assert_eq!(parse("\x1b[25l"), Ok(("", CursorHide)));
            }
        }
    }
}
