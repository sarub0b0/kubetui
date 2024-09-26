use ratatui::{
    style::Style,
    text::{Line, Span},
};

use super::ansi_color::*;

use crate::ansi::{AnsiEscapeSequence, TextParser};

use rayon::prelude::*;

/// ansiエスケープシーケンスを含む文字列のリストのリストをLineのリストのリストに変換する
#[allow(dead_code)]
pub fn convert_nested_lines_to_styled_lines<'a>(multi_lines: &[Vec<String>]) -> Vec<Line<'a>> {
    multi_lines
        .par_iter()
        .flat_map(|lines| convert_lines_to_styled_lines(lines))
        .collect()
}

/// ansiエスケープシーケンスを含む文字列をLineに変換する
pub fn convert_line_to_styled_line<'a>(line: impl AsRef<str>) -> Line<'a> {
    let mut style = Style::default();

    convert_line_to_styled_line_internal(line, &mut style)
}

/// ansiエスケープシーケンスを含む文字列のリストをLineのリストに変換する
pub fn convert_lines_to_styled_lines<'a>(lines: &[String]) -> Vec<Line<'a>> {
    let mut style = Style::default();

    lines
        .iter()
        .map(|line| convert_line_to_styled_line_internal(line, &mut style))
        .collect()
}

fn convert_line_to_styled_line_internal<'a>(line: impl AsRef<str>, style: &mut Style) -> Line<'a> {
    let line = line.as_ref();

    if line.is_empty() {
        return Line::from("");
    }

    let mut span_vec: Vec<Span> = vec![];

    let mut iter = line.ansi_parse().peekable();

    while let Some(parsed) = iter.next() {
        match parsed.ty {
            AnsiEscapeSequence::Chars => {
                span_vec.push(Span::styled(parsed.chars.to_string(), *style));
            }
            AnsiEscapeSequence::SelectGraphicRendition(color) => {
                *style = ansi_to_style(color);

                if iter.peek().is_none() {
                    span_vec.push(Span::styled("", *style));
                }
            }
            _ => {}
        }
    }

    Line::from(span_vec)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use ratatui::style::{Color, Modifier, Style};

    use super::super::wrap::wrap;

    #[test]
    fn spans() {
        let text = vec![
            "> taskbox@0.1.0 start /app",
            "> react-scripts start",
            "",
            "\x1b[34mℹ\x1b[39m \x1b[90m｢wds｣\x1b[39m: Project is running at http://10.1.157.9/",
            "\x1b[34mℹ\x1b[39m \x1b[90m｢wds｣\x1b[39m: webpack output is served from",
            "\x1b[34mℹ\x1b[39m \x1b[90m｢wds｣\x1b[39m: Content not from webpack is served from /app/public",
            "\x1b[34mℹ\x1b[39m \x1b[90m｢wds｣\x1b[39m: 404s will fallback to /",
            "Starting the development server...",
            "",
            "Compiled successfully!",
            "",
            "You can now view taskbox in the browser.",
            "",
            "  Local:            http://localhost:3000",
            "  On Your Network:  http://10.1.157.9:3000",
            "",
            "Note that the development build is not optimized.",
            "To create a production build, use npm run build.",
        ];

        let wrapped = wrap(
            &text
                .iter()
                .cloned()
                .map(String::from)
                .collect::<Vec<String>>(),
            40,
        );

        let expected = vec![
            Line::from("> taskbox@0.1.0 start /app"),
            Line::from("> react-scripts start"),
            Line::from(""),
            Line::from(vec![
                Span::styled("ℹ", Style::default().fg(Color::Blue)),
                Span::styled(" ", Style::default()),
                Span::styled("｢wds｣", Style::default().fg(Color::DarkGray)),
                Span::styled(": Project is running at http://10", Style::default()),
            ]),
            Line::from(Span::styled(".1.157.9/", Style::default())),
            Line::from(vec![
                Span::styled("ℹ", Style::default().fg(Color::Blue)),
                Span::styled(" ", Style::default()),
                Span::styled("｢wds｣", Style::default().fg(Color::DarkGray)),
                Span::styled(": webpack output is served from", Style::default()),
            ]),
            Line::from(vec![
                Span::styled("ℹ", Style::default().fg(Color::Blue)),
                Span::styled(" ", Style::default()),
                Span::styled("｢wds｣", Style::default().fg(Color::DarkGray)),
                Span::styled(": Content not from webpack is ser", Style::default()),
            ]),
            Line::from(Span::styled("ved from /app/public", Style::default())),
            Line::from(vec![
                Span::styled("ℹ", Style::default().fg(Color::Blue)),
                Span::styled(" ", Style::default()),
                Span::styled("｢wds｣", Style::default().fg(Color::DarkGray)),
                Span::styled(": 404s will fallback to /", Style::default()),
            ]),
            Line::from("Starting the development server..."),
            Line::from(""),
            Line::from("Compiled successfully!"),
            Line::from(""),
            Line::from("You can now view taskbox in the browser."),
            Line::from(""),
            Line::from("  Local:            http://localhost:300"),
            Line::from("0"),
            Line::from("  On Your Network:  http://10.1.157.9:30"),
            Line::from("00"),
            Line::from(""),
            Line::from("Note that the development build is not o"),
            Line::from("ptimized."),
            Line::from("To create a production build, use npm ru"),
            Line::from("n build."),
        ];

        let result = convert_nested_lines_to_styled_lines(&wrapped);

        assert_eq!(result, expected);
    }

    mod generate_spans_color {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn color_3_4bit_fg() {
            let text = vec![vec!["hoge\x1b[33mhoge\x1b[39m".to_string()]];

            assert_eq!(
                convert_nested_lines_to_styled_lines(&text),
                vec![Line::from(vec![
                    Span::raw("hoge"),
                    Span::styled("hoge", Style::default().fg(Color::Yellow)),
                    Span::styled("", Style::default()),
                ])]
            )
        }

        #[test]
        fn color_3_4bit_fg_bold() {
            let text = vec![vec!["\x1b[1;33mhoge\x1b[39m".to_string()]];

            assert_eq!(
                convert_nested_lines_to_styled_lines(&text),
                vec![Line::from(vec![
                    Span::styled(
                        "hoge",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    ),
                    Span::styled("", Style::default()),
                ])]
            )
        }

        #[test]
        fn color_8bit_fg() {
            let text = vec![vec!["\x1b[38;5;33mhoge\x1b[39m".to_string()]];

            assert_eq!(
                convert_nested_lines_to_styled_lines(&text),
                vec![Line::from(vec![
                    Span::styled("hoge", Style::default().fg(Color::Indexed(33))),
                    Span::styled("", Style::default()),
                ])]
            )
        }

        #[test]
        fn color_8bit_fg_bold() {
            let text = vec![vec!["\x1b[1;38;5;33mhoge\x1b[39m".to_string()]];

            assert_eq!(
                convert_nested_lines_to_styled_lines(&text),
                vec![Line::from(vec![
                    Span::styled(
                        "hoge",
                        Style::default()
                            .fg(Color::Indexed(33))
                            .add_modifier(Modifier::BOLD)
                    ),
                    Span::styled("", Style::default()),
                ])]
            )
        }

        #[test]
        fn color_24bit_fg() {
            let text = vec![vec!["\x1b[38;2;33;10;10mhoge\x1b[39m".to_string()]];

            assert_eq!(
                convert_nested_lines_to_styled_lines(&text),
                vec![Line::from(vec![
                    Span::styled("hoge", Style::default().fg(Color::Rgb(33, 10, 10))),
                    Span::styled("", Style::default()),
                ])]
            )
        }

        #[test]
        fn color_24bit_fg_bold() {
            let text = vec![vec!["\x1b[1;38;2;33;10;10mhoge\x1b[39m".to_string()]];

            assert_eq!(
                convert_nested_lines_to_styled_lines(&text),
                vec![Line::from(vec![
                    Span::styled(
                        "hoge",
                        Style::default()
                            .fg(Color::Rgb(33, 10, 10))
                            .add_modifier(Modifier::BOLD)
                    ),
                    Span::styled("", Style::default()),
                ])]
            )
        }

        #[test]
        fn color_3_4bit_bg() {
            let text = vec![vec!["\x1b[43mhoge\x1b[49m".to_string()]];

            assert_eq!(
                convert_nested_lines_to_styled_lines(&text),
                vec![Line::from(vec![
                    Span::styled("hoge", Style::default().bg(Color::Yellow)),
                    Span::styled("", Style::default()),
                ])]
            )
        }

        #[test]
        fn color_3_4bit_bg_bold() {
            let text = vec![vec!["\x1b[1;43mhoge\x1b[49m".to_string()]];

            assert_eq!(
                convert_nested_lines_to_styled_lines(&text),
                vec![Line::from(vec![
                    Span::styled(
                        "hoge",
                        Style::default()
                            .bg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    ),
                    Span::styled("", Style::default()),
                ])]
            );

            let text = vec![vec!["\x1b[43;1mhoge\x1b[49m".to_string()]];

            assert_eq!(
                convert_nested_lines_to_styled_lines(&text),
                vec![Line::from(vec![
                    Span::styled(
                        "hoge",
                        Style::default()
                            .bg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    ),
                    Span::styled("", Style::default()),
                ])]
            );
        }

        #[test]
        fn color_8bit_bg() {
            let text = vec![vec!["\x1b[48;5;33mhoge\x1b[49m".to_string()]];

            assert_eq!(
                convert_nested_lines_to_styled_lines(&text),
                vec![Line::from(vec![
                    Span::styled("hoge", Style::default().bg(Color::Indexed(33))),
                    Span::styled("", Style::default()),
                ])]
            );
        }

        #[test]
        fn color_8bit_bg_bold() {
            let text = vec![vec!["\x1b[1;48;5;33mhoge\x1b[49m".to_string()]];

            assert_eq!(
                convert_nested_lines_to_styled_lines(&text),
                vec![Line::from(vec![
                    Span::styled(
                        "hoge",
                        Style::default()
                            .bg(Color::Indexed(33))
                            .add_modifier(Modifier::BOLD)
                    ),
                    Span::styled("", Style::default()),
                ])]
            );

            let text = vec![vec!["\x1b[48;5;33;1mhoge\x1b[49m".to_string()]];

            assert_eq!(
                convert_nested_lines_to_styled_lines(&text),
                vec![Line::from(vec![
                    Span::styled(
                        "hoge",
                        Style::default()
                            .bg(Color::Indexed(33))
                            .add_modifier(Modifier::BOLD)
                    ),
                    Span::styled("", Style::default()),
                ])]
            );
        }

        #[test]
        fn color_24bit_bg() {
            let text = vec![vec!["\x1b[48;2;33;10;10mhoge\x1b[49m".to_string()]];

            assert_eq!(
                convert_nested_lines_to_styled_lines(&text),
                vec![Line::from(vec![
                    Span::styled("hoge", Style::default().bg(Color::Rgb(33, 10, 10))),
                    Span::styled("", Style::default()),
                ])]
            );
        }

        #[test]
        fn color_24bit_bg_bold() {
            let text = vec![vec!["\x1b[1;48;2;33;10;10mhoge\x1b[49m".to_string()]];

            assert_eq!(
                convert_nested_lines_to_styled_lines(&text),
                vec![Line::from(vec![
                    Span::styled(
                        "hoge",
                        Style::default()
                            .bg(Color::Rgb(33, 10, 10))
                            .add_modifier(Modifier::BOLD)
                    ),
                    Span::styled("", Style::default()),
                ])]
            );

            let text = vec![vec!["\x1b[48;2;33;10;10;1mhoge\x1b[49m".to_string()]];

            assert_eq!(
                convert_nested_lines_to_styled_lines(&text),
                vec![Line::from(vec![
                    Span::styled(
                        "hoge",
                        Style::default()
                            .bg(Color::Rgb(33, 10, 10))
                            .add_modifier(Modifier::BOLD)
                    ),
                    Span::styled("", Style::default()),
                ])]
            );
        }

        #[test]
        fn color_24bit_rainbow() {
            let rainbow = vec!["[48;2;0;0;0m [48;2;1;0;0m [48;2;2;0;0m [48;2;3;0;0m [48;2;4;0;0m [48;2;5;0;0m [48;2;6;0;0m [48;2;7;0;0m [48;2;8;0;0m [48;2;9;0;0m [48;2;10;0;0m [0m".to_string()];

            let wrapped = wrap(&rainbow, 3);

            assert_eq!(
                wrapped,
                vec![vec![
                    "[48;2;0;0;0m [48;2;1;0;0m [48;2;2;0;0m ".to_string(),
                    "[48;2;3;0;0m [48;2;4;0;0m [48;2;5;0;0m ".to_string(),
                    "[48;2;6;0;0m [48;2;7;0;0m [48;2;8;0;0m ".to_string(),
                    "[48;2;9;0;0m [48;2;10;0;0m [0m".to_string(),
                ]]
            );

            assert_eq!(
                convert_nested_lines_to_styled_lines(&wrapped),
                vec![
                    Line::from(vec![
                        Span::styled(" ", Style::default().bg(Color::Rgb(0, 0, 0))),
                        Span::styled(" ", Style::default().bg(Color::Rgb(1, 0, 0))),
                        Span::styled(" ", Style::default().bg(Color::Rgb(2, 0, 0)))
                    ]),
                    Line::from(vec![
                        Span::styled(" ", Style::default().bg(Color::Rgb(3, 0, 0))),
                        Span::styled(" ", Style::default().bg(Color::Rgb(4, 0, 0))),
                        Span::styled(" ", Style::default().bg(Color::Rgb(5, 0, 0))),
                    ]),
                    Line::from(vec![
                        Span::styled(" ", Style::default().bg(Color::Rgb(6, 0, 0))),
                        Span::styled(" ", Style::default().bg(Color::Rgb(7, 0, 0))),
                        Span::styled(" ", Style::default().bg(Color::Rgb(8, 0, 0))),
                    ]),
                    Line::from(vec![
                        Span::styled(" ", Style::default().bg(Color::Rgb(9, 0, 0))),
                        Span::styled(" ", Style::default().bg(Color::Rgb(10, 0, 0))),
                        Span::styled("", Style::default())
                    ]),
                ]
            );
        }

        #[test]
        fn color_oneline() {
            let text = vec!["\x1b[31maaaaaaaaaaaaaaa\x1b[0m".to_string()];

            let wrapped = wrap(&text, 10);

            assert_eq!(
                convert_nested_lines_to_styled_lines(&wrapped),
                vec![
                    Line::from(vec![Span::styled(
                        "aaaaaaaaaa",
                        Style::default().fg(Color::Red)
                    )]),
                    Line::from(vec![
                        Span::styled("aaaaa", Style::default().fg(Color::Red)),
                        Span::styled("", Style::default())
                    ]),
                ]
            )
        }
    }
}
