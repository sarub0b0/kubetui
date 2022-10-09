#[allow(unused_imports)]
use chrono::{DateTime, Duration, Utc};

#[allow(dead_code)]
pub fn age(duration: Duration) -> String {
    let duration_seconds = duration.num_seconds();

    let seconds = duration_seconds % 60;
    let minutes = duration_seconds / 60;
    let hours = duration_seconds / 3600;
    let days = (duration_seconds / 3600) / 24;

    if 0 < days && 28 < hours {
        return format!("{}d", days);
    }
    if 0 < hours {
        return format!("{}h", hours);
    }
    if 0 < minutes {
        return format!("{}m", minutes);
    }
    format!("{}s", seconds)
}

#[allow(dead_code)]
pub mod color {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seconds() {
        let duration = Duration::seconds(6);
        assert_eq!(age(duration), "6s");
    }
    #[test]
    fn minutes() {
        let duration = Duration::minutes(6);
        assert_eq!(age(duration), "6m");
        let duration = Duration::seconds(61);
        assert_eq!(age(duration), "1m");
    }
    #[test]
    fn hours() {
        let duration = Duration::hours(10);
        assert_eq!(age(duration), "10h");
        let duration = Duration::minutes(61);
        assert_eq!(age(duration), "1h");
        let duration = Duration::hours(28);
        assert_eq!(age(duration), "28h");
    }
    #[test]
    fn days() {
        let duration = Duration::days(10);
        assert_eq!(age(duration), "10d");
        let duration = Duration::hours(29);
        assert_eq!(age(duration), "1d");
    }
}
