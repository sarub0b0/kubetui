use serde::{Deserialize, Serialize};

use super::{FocusableThemeStyle, ThemeStyle};

/// コンポーネントのテーマ
#[derive(Default, Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ComponentTheme {
    pub title: ThemeStyle,

    pub border: FocusableThemeStyle,

    pub body: ThemeStyle,
}

#[cfg(test)]
mod tests {
    use super::*;

    use indoc::indoc;
    use pretty_assertions::assert_eq;
    use ratatui::style::{Color, Modifier};

    #[test]
    fn default_event_theme() {
        let actual = ComponentTheme::default();

        let expected = ComponentTheme {
            title: ThemeStyle::default(),
            border: FocusableThemeStyle::default(),
            body: ThemeStyle::default(),
        };

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_serialize_event_theme() {
        let theme = ComponentTheme {
            title: ThemeStyle {
                fg_color: Color::Red,
                bg_color: Color::Blue,
                modifier: Modifier::BOLD,
            },
            border: FocusableThemeStyle {
                active: ThemeStyle {
                    fg_color: Color::Green,
                    bg_color: Color::Yellow,
                    modifier: Modifier::ITALIC,
                },
                inactive: ThemeStyle {
                    fg_color: Color::DarkGray,
                    bg_color: Color::default(),
                    modifier: Modifier::default(),
                },
                mouse_over: ThemeStyle::default(),
            },
            body: ThemeStyle {
                fg_color: Color::Blue,
                bg_color: Color::Red,
                modifier: Modifier::ITALIC,
            },
        };

        let actual = serde_yaml::to_string(&theme).unwrap();

        let expected = indoc! {r#"
            title:
              fg_color: red
              bg_color: blue
              modifier: bold
            border:
              active:
                fg_color: green
                bg_color: yellow
                modifier: italic
              inactive:
                fg_color: darkgray
                bg_color: default
                modifier: none
              mouse_over:
                fg_color: default
                bg_color: default
                modifier: none
            body:
              fg_color: blue
              bg_color: red
              modifier: italic
        "#};

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_deserialize_event_theme() {
        let data = indoc! {r#"
            title:
              fg_color: red
              bg_color: blue
              modifier: bold
            border:
              active:
                fg_color: green
                bg_color: yellow
                modifier: italic
              inactive:
                fg_color: default
                bg_color: default
                modifier: none
            body:
              fg_color: blue
              bg_color: red
              modifier: italic
        "#};

        let actual: ComponentTheme = serde_yaml::from_str(data).unwrap();

        let expected = ComponentTheme {
            title: ThemeStyle {
                fg_color: Color::Red,
                bg_color: Color::Blue,
                modifier: Modifier::BOLD,
            },
            border: FocusableThemeStyle {
                active: ThemeStyle {
                    fg_color: Color::Green,
                    bg_color: Color::Yellow,
                    modifier: Modifier::ITALIC,
                },
                inactive: ThemeStyle::default(),
                mouse_over: ThemeStyle::default(),
            },
            body: ThemeStyle {
                fg_color: Color::Blue,
                bg_color: Color::Red,
                modifier: Modifier::ITALIC,
            },
        };

        assert_eq!(actual, expected);
    }
}
