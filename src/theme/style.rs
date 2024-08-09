use ratatui::style::{Color, Modifier};
use serde::{Deserialize, Serialize};

/// フォーカスイベントありのスタイル
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct FocusableThemeStyle {
    #[serde(default)]
    pub active: ThemeStyle,

    #[serde(default = "default_inactive")]
    pub inactive: ThemeStyle,

    #[serde(default)]
    pub mouse_over: ThemeStyle,
}

impl Default for FocusableThemeStyle {
    fn default() -> Self {
        Self {
            active: ThemeStyle::default(),
            inactive: default_inactive(),
            mouse_over: ThemeStyle::default(),
        }
    }
}

/// FocusableThemeStyleのinactiveのデフォルト値となるThemeStyleを返す
fn default_inactive() -> ThemeStyle {
    ThemeStyle {
        fg_color: Color::DarkGray,
        ..Default::default()
    }
}

/// Theme用のスタイル
/// - 上位レイヤーで指定されていない場合は下位レイヤーのスタイルを継承する
/// - 上位レイヤーで指定されている場合は上位レイヤーのスタイルを優先する
/// - Modifierは加算方式
#[derive(Default, Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ThemeStyle {
    #[serde(with = "serde_color", default)]
    pub fg_color: Color,

    #[serde(with = "serde_color", default)]
    pub bg_color: Color,

    #[serde(with = "serde_modifier", default)]
    pub modifier: Modifier,
}

impl ThemeStyle {
    pub fn to_style(&self) -> ratatui::style::Style {
        ratatui::style::Style::new()
            .fg(self.fg_color)
            .bg(self.bg_color)
            .add_modifier(self.modifier)
    }
}

/// Modifierに対して大文字・小文字を区別せずにパースできるように拡張する
mod serde_modifier {
    use serde::Deserialize as _;

    use super::Modifier;

    pub fn serialize<S>(modifier: &Modifier, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if modifier.is_empty() {
            return serializer.serialize_str("none");
        }

        let s = serde_yaml::to_string(modifier)
            .map_err(serde::ser::Error::custom)?
            .trim_end()
            .to_lowercase();

        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Modifier, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;

        let s = s.to_uppercase();

        match s.as_str() {
            "NONE" => Ok(Modifier::empty()),
            _ => serde_yaml::from_str(&s).map_err(serde::de::Error::custom),
        }
    }
}

mod serde_color {
    use serde::Deserialize as _;

    use super::Color;

    /// Colorをシリアライズした結果を小文字に変換する
    pub fn serialize<S>(color: &Color, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if *color == Color::default() {
            return serializer.serialize_str("default");
        }

        let s = serde_yaml::to_string(color)
            .map_err(serde::ser::Error::custom)?
            .trim_end()
            .to_lowercase();

        serializer.serialize_str(&s)
    }

    /// Colorをデシリアライズする際に"default"文字列をサポートし、Color::Defaultに変換する
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Color, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;

