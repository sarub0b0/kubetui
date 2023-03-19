const COLOR: [u8; 6] = [32, 33, 34, 35, 36, 37];

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
