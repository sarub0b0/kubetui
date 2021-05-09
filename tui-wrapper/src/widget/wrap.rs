use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use rayon::prelude::*;

use ansi::{self, AnsiEscapeSequence, TextParser};

pub fn wrap(lines: &[String], wrap_width: usize) -> Vec<Vec<String>> {
    lines
        .par_iter()
        .map(|line| wrap_line(line, wrap_width))
        .collect()
}

pub fn wrap_line(text: &str, wrap_width: usize) -> Vec<String> {
    if text.is_empty() {
        return vec!["".to_string()];
    }

    text.lines()
        .map(|line| {
            if wrap_width < line.width() {
                wrap_one_line(line, wrap_width)
            } else {
                vec![line.to_string()]
            }
        })
        .flatten()
        .collect()
}

fn wrap_one_line(line: &str, wrap_width: usize) -> Vec<String> {
    let mut ret = Vec::new();

    let mut buf = String::with_capacity(line.len());
    let mut sum_width = 0;

    for parsed in line.ansi_parse() {
        if parsed.ty == AnsiEscapeSequence::Chars {
            let parsed_width = parsed.chars.width();

            if wrap_width <= sum_width + parsed_width {
                let graphemes: Vec<&str> = parsed.chars.graphemes(true).collect();

                graphemes.iter().for_each(|c| {
                    sum_width += c.width();

                    if wrap_width <= sum_width {
                        if wrap_width == sum_width {
                            buf += c;
                            ret.push(buf.to_string());
                            buf.clear();

                            sum_width = 0;
                        } else {
                            ret.push(buf.to_string());
                            buf.clear();

                            buf += c;
                            sum_width = c.width();
                        }
                    } else {
                        buf += c;
                    }
                });
            } else {
                buf += parsed.chars;
                sum_width += parsed_width;
            }
        } else {
            buf += parsed.chars;
        }
    }

    if !buf.is_empty() {
        if 0 < sum_width {
            ret.push(buf);
        } else if let Some(last) = ret.last_mut() {
            *last += &buf;
        } else {
            ret.push(buf);
        }
    }

    ret
}

#[cfg(test)]
mod tests {
    use super::*;

    mod one_line {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn contains_escape_sequence() {
            let text = "\x1b[1Aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\x1b[1A\x1b[1A";

            assert_eq!(
                wrap_one_line(text, 10),
                vec![
                    "\x1b[1Aaaaaaaaaaa".to_string(),
                    "aaaaaaaaaa".to_string(),
                    "aaaaaaaaaa\x1b[1A\x1b[1A".to_string(),
                ]
            );

            let text =
                "\x1b[34mℹ\x1b[39m \x1b[90m｢wds｣\x1b[39m: Project is running at http://10.1.157.9/";

            assert_eq!(
                wrap_one_line(text, 40),
                vec![
                    "\x1b[34mℹ\x1b[39m \x1b[90m｢wds｣\x1b[39m: Project is running at http://10"
                        .to_string(),
                    ".1.157.9/".to_string(),
                ]
            );
        }

        #[test]
        fn text_only() {
            let text = vec!["ℹ ｢wds｣: Project is running at http://10.1.157.45/".to_string()];
            assert_eq!(
                wrap(&text, 40),
                vec![vec![
                    "ℹ ｢wds｣: Project is running at http://10".to_string(),
                    ".1.157.45/".to_string()
                ],]
            );
        }

        #[test]
        fn wrap_japanese() {
            let text = "あいうえおかきくけこさしすせそ";

            assert_eq!(
                wrap_one_line(text, 10),
                vec![
                    "あいうえお".to_string(),
                    "かきくけこ".to_string(),
                    "さしすせそ".to_string(),
                ]
            );

            assert_eq!(
                wrap_one_line(text, 9),
                vec![
                    "あいうえ".to_string(),
                    "おかきく".to_string(),
                    "けこさし".to_string(),
                    "すせそ".to_string(),
                ]
            );
        }
    }

    mod wrap {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn unwrap() {
            let text = vec!["aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string()];

            assert_eq!(
                wrap(&text, 100),
                vec![vec!["aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string()]]
            );
        }

        #[test]
        fn unwrap_contains_escape_sequence() {
            let text = vec!["\x1b[1Aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\x1b[1A\x1b[1A".to_string()];

            assert_eq!(
                wrap(&text, 30),
                vec![vec![
                    "\x1b[1Aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\x1b[1A\x1b[1A".to_string()
                ]]
            );
        }
        #[test]
        fn contains_newline_string() {
            let text = vec!["hoge".to_string(), "".to_string(), "hoge".to_string()];

            assert_eq!(
                wrap(&text, 100),
                vec![
                    vec!["hoge".to_string()],
                    vec!["".to_string()],
                    vec!["hoge".to_string()]
                ]
            );
        }

        #[test]
        fn short() {
            let text = vec!["aaaaaaaaaaaaaaa".to_string()];

            assert_eq!(
                wrap(&text, 10),
                vec![vec!["aaaaaaaaaa".to_string(), "aaaaa".to_string()]]
            );
            assert_eq!(
                wrap(&text, 5),
                vec![vec![
                    "aaaaa".to_string(),
                    "aaaaa".to_string(),
                    "aaaaa".to_string(),
                ]]
            );
        }

        #[test]
        fn string_contains_newline() {
            let text = vec!["123456789\n123456789\n123456789\n123456789\n123456789\n123456789\n123456789\n123456789\n".to_string()];

            assert_eq!(
                wrap(&text, 12),
                vec![vec![
                    "123456789".to_string(),
                    "123456789".to_string(),
                    "123456789".to_string(),
                    "123456789".to_string(),
                    "123456789".to_string(),
                    "123456789".to_string(),
                    "123456789".to_string(),
                    "123456789".to_string(),
                ]]
            );
        }
    }
}
