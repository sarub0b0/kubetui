use nom::{
    branch::{alt, permutation},
    bytes::complete::{tag, take},
    character::complete::{char, digit0, digit1, multispace0},
    combinator::{map, map_opt, map_res, opt},
    multi::{count, many0, many1, separated_list0, separated_list1},
    sequence::delimited,
    IResult,
};

use super::AnsiEscapeSequence::{self, *};

fn escape(s: &str) -> IResult<&str, char> {
    char('\x1b')(s)
}

fn control_sequence(s: &str) -> IResult<&str, (char, char)> {
    permutation((escape, char('[')))(s)
}

// "", "n", "n;", ";m", "n;m"
// ( "" | NUM ( ";" ( NUM )? )? | ";" NUM ) CHAR
fn expr_row_col(s: &str, close: char) -> IResult<&str, (u16, u16)> {
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

fn cursor(s: &str, dir: char) -> IResult<&str, &str> {
    delimited(control_sequence, digit0, char(dir))(s)
}

fn cursor_up(s: &str) -> IResult<&str, AnsiEscapeSequence> {
    let (s, n) = cursor(s, 'A')?;
    Ok((s, CursorUp(n.parse::<u16>().unwrap_or(1))))
}

fn cursor_down(s: &str) -> IResult<&str, AnsiEscapeSequence> {
    let (s, n) = cursor(s, 'B')?;
    Ok((s, CursorDown(n.parse::<u16>().unwrap_or(1))))
}

fn cursor_forward(s: &str) -> IResult<&str, AnsiEscapeSequence> {
    let (s, n) = cursor(s, 'C')?;
    Ok((s, CursorForward(n.parse::<u16>().unwrap_or(1))))
}

fn cursor_back(s: &str) -> IResult<&str, AnsiEscapeSequence> {
    let (s, n) = cursor(s, 'D')?;
    Ok((s, CursorBack(n.parse::<u16>().unwrap_or(1))))
}

fn cursor_next_line(s: &str) -> IResult<&str, AnsiEscapeSequence> {
    let (s, n) = cursor(s, 'E')?;
    Ok((s, CursorNextLine(n.parse::<u16>().unwrap_or(1))))
}

fn cursor_previous_line(s: &str) -> IResult<&str, AnsiEscapeSequence> {
    let (s, n) = cursor(s, 'F')?;
    Ok((s, CursorPreviousLine(n.parse::<u16>().unwrap_or(1))))
}

fn cursor_horizontal_absolute(s: &str) -> IResult<&str, AnsiEscapeSequence> {
    let (s, n) = cursor(s, 'G')?;
    Ok((s, CursorHorizontalAbs(n.parse::<u16>().unwrap_or(1))))
}

fn cursor_position(s: &str) -> IResult<&str, AnsiEscapeSequence> {
    let (s, (n, m)) = expr_row_col(s, 'H')?;

    Ok((s, CursorPos(n, m)))
}

fn cursor_alt(s: &str) -> IResult<&str, AnsiEscapeSequence> {
    alt((
        cursor_up,
        cursor_down,
        cursor_forward,
        cursor_back,
        cursor_next_line,
        cursor_previous_line,
        cursor_horizontal_absolute,
        cursor_position,
    ))(s)
}

pub fn parse(s: &str) -> IResult<&str, AnsiEscapeSequence> {
    cursor_alt(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    mod global {
        use super::*;
        #[test]
        fn empty() {
            assert!(parse("").is_err());
        }

        #[test]
        fn escape() {
            assert!(parse("\x1b").is_err());
        }
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
}
