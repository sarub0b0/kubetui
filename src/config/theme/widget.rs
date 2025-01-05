use ratatui::style::{Color, Modifier};
use serde::{Deserialize, Serialize};

use crate::ui::widget::{
    multiple_select, single_select, table, InputFormTheme, ListTheme, SearchFormTheme, TableTheme,
    TextTheme, WidgetTheme,
};

use super::{
    BorderThemeConfig, DialogThemeConfig, FilterFormThemeConfig, InputFormThemeConfig,
    ListThemeConfig, TableThemeConfig, TextThemeConfig, ThemeStyleConfig,
};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct TitleThemeConfig {
    #[serde(default = "default_title_active")]
    pub active: ThemeStyleConfig,

    #[serde(default = "default_title_inactive")]
    pub inactive: ThemeStyleConfig,
}

impl Default for TitleThemeConfig {
    fn default() -> Self {
        Self {
            active: default_title_active(),
            inactive: default_title_inactive(),
        }
    }
}

fn default_title_active() -> ThemeStyleConfig {
    ThemeStyleConfig {
        modifier: Modifier::BOLD,
        ..Default::default()
    }
}

fn default_title_inactive() -> ThemeStyleConfig {
    ThemeStyleConfig {
        fg_color: Some(Color::DarkGray),
        ..Default::default()
    }
}

/// コンポーネントのテーマ
#[derive(Default, Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct WidgetThemeConfig {
    #[serde(default)]
    pub base: ThemeStyleConfig,

    #[serde(default)]
    pub title: TitleThemeConfig,

    #[serde(default)]
    pub border: BorderThemeConfig,

    #[serde(default)]
    pub text: TextThemeConfig,

    #[serde(default)]
    pub table: TableThemeConfig,

    #[serde(default)]
    pub list: ListThemeConfig,

    #[serde(default)]
    pub input: InputFormThemeConfig,

    #[serde(default)]
    pub dialog: DialogThemeConfig,
}

impl From<WidgetThemeConfig> for WidgetTheme {
    fn from(theme: WidgetThemeConfig) -> Self {
        WidgetTheme::default()
            .base_style(theme.base)
            .title_active_style(theme.title.active)
            .title_inactive_style(theme.title.inactive)
            .border_type(theme.border.ty)
            .border_active_style(theme.border.active)
            .border_mouse_over_style(theme.border.mouse_over)
            .border_inactive_style(theme.border.inactive)
    }
}

impl From<WidgetThemeConfig> for TableTheme {
    fn from(theme: WidgetThemeConfig) -> Self {
        Self::default().header_style(theme.table.header)
    }
}

impl From<WidgetThemeConfig> for SearchFormTheme {
    fn from(theme: WidgetThemeConfig) -> Self {
        let base_style = theme.text.search.form.base.unwrap_or(theme.base);

        Self::default()
            .base_style(base_style)
            .input_form_theme(theme.text.search.form)
    }
}

impl From<WidgetThemeConfig> for InputFormTheme {
    fn from(theme: WidgetThemeConfig) -> Self {
        Self::default().content_style(theme.input)
    }
}

impl From<WidgetThemeConfig> for TextTheme {
    fn from(theme: WidgetThemeConfig) -> Self {
        Self {
            search: theme.text.search.highlight.into(),
            selecton: theme.text.selection.into(),
        }
    }
}

impl From<WidgetThemeConfig> for (WidgetTheme, InputFormTheme) {
    fn from(theme: WidgetThemeConfig) -> Self {
        let FilterFormThemeConfig {
            base,
            border,
            prefix,
            query,
        } = theme.table.filter;

        let border = border.unwrap_or(theme.border);
        let base = base.unwrap_or(theme.base);

        let input_form_theme = InputFormTheme::default()
            .prefix_style(prefix)
            .content_style(query);

        let widget_theme = WidgetTheme::default()
            .base_style(base)
            .border_type(border.ty)
            .border_active_style(border.active)
            .border_inactive_style(border.inactive);

        (widget_theme, input_form_theme)
    }
}

impl From<WidgetThemeConfig> for table::FilterFormTheme {
    fn from(theme: WidgetThemeConfig) -> Self {
        let (widget_theme, input_form_theme) = (theme).into();

        Self::default()
            .widget_theme(widget_theme)
            .input_form_theme(input_form_theme)
    }
}

