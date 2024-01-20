pub mod fg {

    #[allow(dead_code)]
    #[derive(Clone, Copy)]
    pub enum Color {
        Reset = 39,

        Red = 31,
        Green = 32,
        Yellow = 33,
        Blue = 34,
        Magenta = 35,
        Cyan = 36,
        Gray = 37,

        DarkGray = 90,
        LightRed = 91,
        LightGreen = 92,
        LightYellow = 93,
        LightBlue = 94,
        LightMagenta = 95,
        LightCyan = 96,
        White = 97,
    }

    impl Color {
        pub fn wrap(&self, s: impl Into<String>) -> String {
            format!("\x1b[{}m{}\x1b[39m", *self as u8, s.into())
        }
    }
}

const COLOR: [u8; 6] = [
    fg::Color::Green as u8,
    fg::Color::Yellow as u8,
    fg::Color::Blue as u8,
    fg::Color::Magenta as u8,
    fg::Color::Cyan as u8,
    fg::Color::Gray as u8,
];

pub struct Color {
    index: usize,
}

impl Color {
    pub fn new() -> Self {
        Self { index: 0 }
    }

    pub fn next_color(&mut self) -> u8 {
        if COLOR.len() <= self.index {
            self.index = 0;
        }
        self.index += 1;
        COLOR[self.index - 1]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_default() {
        let mut color = Color::new();
        assert_eq!(color.next_color(), 32)
    }

    #[test]
    fn color_next_1() {
        let mut color = Color::new();
        color.next_color();
        assert_eq!(color.next_color(), 33)
    }

    #[test]
    fn color_next_last() {
        let mut color = Color::new();
        color.next_color();
        color.next_color();
        color.next_color();
        color.next_color();
        color.next_color();
        assert_eq!(color.next_color(), 37)
    }

    #[test]
    fn color_next_loop() {
        let mut color = Color::new();
        color.next_color();
        color.next_color();
        color.next_color();
        color.next_color();
        color.next_color();
        color.next_color();
        assert_eq!(color.next_color(), 32)
    }
}
