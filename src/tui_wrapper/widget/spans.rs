#![allow(dead_code)]

use tui::{
    style::Style,
    text::{Span, Spans},
};

use super::ansi_color::*;

use crate::ansi::{AnsiEscapeSequence, TextParser};

use rayon::prelude::*;

pub fn generate_spans<'a>(multi_lines: &[Vec<String>]) -> Vec<Spans<'a>> {
    multi_lines
        .par_iter()
        .flat_map(|lines| generate_spans_line(lines))
        .collect()
}

pub fn generate_spans_line<'a>(lines: &[String]) -> Vec<Spans<'a>> {
    let mut style = Style::default();

    lines
        .iter()
        .map(|line| {
            if line.is_empty() {
                return Spans::from(Span::styled("", Style::default()));
            }
            let mut span_vec: Vec<Span> = vec![];

            let mut iter = line.ansi_parse().peekable();

            while let Some(parsed) = iter.next() {
                match parsed.ty {
                    AnsiEscapeSequence::Chars => {
                        span_vec.push(Span::styled(parsed.chars.to_string(), style));
                    }
                    AnsiEscapeSequence::SelectGraphicRendition(color) => {
                        style = generate_style_from_ansi_color(color);

                        if iter.peek().is_none() {
                            span_vec.push(Span::styled("", style));
                        }
                    }
                    _ => {}
                }
            }

            Spans::from(span_vec)
        })
        .collect::<Vec<Spans>>()
}
#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use tui::style::{Color, Modifier, Style};

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
            Spans::from("> taskbox@0.1.0 start /app"),
            Spans::from("> react-scripts start"),
            Spans::from(""),
            Spans::from(vec![
                Span::styled("ℹ", Style::default().fg(Color::Blue)),
                Span::styled(" ", Style::default().fg(Color::Reset)),
                Span::styled("｢wds｣", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    ": Project is running at http://10",
                    Style::default().fg(Color::Reset),
                ),
            ]),
            Spans::from(Span::styled(".1.157.9/", Style::default().fg(Color::Reset))),
            Spans::from(vec![
                Span::styled("ℹ", Style::default().fg(Color::Blue)),
                Span::styled(" ", Style::default().fg(Color::Reset)),
                Span::styled("｢wds｣", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    ": webpack output is served from",
                    Style::default().fg(Color::Reset),
                ),
            ]),
            Spans::from(vec![
                Span::styled("ℹ", Style::default().fg(Color::Blue)),
                Span::styled(" ", Style::default().fg(Color::Reset)),
                Span::styled("｢wds｣", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    ": Content not from webpack is ser",
                    Style::default().fg(Color::Reset),
                ),
            ]),
            Spans::from(Span::styled(
                "ved from /app/public",
                Style::default().fg(Color::Reset),
            )),
            Spans::from(vec![
                Span::styled("ℹ", Style::default().fg(Color::Blue)),
                Span::styled(" ", Style::default().fg(Color::Reset)),
                Span::styled("｢wds｣", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    ": 404s will fallback to /",
                    Style::default().fg(Color::Reset),
                ),
            ]),
            Spans::from("Starting the development server..."),
            Spans::from(""),
            Spans::from("Compiled successfully!"),
            Spans::from(""),
            Spans::from("You can now view taskbox in the browser."),
            Spans::from(""),
            Spans::from("  Local:            http://localhost:300"),
            Spans::from("0"),
            Spans::from("  On Your Network:  http://10.1.157.9:30"),
            Spans::from("00"),
            Spans::from(""),
            Spans::from("Note that the development build is not o"),
            Spans::from("ptimized."),
            Spans::from("To create a production build, use npm ru"),
            Spans::from("n build."),
        ];

        let result = generate_spans(&wrapped);
        for (i, l) in result.iter().enumerate() {
            assert_eq!(*l, expected[i]);
        }
    }

    mod generate_spans_color {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn color_3_4bit_fg() {
            let text = vec![vec!["hoge\x1b[33mhoge\x1b[39m".to_string()]];

            assert_eq!(
                generate_spans(&text),
                vec![Spans::from(vec![
                    Span::raw("hoge"),
                    Span::styled("hoge", Style::default().fg(Color::Yellow)),
                    Span::styled("", Style::default().fg(Color::Reset)),
                ])]
            )
        }

        #[test]
        fn color_3_4bit_fg_bold() {
            let text = vec![vec!["\x1b[1;33mhoge\x1b[39m".to_string()]];

            assert_eq!(
                generate_spans(&text),
                vec![Spans::from(vec![
                    Span::styled(
                        "hoge",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    ),
                    Span::styled("", Style::default().fg(Color::Reset)),
                ])]
            )
        }

        #[test]
        fn color_8bit_fg() {
            let text = vec![vec!["\x1b[38;5;33mhoge\x1b[39m".to_string()]];

            assert_eq!(
                generate_spans(&text),
                vec![Spans::from(vec![
                    Span::styled("hoge", Style::default().fg(Color::Indexed(33))),
                    Span::styled("", Style::default().fg(Color::Reset)),
                ])]
            )
        }

        #[test]
        fn color_8bit_fg_bold() {
            let text = vec![vec!["\x1b[1;38;5;33mhoge\x1b[39m".to_string()]];

            assert_eq!(
                generate_spans(&text),
                vec![Spans::from(vec![
                    Span::styled(
                        "hoge",
                        Style::default()
                            .fg(Color::Indexed(33))
                            .add_modifier(Modifier::BOLD)
                    ),
                    Span::styled("", Style::default().fg(Color::Reset)),
                ])]
            )
        }

        #[test]
        fn color_24bit_fg() {
            let text = vec![vec!["\x1b[38;2;33;10;10mhoge\x1b[39m".to_string()]];

            assert_eq!(
                generate_spans(&text),
                vec![Spans::from(vec![
                    Span::styled("hoge", Style::default().fg(Color::Rgb(33, 10, 10))),
                    Span::styled("", Style::default().fg(Color::Reset)),
                ])]
            )
        }

        #[test]
        fn color_24bit_fg_bold() {
            let text = vec![vec!["\x1b[1;38;2;33;10;10mhoge\x1b[39m".to_string()]];

            assert_eq!(
                generate_spans(&text),
                vec![Spans::from(vec![
                    Span::styled(
                        "hoge",
                        Style::default()
                            .fg(Color::Rgb(33, 10, 10))
                            .add_modifier(Modifier::BOLD)
                    ),
                    Span::styled("", Style::default().fg(Color::Reset)),
                ])]
            )
        }

        #[test]
        fn color_3_4bit_bg() {
            let text = vec![vec!["\x1b[43mhoge\x1b[49m".to_string()]];

            assert_eq!(
                generate_spans(&text),
                vec![Spans::from(vec![
                    Span::styled("hoge", Style::default().bg(Color::Yellow)),
                    Span::styled("", Style::default().bg(Color::Reset)),
                ])]
            )
        }

        #[test]
        fn color_3_4bit_bg_bold() {
            let text = vec![vec!["\x1b[1;43mhoge\x1b[49m".to_string()]];

            assert_eq!(
                generate_spans(&text),
                vec![Spans::from(vec![
                    Span::styled(
                        "hoge",
                        Style::default()
                            .bg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    ),
                    Span::styled("", Style::default().bg(Color::Reset)),
                ])]
            );

            let text = vec![vec!["\x1b[43;1mhoge\x1b[49m".to_string()]];

            assert_eq!(
                generate_spans(&text),
                vec![Spans::from(vec![
                    Span::styled(
                        "hoge",
                        Style::default()
                            .bg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    ),
                    Span::styled("", Style::default().bg(Color::Reset)),
                ])]
            );
        }

        #[test]
        fn color_8bit_bg() {
            let text = vec![vec!["\x1b[48;5;33mhoge\x1b[49m".to_string()]];

            assert_eq!(
                generate_spans(&text),
                vec![Spans::from(vec![
                    Span::styled("hoge", Style::default().bg(Color::Indexed(33))),
                    Span::styled("", Style::default().bg(Color::Reset)),
                ])]
            );
        }

        #[test]
        fn color_8bit_bg_bold() {
            let text = vec![vec!["\x1b[1;48;5;33mhoge\x1b[49m".to_string()]];

            assert_eq!(
                generate_spans(&text),
                vec![Spans::from(vec![
                    Span::styled(
                        "hoge",
                        Style::default()
                            .bg(Color::Indexed(33))
                            .add_modifier(Modifier::BOLD)
                    ),
                    Span::styled("", Style::default().bg(Color::Reset)),
                ])]
            );

            let text = vec![vec!["\x1b[48;5;33;1mhoge\x1b[49m".to_string()]];

            assert_eq!(
                generate_spans(&text),
                vec![Spans::from(vec![
                    Span::styled(
                        "hoge",
                        Style::default()
                            .bg(Color::Indexed(33))
                            .add_modifier(Modifier::BOLD)
                    ),
                    Span::styled("", Style::default().bg(Color::Reset)),
                ])]
            );
        }

        #[test]
        fn color_24bit_bg() {
            let text = vec![vec!["\x1b[48;2;33;10;10mhoge\x1b[49m".to_string()]];

            assert_eq!(
                generate_spans(&text),
                vec![Spans::from(vec![
                    Span::styled("hoge", Style::default().bg(Color::Rgb(33, 10, 10))),
                    Span::styled("", Style::default().bg(Color::Reset)),
                ])]
            );
        }

        #[test]
        fn color_24bit_bg_bold() {
            let text = vec![vec!["\x1b[1;48;2;33;10;10mhoge\x1b[49m".to_string()]];

            assert_eq!(
                generate_spans(&text),
                vec![Spans::from(vec![
                    Span::styled(
                        "hoge",
                        Style::default()
                            .bg(Color::Rgb(33, 10, 10))
                            .add_modifier(Modifier::BOLD)
                    ),
                    Span::styled("", Style::default().bg(Color::Reset)),
                ])]
            );

            let text = vec![vec!["\x1b[48;2;33;10;10;1mhoge\x1b[49m".to_string()]];

            assert_eq!(
                generate_spans(&text),
                vec![Spans::from(vec![
                    Span::styled(
                        "hoge",
                        Style::default()
                            .bg(Color::Rgb(33, 10, 10))
                            .add_modifier(Modifier::BOLD)
                    ),
                    Span::styled("", Style::default().bg(Color::Reset)),
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
                generate_spans(&wrapped),
                vec![
                    Spans::from(vec![
                        Span::styled(" ", Style::default().bg(Color::Rgb(0, 0, 0))),
                        Span::styled(" ", Style::default().bg(Color::Rgb(1, 0, 0))),
                        Span::styled(" ", Style::default().bg(Color::Rgb(2, 0, 0)))
                    ]),
                    Spans::from(vec![
                        Span::styled(" ", Style::default().bg(Color::Rgb(3, 0, 0))),
                        Span::styled(" ", Style::default().bg(Color::Rgb(4, 0, 0))),
                        Span::styled(" ", Style::default().bg(Color::Rgb(5, 0, 0))),
                    ]),
                    Spans::from(vec![
                        Span::styled(" ", Style::default().bg(Color::Rgb(6, 0, 0))),
                        Span::styled(" ", Style::default().bg(Color::Rgb(7, 0, 0))),
                        Span::styled(" ", Style::default().bg(Color::Rgb(8, 0, 0))),
                    ]),
                    Spans::from(vec![
                        Span::styled(" ", Style::default().bg(Color::Rgb(9, 0, 0))),
                        Span::styled(" ", Style::default().bg(Color::Rgb(10, 0, 0))),
                        Span::styled("", Style::reset())
                    ]),
                ]
            );
        }

        #[test]
        fn color_oneline() {
            let text = vec!["\x1b[31maaaaaaaaaaaaaaa\x1b[0m".to_string()];

            let wrapped = wrap(&text, 10);

            assert_eq!(
                generate_spans(&wrapped),
                vec![
                    Spans::from(vec![Span::styled(
                        "aaaaaaaaaa",
                        Style::default().fg(Color::Red)
                    )]),
                    Spans::from(vec![
                        Span::styled("aaaaa", Style::default().fg(Color::Red)),
                        Span::styled("", Style::reset())
                    ]),
                ]
            )
        }
    }
}
