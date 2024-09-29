use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use rayon::prelude::*;

use crate::ansi::{AnsiEscapeSequence, TextParser};

#[allow(dead_code)]
pub fn wrap(lines: &[String], wrap_width: usize) -> Vec<Vec<String>> {
    lines
        .par_iter()
        .map(|line| wrap_line(line, wrap_width))
        .collect()
}

/// 文字列を指定した幅で折り返して、文字列のベクタを返す
pub fn wrap_line(text: &str, wrap_width: usize) -> Vec<String> {
    if text.is_empty() {
        return vec!["".to_string()];
    }

    text.lines()
        .flat_map(|line| {
            if wrap_width < line.width() {
                wrap_line_internal(line, wrap_width)
            } else {
                vec![line.into()]
            }
        })
        .collect()
}

fn wrap_line_internal(line: &str, wrap_width: usize) -> Vec<String> {
    let mut wrapped_lines = Vec::new();

    let mut line_buffer = String::with_capacity(line.len());
    let mut current_width = 0;

    //
    // ANSIエスケープシーケンスを含む文字列をパースして、文字列とタイプを取得する
    //
    for segment in line.ansi_parse() {
        //
        // 表示文字数に関係しないANSIエスケープシーケンスの場合、そのままline_bufferに追加する
        // それ以外の場合は、表示文字数を計算して、折り返し処理を行う
        //
        if segment.ty != AnsiEscapeSequence::Chars {
            line_buffer += segment.chars;
            continue;
        }

        // セグメントの表示文字数を取得
        let parsed_width = segment.chars.width();

        // 折り返す必要がない場合、line_bufferとcurrent_widthにセグメントを追加して、次のセグメントへ
        if (current_width + parsed_width) < wrap_width {
            line_buffer += segment.chars;
            current_width += parsed_width;
            continue;
        }

        //
        // 折り返し幅を超える場合、セグメントを書記素単位に分割して、折り返し処理を行う
        // 文字幅が１と２の場合があるため、１文字ずつ処理する
        //
        for grapheme in segment.chars.graphemes(true) {
            let grapheme_width = grapheme.width();

            // 折り返し幅を超える場合、line_bufferをwrapped_linesに追加して、line_bufferをクリアし、
            // 次の行となるgraphemeを初期化したline_bufferに追加する
            if wrap_width < (current_width + grapheme_width) {
                wrapped_lines.push(line_buffer.to_string());

                line_buffer.clear();

                line_buffer += grapheme;

                current_width = grapheme_width;

                continue;
            }

            // 折り返し幅と同じ場合、line_bufferをwrapped_linesに追加して、line_bufferをクリアする
            if wrap_width == (current_width + grapheme_width) {
                line_buffer += grapheme;

                wrapped_lines.push(line_buffer.to_string());

                line_buffer.clear();

                current_width = 0;

                continue;
            }

            // 折り返さないため、line_bufferにgraphemeを追加する
            line_buffer += grapheme;
            current_width += grapheme_width;
        }
    }

    if !line_buffer.is_empty() {
        if 0 < current_width {
            wrapped_lines.push(line_buffer);
        } else if let Some(last_line) = wrapped_lines.last_mut() {
            *last_line += &line_buffer;
        } else {
            wrapped_lines.push(line_buffer);
        }
    }

    wrapped_lines
}

#[cfg(test)]
mod tests {
    use super::*;

    mod wrap_line {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn contains_escape_sequence() {
            let text = "\x1b[1Aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\x1b[1A\x1b[1A";

            assert_eq!(
                wrap_line_internal(text, 10),
                vec![
                    "\x1b[1Aaaaaaaaaaa".to_string(),
                    "aaaaaaaaaa".to_string(),
                    "aaaaaaaaaa\x1b[1A\x1b[1A".to_string(),
                ]
            );

            let text =
                "\x1b[34mℹ\x1b[39m \x1b[90m｢wds｣\x1b[39m: Project is running at http://10.1.157.9/";

            assert_eq!(
                wrap_line_internal(text, 40),
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
                wrap_line_internal(text, 10),
                vec![
                    "あいうえお".to_string(),
                    "かきくけこ".to_string(),
                    "さしすせそ".to_string(),
                ]
            );

            assert_eq!(
                wrap_line_internal(text, 9),
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
