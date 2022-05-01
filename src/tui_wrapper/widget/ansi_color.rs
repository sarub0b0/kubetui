use tui::style::{Color, Modifier, Style};

pub struct SGR(pub Vec<u8>);

impl From<Vec<u8>> for SGR {
    fn from(code: Vec<u8>) -> Self {
        Self(code)
    }
}

impl SGR {
    pub fn new(code: Vec<u8>) -> Self {
        Self(code)
    }
}

impl From<SGR> for Style {
    fn from(sgr: SGR) -> Self {
        generate_style_from_ansi_color(sgr.0)
    }
}

fn normal_color(n: u8) -> Color {
    match n {
        30 => Color::Black,
        31 => Color::Red,
        32 => Color::Green,
        33 => Color::Yellow,
        34 => Color::Blue,
        35 => Color::Magenta,
        36 => Color::Cyan,
        37 => Color::White,
        _ => unreachable!(),
    }
}

fn bright_color(n: u8) -> Color {
    match n {
        90 => Color::DarkGray,
        91 => Color::LightRed,
        92 => Color::LightGreen,
        93 => Color::LightYellow,
        94 => Color::LightBlue,
        95 => Color::LightMagenta,
        96 => Color::LightCyan,
        97 => Color::Gray,
        _ => unreachable!(),
    }
}

fn modifiers(n: u8) -> Modifier {
    match n {
        1 => Modifier::BOLD,
        2 => Modifier::DIM,
        3 => Modifier::ITALIC,
        4 => Modifier::UNDERLINED,
        5 => Modifier::SLOW_BLINK,
        6 => Modifier::RAPID_BLINK,
        7 => Modifier::REVERSED,
        8 => Modifier::HIDDEN,
        9 => Modifier::CROSSED_OUT,
        _ => unreachable!(),
    }
}

fn color_3_4bit(style: Style, code: u8) -> Style {
    match code {
        //////////////////////////
        // modifiers
        //////////////////////////
        n @ 1..=9 => style.add_modifier(modifiers(n)),

        20 => Style::reset(),
        21 => Style::reset(),
        22 => style.remove_modifier(Modifier::BOLD | Modifier::DIM),
        23 => style.remove_modifier(Modifier::ITALIC),
        24 => style.remove_modifier(Modifier::UNDERLINED),
        25 => style.remove_modifier(Modifier::SLOW_BLINK | Modifier::RAPID_BLINK),
        27 => style.remove_modifier(Modifier::REVERSED),
        28 => Style::reset(),
        29 => style.remove_modifier(Modifier::CROSSED_OUT),
        //////////////////////////
        // foreground
        //////////////////////////
        n @ 30..=37 => style.fg(normal_color(n)),
        n @ 90..=97 => style.fg(bright_color(n)),
        39 => style.fg(Color::Reset),
        //////////////////////////
        // background
        //////////////////////////
        n @ 40..=47 => style.bg(normal_color(n - 10)),
        n @ 100..=107 => style.bg(bright_color(n - 10)),
        49 => style.bg(Color::Reset),

        // error
        _ => Style::reset(),
    }
}

