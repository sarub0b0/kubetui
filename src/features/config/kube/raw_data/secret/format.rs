use crate::workers::kube::color::fg::Color;

pub(super) fn format_utf8(key: &str, value: &str, color: u8) -> String {
    if value.contains('\n') {
        let mut ret = format!("\x1b[{color}m{key}:\x1b[39m |\n");

        value.lines().for_each(|l| {
            ret += &format!("  {l}\n");
        });

        ret.trim_end().to_string()
    } else {
        format!("\x1b[{color}m{key}:\x1b[39m {value}")
    }
}

pub(super) fn format_error(key: &str, value: &str, err: &str, color: u8) -> String {
    format!(
        "\x1b[{color}m{key}:\x1b[39m | \x1b[{error_color}m# {error}\x1b[39m\n  [base64-encoded] {value}",
        color = color,
        key = key,
        value = value,
        error_color = Color::DarkGray as u8,
        error = err
    )
}
