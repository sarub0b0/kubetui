pub mod pane;
pub mod popup;
pub mod tab;
pub mod window;

pub use pane::Pane;
pub use popup::Popup;
pub use tab::Tab;
pub use window::Window;

use tui::{
    style::{Color, Modifier, Style},
    text::{Span, Spans},
};

fn generate_title(title: &str, border_color: Color, selected: bool) -> Spans {
    let prefix = if selected { "✔︎ " } else { "──" };
    Spans::from(vec![
        Span::styled("─", Style::default()),
        Span::styled(prefix, Style::default().fg(border_color)),
        Span::styled(
            title,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
    ])
}