        match s.as_str() {
            "default" => Ok(Color::default()),
            _ => serde_yaml::from_str(&s).map_err(serde::de::Error::custom),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod focusable {
        use super::*;

        use indoc::indoc;
        use pretty_assertions::assert_eq;
        use ratatui::style::Modifier;

        #[test]
        fn default_focusable_style() {
            let actual = FocusableThemeStyle::default();

            let expected = FocusableThemeStyle {
                active: ThemeStyle::default(),
                inactive: ThemeStyle {
                    fg_color: Color::DarkGray,
                    bg_color: Color::default(),
                    modifier: Modifier::default(),
                },
                mouse_over: ThemeStyle::default(),
            };

            assert_eq!(actual, expected);
        }

        #[test]
        fn serialize_focusable_style() {
            let style = FocusableThemeStyle {
                active: ThemeStyle {
                    fg_color: Color::Red,
                    bg_color: Color::Blue,
                    modifier: Modifier::BOLD,
                },
                inactive: ThemeStyle {
                    fg_color: Color::Green,
                    bg_color: Color::Yellow,
                    modifier: Modifier::ITALIC,
                },
                mouse_over: ThemeStyle {
                    fg_color: Color::Cyan,
                    bg_color: Color::Magenta,
                    modifier: Modifier::UNDERLINED,
                },
            };

            let serialized = serde_yaml::to_string(&style).unwrap();

            let expected = indoc! {r#"
                active:
                  fg_color: red
                  bg_color: blue
                  modifier: bold
                inactive:
                  fg_color: green
                  bg_color: yellow
                  modifier: italic
                mouse_over:
                  fg_color: cyan
                  bg_color: magenta
                  modifier: underlined
            "#};

            assert_eq!(serialized, expected);
        }

        #[test]
        fn deserialize_focusable_style() {
            let data = indoc! {r#"
                active:
                  fg_color: red
                  bg_color: blue
                  modifier: bold
                inactive:
                  fg_color: green
                  bg_color: yellow
                  modifier: italic
                mouse_over:
                  fg_color: cyan
                  bg_color: magenta
                  modifier: underlined
            "#};

            let actual: FocusableThemeStyle = serde_yaml::from_str(data).unwrap();

            let expected = FocusableThemeStyle {
                active: ThemeStyle {
                    fg_color: Color::Red,
                    bg_color: Color::Blue,
                    modifier: Modifier::BOLD,
                },
                inactive: ThemeStyle {
                    fg_color: Color::Green,
                    bg_color: Color::Yellow,
                    modifier: Modifier::ITALIC,
                },
                mouse_over: ThemeStyle {
                    fg_color: Color::Cyan,
                    bg_color: Color::Magenta,
                    modifier: Modifier::UNDERLINED,
                },
            };

            assert_eq!(actual, expected);
        }

        /// 空文字を与えたときにDefault値が返ることを確認する
        #[test]
        fn deserialize_focusable_style_empty() {
            let data = "";

            let actual: FocusableThemeStyle = serde_yaml::from_str(data).unwrap();

            let expected = FocusableThemeStyle::default();

            assert_eq!(actual, expected);
        }
    }

    mod style {
        use super::*;

        use indoc::indoc;
        use pretty_assertions::assert_eq;

        #[test]
        fn default_theme_style() {
            let actual = ThemeStyle::default();

            let expected = ThemeStyle {
                fg_color: Color::default(),
                bg_color: Color::default(),
                modifier: Modifier::empty(),
            };

            assert_eq!(actual, expected);
        }

        #[test]
        fn serialize_theme_style() {
            let theme = ThemeStyle {
                fg_color: Color::Red,
                bg_color: Color::Blue,
                modifier: Modifier::BOLD | Modifier::ITALIC,
            };

            let actual = serde_yaml::to_string(&theme).unwrap();

            let expected = indoc! { "
                fg_color: red
                bg_color: blue
                modifier: bold | italic
            " };

            assert_eq!(actual, expected);
        }

        #[test]
        fn deserialize_theme_style() {
            let yaml = indoc! { "
                fg_color: red
                bg_color: blue
                modifier: bold | italic
            " };

            let actual: ThemeStyle = serde_yaml::from_str(yaml).unwrap();

            let expected = ThemeStyle {
                fg_color: Color::Red,
                bg_color: Color::Blue,
                modifier: Modifier::BOLD | Modifier::ITALIC,
            };

            assert_eq!(actual, expected);
        }

        /// 空文字を与えたときにDefault値が返ることを確認する
        #[test]
        fn deserialize_theme_style_empty_string() {
            let yaml = "";

            let actual: ThemeStyle = serde_yaml::from_str(yaml).unwrap();

            let expected = ThemeStyle::default();

            assert_eq!(actual, expected);
        }

        /// "default"を与えたときにColor::default()が返ることを確認する
        #[test]
        fn deserialize_color_default_string() {
            let yaml = indoc! { "
                fg_color: default
                bg_color: default
            " };

            let actual: ThemeStyle = serde_yaml::from_str(yaml).unwrap();

            let expected = ThemeStyle {
                fg_color: Color::default(),
                bg_color: Color::default(),
                modifier: Modifier::empty(),
            };

            assert_eq!(actual, expected);
        }

        /// "none"を与えたときにModifier::empty()が返ることを確認する
        #[test]
        fn deserialize_modifier_none_string() {
            let yaml = indoc! { "
                modifier: none
            " };

            let actual: ThemeStyle = serde_yaml::from_str(yaml).unwrap();

            let expected = ThemeStyle {
                fg_color: Color::default(),
                bg_color: Color::default(),
                modifier: Modifier::empty(),
            };

            assert_eq!(actual, expected);
        }
    }
}