pub fn generate_style_from_ansi_color(codes: Vec<u8>) -> Style {
    let mut style = Style::default();

    let mut iter = codes.iter();
    while let Some(code) = iter.next() {
        //////////////////////////////
        // 8bit, 24bit
        //////////////////////////////
        //
        //=============================
        // 8bit
        //
        // ESC[ 38;5;⟨n⟩ m Select foreground color
        // ESC[ 48;5;⟨n⟩ m Select background color
        //   0-  7:  standard colors (as in ESC [ 30–37 m)
        //   8- 15:  high intensity colors (as in ESC [ 90–97 m)
        //  16-231:  6 × 6 × 6 cube (216 colors): 16 + 36 × r + 6 × g + b (0 ≤ r, g, b ≤ 5)
        // 232-255:  grayscale from black to white in 24 steps
        //
        //==============================
        // 24bit
        // ESC[ 38;2;⟨r⟩;⟨g⟩;⟨b⟩ m Select RGB foreground color
        // ESC[ 48;2;⟨r⟩;⟨g⟩;⟨b⟩ m Select RGB background color
        style = match code {
            // foreground
            38 => match iter.next().unwrap() {
                2 => {
                    let (r, g, b) = (
                        iter.next().unwrap(),
                        iter.next().unwrap(),
                        iter.next().unwrap(),
                    );
                    style.fg(Color::Rgb(*r, *g, *b))
                }
                5 => {
                    let n = iter.next().unwrap();
                    style.fg(Color::Indexed(*n))
                }
                _ => {
                    unreachable!()
                }
            },
            // background
            48 => match iter.next().unwrap() {
                2 => {
                    let (r, g, b) = (
                        iter.next().unwrap(),
                        iter.next().unwrap(),
                        iter.next().unwrap(),
                    );
                    style.bg(Color::Rgb(*r, *g, *b))
                }
                5 => {
                    let n = iter.next().unwrap();
                    style.bg(Color::Indexed(*n))
                }
                _ => {
                    unreachable!()
                }
            },

            //////////////////////////////
            // 3bit, 4bit
            //////////////////////////////
            0 => Style::reset(),
            _ => color_3_4bit(style, *code),
        };
    }
    style
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn color_3_4bit_fg() {
        assert_eq!(
            color_3_4bit(Style::default(), 35),
            Style::default().fg(Color::Magenta)
        );
    }
    #[test]
    fn color_3_4bit_fg_bright() {
        assert_eq!(
            color_3_4bit(Style::default(), 95),
            Style::default().fg(Color::LightMagenta)
        );
    }
    #[test]
    fn color_3_4bit_bg() {
        assert_eq!(
            color_3_4bit(Style::default(), 45),
            Style::default().bg(Color::Magenta)
        );
    }
    #[test]
    fn color_3_4bit_bg_bright() {
        assert_eq!(
            color_3_4bit(Style::default(), 105),
            Style::default().bg(Color::LightMagenta)
        );
    }

    #[test]
    fn color_3_4bit_bold() {
        assert_eq!(
            color_3_4bit(Style::default(), 1),
            Style::default().add_modifier(Modifier::BOLD)
        );
    }

    #[test]
    fn generate_style_color_3_4bit_reset() {
        assert_eq!(generate_style_from_ansi_color(vec![0]), Style::reset());
    }

    #[test]
    fn generate_style_color_8bit_fg() {
        assert_eq!(
            generate_style_from_ansi_color(vec![38, 5, 100]),
            Style::default().fg(Color::Indexed(100))
        );
    }

    #[test]
    fn generate_style_color_8bit_bg() {
        assert_eq!(
            generate_style_from_ansi_color(vec![48, 5, 100]),
            Style::default().bg(Color::Indexed(100))
        );
    }

    #[test]
    fn generate_style_color_8bit_fg_bold() {
        assert_eq!(
            generate_style_from_ansi_color(vec![1, 38, 5, 100]),
            Style::default()
                .fg(Color::Indexed(100))
                .add_modifier(Modifier::BOLD)
        );
        assert_eq!(
            generate_style_from_ansi_color(vec![38, 5, 100, 1]),
            Style::default()
                .fg(Color::Indexed(100))
                .add_modifier(Modifier::BOLD)
        );
    }

    #[test]
    fn generate_style_color_24bit_fg() {
        assert_eq!(
            generate_style_from_ansi_color(vec![38, 2, 10, 10, 10]),
            Style::default().fg(Color::Rgb(10, 10, 10))
        );
    }

    #[test]
    fn generate_style_color_24bit_bg() {
        assert_eq!(
            generate_style_from_ansi_color(vec![48, 2, 10, 10, 10]),
            Style::default().bg(Color::Rgb(10, 10, 10))
        );
    }

    #[test]
    fn generate_style_color_24bit_bold() {
        assert_eq!(
            generate_style_from_ansi_color(vec![1, 38, 2, 10, 10, 10]),
            Style::default()
                .fg(Color::Rgb(10, 10, 10))
                .add_modifier(Modifier::BOLD)
        );
        assert_eq!(
            generate_style_from_ansi_color(vec![38, 2, 10, 10, 10, 1]),
            Style::default()
                .fg(Color::Rgb(10, 10, 10))
                .add_modifier(Modifier::BOLD)
        );
    }

    #[test]
    fn color_3_4bit_panic() {
        assert_eq!(color_3_4bit(Style::default(), 108), Style::reset())
    }
}
