use ratatui::style::{Color, Modifier, Style};
use unicode_width::UnicodeWidthStr;

use crate::{
    ansi::{AnsiEscapeSequence, TextParser},
    config::theme::ThemeConfig,
    features::component_id::HELP_DIALOG_ID,
    ui::widget::{
        ansi_color::style_to_ansi, SearchForm, SearchFormTheme, Text, TextTheme, Widget,
        WidgetBase, WidgetTheme,
    },
};

const LEFT_HELP_TEXT: &[HelpBlock] = &[
    HelpBlock {
        title: "General",
        bindings: &[
            KeyBindings {
                keys: &["0~6"],
                desc: "switch tab",
            },
            KeyBindings {
                keys: &["Enter"],
                desc: "select",
            },
            KeyBindings {
                keys: &["c"],
                desc: "change context",
            },
            KeyBindings {
                keys: &["n"],
                desc: "select namespace",
            },
            KeyBindings {
                keys: &["N"],
                desc: "select namespaces",
            },
            KeyBindings {
                keys: &["Tab"],
                desc: "change focus",
            },
            KeyBindings {
                keys: &["y"],
                desc: "open yaml dialog",
            },
            KeyBindings {
                keys: &["q", "Esc"],
                desc: "quit",
            },
            KeyBindings {
                keys: &["q", "Esc"],
                desc: "close dialog",
            },
            KeyBindings {
                keys: &["h", "?"],
                desc: "Show this help",
            },
        ],
    },
    HelpBlock {
        title: "View Control",
        bindings: &[
            KeyBindings {
                keys: &["j", "k", "Down", "Up"],
                desc: "goto next/previous line",
            },
            KeyBindings {
                keys: &["PgDn", "PgUp"],
                desc: "scroll upward/downward",
            },
            KeyBindings {
                keys: &["Left", "Right"],
                desc: "scroll horizontal",
            },
            KeyBindings {
                keys: &["g"],
                desc: "goto first line",
            },
            KeyBindings {
                keys: &["G"],
                desc: "goto last line",
            },
        ],
    },
    HelpBlock {
        title: "Remap Keys",
        bindings: &[
            KeyBindings {
                keys: &["Ctrl-p"],
                desc: "↑",
            },
            KeyBindings {
                keys: &["Ctrl-n"],
                desc: "↓",
            },
            KeyBindings {
                keys: &["Ctrl-f"],
                desc: "→",
            },
            KeyBindings {
                keys: &["Ctrl-b"],
                desc: "←",
            },
            KeyBindings {
                keys: &["Ctrl-u"],
                desc: "PgUp",
            },
            KeyBindings {
                keys: &["Ctrl-d"],
                desc: "PgDn",
            },
            KeyBindings {
                keys: &["Ctrl-h", "BS"],
                desc: "Del",
            },
            KeyBindings {
                keys: &["Ctrl-a"],
                desc: "Home",
            },
            KeyBindings {
                keys: &["Ctrl-e"],
                desc: "End",
            },
            KeyBindings {
                keys: &["Ctrl-["],
                desc: "Esc",
            },
        ],
    },
];

const RIGHT_HELP_TEXT: &[HelpBlock] = &[
    HelpBlock {
        title: "Input Form",
        bindings: &[
            KeyBindings {
                keys: &["Ctrl-a", "Home"],
                desc: "move the cursor to the first",
            },
            KeyBindings {
                keys: &["Ctrl-e", "End"],
                desc: "move the cursor to the end",
            },
            KeyBindings {
                keys: &["Ctrl-f", "Right"],
                desc: "move the cursor to the right",
            },
            KeyBindings {
                keys: &["Ctrl-b", "Left"],
                desc: "move the cursor to the left",
            },
            KeyBindings {
                keys: &["Ctrl-w"],
                desc: "delete the text from the cursor position to the first",
            },
            KeyBindings {
                keys: &["Ctrl-k"],
                desc: "delete the text from the cursor position to the end",
            },
        ],
    },
    HelpBlock {
        title: "API / Yaml Tab",
        bindings: &[KeyBindings {
            keys: &["f"],
            desc: "open select dialog",
        }],
    },
    HelpBlock {
        title: "Search (Only text view)",
        bindings: &[
            KeyBindings {
                keys: &["/"],
                desc: "enable search mode",
            },
            KeyBindings {
                keys: &["q", "Esc"],
                desc: "disable search mode",
            },
            KeyBindings {
                keys: &["Enter"],
                desc: "confirm search word",
            },
            KeyBindings {
                keys: &["n", "N"],
                desc: "goto next/prev word",
            },
        ],
    },
    HelpBlock {
        title: "Filter (Only table view)",
        bindings: &[
            KeyBindings {
                keys: &["/"],
                desc: "open filter form",
            },
            KeyBindings {
                keys: &["q", "Esc"],
                desc: "clear filter form",
            },
            KeyBindings {
                keys: &["Enter"],
                desc: "confirm filter word",
            },
        ],
    },
    HelpBlock {
        title: "Log",
        bindings: &[
            KeyBindings {
                keys: &["Enter"],
                desc: "insert blank line",
            },
            KeyBindings {
                keys: &["f", "p"],
                desc: "toggle json pretty print",
            },
        ],
    },
    HelpBlock {
        title: "Pod Columns",
        bindings: &[
            KeyBindings {
                keys: &["j", "k", "Up", "Down"],
                desc: "move cursor up/down",
            },
            KeyBindings {
                keys: &["g", "G", "PgUp", "PgDn"],
                desc: "move cursor to the first/last line",
            },
            KeyBindings {
                keys: &["Space", "Enter"],
                desc: "toggle column visibility",
            },
            KeyBindings {
                keys: &["J", "K"],
                desc: "move column up/down",
            },
        ],
    },
];