impl From<WidgetThemeConfig> for multiple_select::FilterFormTheme {
    fn from(theme: WidgetThemeConfig) -> Self {
        let (widget_theme, input_form_theme) = (theme).into();

        Self::default()
            .widget_theme(widget_theme)
            .input_form_theme(input_form_theme)
    }
}

impl From<WidgetThemeConfig> for single_select::FilterFormTheme {
    fn from(theme: WidgetThemeConfig) -> Self {
        let (widget_theme, input_form_theme) = (theme).into();

        Self::default()
            .widget_theme(widget_theme)
            .input_form_theme(input_form_theme)
    }
}

impl From<WidgetThemeConfig> for ListTheme {
    fn from(theme: WidgetThemeConfig) -> Self {
        Self {
            selected: theme.list.selected_item.into(),
        }
    }
}

impl From<WidgetThemeConfig> for multiple_select::SelectFormTheme {
    fn from(theme: WidgetThemeConfig) -> Self {
        Self {
            list_theme: theme.clone().into(),
            widget_theme: theme.into(),
        }
    }
}

impl From<WidgetThemeConfig> for single_select::SelectFormTheme {
    fn from(theme: WidgetThemeConfig) -> Self {
        Self {
            list_theme: theme.clone().into(),
            widget_theme: theme.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::config::theme::{
        DialogSizeThemeConfig, FilterFormThemeConfig, SearchFormThemeConfig,
        SearchHighlightThemeConfig, SearchThemeConfig, SelectionThemeConfig,
    };

    use super::*;

    use indoc::indoc;
    use pretty_assertions::assert_eq;
    use ratatui::{
        style::{Color, Modifier},
        widgets::BorderType,
    };

    #[test]
    fn default_widget_theme_config() {
        let actual = WidgetThemeConfig::default();

        let expected = WidgetThemeConfig {
            base: ThemeStyleConfig::default(),
            title: TitleThemeConfig::default(),
            border: BorderThemeConfig::default(),
            text: TextThemeConfig::default(),
            table: TableThemeConfig::default(),
            list: ListThemeConfig::default(),
            input: InputFormThemeConfig::default(),
            dialog: DialogThemeConfig::default(),
        };

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_serialize_widget_theme_config() {
        let theme = WidgetThemeConfig {
            base: ThemeStyleConfig {
                fg_color: Some(Color::Blue),
                bg_color: Some(Color::Red),
                modifier: Modifier::ITALIC,
            },
            title: TitleThemeConfig {
                active: ThemeStyleConfig {
                    fg_color: Some(Color::Red),
                    bg_color: Some(Color::Blue),
                    modifier: Modifier::BOLD,
                },
                inactive: ThemeStyleConfig {
                    fg_color: Some(Color::Green),
                    bg_color: Some(Color::Yellow),
                    modifier: Modifier::ITALIC,
                },
            },
            border: BorderThemeConfig {
                ty: BorderType::Plain,
                active: ThemeStyleConfig {
                    fg_color: Some(Color::Green),
                    bg_color: Some(Color::Yellow),
                    modifier: Modifier::ITALIC,
                },
                inactive: ThemeStyleConfig {
                    fg_color: Some(Color::DarkGray),
                    bg_color: None,
                    modifier: Modifier::default(),
                },
                mouse_over: ThemeStyleConfig::default(),
            },
            text: TextThemeConfig::default(),
            table: TableThemeConfig {
                filter: FilterFormThemeConfig {
                    base: Some(ThemeStyleConfig {
                        fg_color: Some(Color::White),
                        ..Default::default()
                    }),
                    border: Some(BorderThemeConfig {
                        ty: BorderType::Plain,
                        active: ThemeStyleConfig {
                            fg_color: Some(Color::White),
                            ..Default::default()
                        },
                        inactive: ThemeStyleConfig {
                            fg_color: Some(Color::White),
                            ..Default::default()
                        },
                        mouse_over: ThemeStyleConfig::default(),
                    }),
                    prefix: ThemeStyleConfig {
                        fg_color: Some(Color::White),
                        ..Default::default()
                    },
                    query: ThemeStyleConfig {
                        fg_color: Some(Color::White),
                        ..Default::default()
                    },
                },
                header: ThemeStyleConfig {
                    fg_color: Some(Color::DarkGray),
                    ..Default::default()
                },
            },
            list: ListThemeConfig {
                filter: FilterFormThemeConfig {
                    base: Some(ThemeStyleConfig {
                        fg_color: Some(Color::White),
                        ..Default::default()
                    }),
                    border: Some(BorderThemeConfig {
                        ty: BorderType::Plain,
                        active: ThemeStyleConfig {
                            fg_color: Some(Color::White),
                            ..Default::default()
                        },
                        inactive: ThemeStyleConfig {
                            fg_color: Some(Color::White),
                            ..Default::default()
                        },
                        mouse_over: ThemeStyleConfig::default(),
                    }),
                    prefix: ThemeStyleConfig {
                        fg_color: Some(Color::White),
                        ..Default::default()
                    },
                    query: ThemeStyleConfig {
                        fg_color: Some(Color::White),
                        ..Default::default()
                    },
                },
                selected_item: ThemeStyleConfig {
                    fg_color: Some(Color::Green),
                    modifier: Modifier::REVERSED,
                    ..Default::default()
                },
                status: ThemeStyleConfig {
                    fg_color: Some(Color::Yellow),
                    ..Default::default()
                },
            },
            input: InputFormThemeConfig(ThemeStyleConfig {
                fg_color: Some(Color::White),
                ..Default::default()
            }),
            dialog: DialogThemeConfig {
                base: Some(ThemeStyleConfig {
                    fg_color: Some(Color::Blue),
                    bg_color: Some(Color::Red),
                    modifier: Modifier::ITALIC,
                }),
                size: DialogSizeThemeConfig {
                    width: 85.0,
                    height: 85.0,
                },
            },
        };

        let actual = serde_yaml::to_string(&theme).unwrap();

        let expected = indoc! {r#"
            base:
              fg_color: blue
              bg_color: red
              modifier: italic
            title:
              active:
                fg_color: red
                bg_color: blue
                modifier: bold
              inactive:
                fg_color: green
                bg_color: yellow
                modifier: italic
            border:
              type: plain
              active:
                fg_color: green
                bg_color: yellow
                modifier: italic
              inactive:
                fg_color: darkgray
              mouse_over: {}
            text:
              search:
                form:
                  prefix: {}
                  query: {}
                  suffix: {}
                highlight:
                  focus:
                    fg_color: yellow
                    modifier: reversed
                  matches:
                    modifier: reversed
              selection:
                modifier: reversed
            table:
              filter:
                base:
                  fg_color: white
                border:
                  type: plain
                  active:
                    fg_color: white
                  inactive:
                    fg_color: white
                  mouse_over: {}
                prefix:
                  fg_color: white
                query:
                  fg_color: white
              header:
                fg_color: darkgray
            list:
              filter:
                base:
                  fg_color: white
                border:
                  type: plain
                  active:
                    fg_color: white
                  inactive:
                    fg_color: white
                  mouse_over: {}
                prefix:
                  fg_color: white
                query:
                  fg_color: white
              selected_item:
                fg_color: green
                modifier: reversed
              status:
                fg_color: yellow
            input:
              fg_color: white
            dialog:
              base:
                fg_color: blue
                bg_color: red
                modifier: italic
              size:
                width: 85.0
                height: 85.0
        "#};

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_deserialize_widget_theme_config() {
        let data = indoc! {r#"
            base:
              fg_color: blue
              bg_color: red
              modifier: italic
            title:
              active:
                fg_color: red
                bg_color: blue
                modifier: bold
              inactive:
                fg_color: green
                bg_color: yellow
                modifier: italic
            border:
              active:
                fg_color: green
                bg_color: yellow
                modifier: italic
              inactive: {}
            text:
              search:
                form:
                  base:
                    fg_color: yellow
                highlight:
                  focus:
                    fg_color: yellow
                  matches:
                    fg_color: blue
              selection:
                bg_color: red
            table:
              filter:
                base:
                  fg_color: white
                border:
                  type: plain
                  active:
                    fg_color: white
                  inactive:
                    fg_color: white
                  mouse_over: {}
                prefix:
                  fg_color: white
                query:
                  fg_color: white
              header:
                fg_color: darkgray
            list:
              filter:
                base:
                  fg_color: white
                border:
                  type: plain
                  active:
                    fg_color: white
                  inactive:
                    fg_color: white
                  mouse_over: {}
                prefix:
                  fg_color: white
                query:
                  fg_color: white
              selected_item:
                fg_color: green
                modifier: reversed
              status:
                fg_color: yellow
            input:
              fg_color: white
            dialog:
              base:
                fg_color: blue
                bg_color: red
                modifier: italic
              size:
                width: 85.0
                height: 85.0
        "#};

        let actual: WidgetThemeConfig = serde_yaml::from_str(data).unwrap();

        let expected = WidgetThemeConfig {
            base: ThemeStyleConfig {
                fg_color: Some(Color::Blue),
                bg_color: Some(Color::Red),
                modifier: Modifier::ITALIC,
            },
            title: TitleThemeConfig {
                active: ThemeStyleConfig {
                    fg_color: Some(Color::Red),
                    bg_color: Some(Color::Blue),
                    modifier: Modifier::BOLD,
                },
                inactive: ThemeStyleConfig {
                    fg_color: Some(Color::Green),
                    bg_color: Some(Color::Yellow),
                    modifier: Modifier::ITALIC,
                },
            },

            border: BorderThemeConfig {
                ty: BorderType::Plain,
                active: ThemeStyleConfig {
                    fg_color: Some(Color::Green),
                    bg_color: Some(Color::Yellow),
                    modifier: Modifier::ITALIC,
                },
                inactive: ThemeStyleConfig::default(),
                mouse_over: ThemeStyleConfig {
                    fg_color: Some(Color::Gray),
                    bg_color: None,
                    modifier: Modifier::default(),
                },
            },
            text: TextThemeConfig {
                search: SearchThemeConfig {
                    form: SearchFormThemeConfig {
                        base: Some(ThemeStyleConfig {
                            fg_color: Some(Color::Yellow),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                    highlight: SearchHighlightThemeConfig {
                        focus: ThemeStyleConfig {
                            fg_color: Some(Color::Yellow),
                            ..Default::default()
                        },
                        matches: ThemeStyleConfig {
                            fg_color: Some(Color::Blue),
                            ..Default::default()
                        },
                    },
                },
                selection: SelectionThemeConfig(ThemeStyleConfig {
                    bg_color: Some(Color::Red),
                    ..Default::default()
                }),
            },
            table: TableThemeConfig {
                filter: FilterFormThemeConfig {
                    base: Some(ThemeStyleConfig {
                        fg_color: Some(Color::White),
                        ..Default::default()
                    }),
                    border: Some(BorderThemeConfig {
                        ty: BorderType::Plain,
                        active: ThemeStyleConfig {
                            fg_color: Some(Color::White),
                            ..Default::default()
                        },
                        inactive: ThemeStyleConfig {
                            fg_color: Some(Color::White),
                            ..Default::default()
                        },
                        mouse_over: ThemeStyleConfig::default(),
                    }),
                    prefix: ThemeStyleConfig {
                        fg_color: Some(Color::White),
                        ..Default::default()
                    },
                    query: ThemeStyleConfig {
                        fg_color: Some(Color::White),
                        ..Default::default()
                    },
                },
                header: ThemeStyleConfig {
                    fg_color: Some(Color::DarkGray),
                    ..Default::default()
                },
            },
            list: ListThemeConfig {
                filter: FilterFormThemeConfig {
                    base: Some(ThemeStyleConfig {
                        fg_color: Some(Color::White),
                        ..Default::default()
                    }),
                    border: Some(BorderThemeConfig {
                        ty: BorderType::Plain,
                        active: ThemeStyleConfig {
                            fg_color: Some(Color::White),
                            ..Default::default()
                        },
                        inactive: ThemeStyleConfig {
                            fg_color: Some(Color::White),
                            ..Default::default()
                        },
                        mouse_over: ThemeStyleConfig::default(),
                    }),
                    prefix: ThemeStyleConfig {
                        fg_color: Some(Color::White),
                        ..Default::default()
                    },
                    query: ThemeStyleConfig {
                        fg_color: Some(Color::White),
                        ..Default::default()
                    },
                },
                selected_item: ThemeStyleConfig {
                    fg_color: Some(Color::Green),
                    modifier: Modifier::REVERSED,
                    ..Default::default()
                },
                status: ThemeStyleConfig {
                    fg_color: Some(Color::Yellow),
                    ..Default::default()
                },
            },
            input: InputFormThemeConfig(ThemeStyleConfig {
                fg_color: Some(Color::White),
                ..Default::default()
            }),
            dialog: DialogThemeConfig {
                base: Some(ThemeStyleConfig {
                    fg_color: Some(Color::Blue),
                    bg_color: Some(Color::Red),
                    modifier: Modifier::ITALIC,
                }),
                size: DialogSizeThemeConfig {
                    width: 85.0,
                    height: 85.0,
                },
            },
        };

        assert_eq!(actual, expected);
    }
}
