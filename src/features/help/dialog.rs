use unicode_width::UnicodeWidthStr;

use crate::{
    ansi::{AnsiEscapeSequence, TextParser},
    features::component_id::HELP_DIALOG_ID,
    ui::widget::{Text, Widget, WidgetBase},
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
        title: "List / Yaml Tab",
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
        bindings: &[KeyBindings {
            keys: &["Enter"],
            desc: "insert blank line",
        }],
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

impl HelpBlock {
    fn print(&self) -> Vec<String> {
        let mut block = Vec::new();

        block.push(format!("\x1b[1m[ {} ]\x1b[0m", self.title));

        let max_key_len = self
            .bindings
            .iter()
            .map(|b| b.keys().width())
            .max()
            .expect("no bindings");

        let lines: Vec<String> = self
            .bindings
            .iter()
            .map(|b| {
                format!(
                    "\x1b[96m{:>pad$}:\x1b[0m {}",
                    b.keys(),
                    b.desc(),
                    pad = max_key_len
                )
            })
            .collect();

        block.extend(lines);

        block
    }
}

#[derive(Clone)]
struct HelpText {
    blocks: Vec<HelpBlock>,
}

impl HelpText {
    fn new(blocks: Vec<HelpBlock>) -> Self {
        Self { blocks }
    }

    fn print(&self) -> Vec<String> {
        self.blocks
            .iter()
            .flat_map(|b| {
                let mut b = b.print();
                b.push("".to_string());
                b
            })
            .collect()
    }
}

fn generate() -> Vec<String> {
    let mut left = HelpText::new(LEFT_HELP_TEXT.to_vec()).print();

    let mut right = HelpText::new(RIGHT_HELP_TEXT.to_vec()).print();

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

#[derive(Debug)]
pub struct HelpDialog {
    pub widget: Widget<'static>,
}

impl HelpDialog {
    pub fn new() -> Self {
        Self {
            widget: Text::builder()
                .id(HELP_DIALOG_ID)
                .widget_base(WidgetBase::builder().title("Help").build())
                .items(generate())
                .build()
                .into(),
        }
    }
}