struct KeyBindings {
    keys: &'static [&'static str],
    desc: &'static str,
}

impl KeyBindings {
    fn keys(&self) -> String {
        self.keys.join(" ")
    }

    fn desc(&self) -> String {
        self.desc.to_string()
    }
}

#[derive(Clone)]
struct HelpBlock {
    title: &'static str,
    bindings: &'static [KeyBindings],
}

fn print_help_block(block: &HelpBlock, theme: &HelpItemTheme) -> Vec<String> {
    let mut line = Vec::new();

    line.push(format!(
        "{}[ {} ]\x1b[39m",
        style_to_ansi(theme.title_style),
        block.title
    ));

    let max_key_len = block
        .bindings
        .iter()
        .map(|b| b.keys().width())
        .max()
        .expect("no bindings");

    let lines: Vec<String> = block
        .bindings
        .iter()
        .map(|b| {
            format!(
                "{}{:>pad$}:\x1b[39m {}{}",
                style_to_ansi(theme.key_style),
                b.keys(),
                style_to_ansi(theme.desc_style),
                b.desc(),
                pad = max_key_len
            )
        })
        .collect();

    line.extend(lines);

    line
}

fn print_help_blocks(blocks: &[HelpBlock], theme: &HelpItemTheme) -> Vec<String> {
    blocks
        .iter()
        .flat_map(|block| {
            let mut lines = print_help_block(block, theme);
            lines.push("".to_string());
            lines
        })
        .collect()
}

fn generate(theme: HelpItemTheme) -> Vec<String> {
    let mut left = print_help_blocks(LEFT_HELP_TEXT, &theme);

    let mut right = print_help_blocks(RIGHT_HELP_TEXT, &theme);

    let len = left.len().max(right.len());

    left.resize(len, String::default());
    right.resize(len, String::default());

    // 見える文字数が一番長い行
    let view_padding: usize = left
        .iter()
        .map(|l| {
            // １行当たりの見える文字数
            l.ansi_parse()
                .filter(|p| p.ty == AnsiEscapeSequence::Chars)
                .map(|c| c.chars.width())
                .sum::<usize>()
        })
        .max()
        .expect("no lines");

    left.iter()
        .zip(right)
        .map(|(l, r)| {
            // 制御文字のascii文字を計算して調整
            let escape_len = l.ansi_parse().fold(0, |len, p| match p.ty {
                AnsiEscapeSequence::Chars => len,
                _ => len + p.chars.len(),
            });

            let pad = view_padding + escape_len;
            format!(" {:pad$}   {}", l, r)
        })
        .collect()
}

#[derive(Clone)]
pub struct HelpItemTheme {
    pub title_style: Style,
    pub key_style: Style,
    pub desc_style: Style,
}

impl Default for HelpItemTheme {
    fn default() -> Self {
        Self {
            title_style: Style::default().add_modifier(Modifier::BOLD),
            key_style: Style::default().fg(Color::LightCyan),
            desc_style: Style::default(),
        }
    }
}

#[derive(Debug)]
pub struct HelpDialog {
    pub widget: Widget<'static>,
}

impl HelpDialog {
    pub fn new(theme: ThemeConfig) -> Self {
        let widget_theme = WidgetTheme::from(theme.component.clone());
        let text_theme = TextTheme::from(theme.component.clone());
        let search_theme = SearchFormTheme::from(theme.component.clone());

        let widget_base = WidgetBase::builder()
            .title("Help")
            .theme(widget_theme)
            .build();

        let search_form = SearchForm::builder().theme(search_theme).build();

        let item_theme = HelpItemTheme::from(theme.help.clone());

        Self {
            widget: Text::builder()
                .id(HELP_DIALOG_ID)
                .widget_base(widget_base)
                .search_form(search_form)
                .theme(text_theme)
                .items(generate(item_theme))
                .build()
                .into(),
        }
    }
}
