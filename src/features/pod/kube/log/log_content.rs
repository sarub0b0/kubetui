pub struct LogContent {
    pub prefix: String,
    pub content: String,
}

impl LogContent {
    pub fn print(&self) -> String {
        format!("{} {}", self.prefix, self.content)
    }

    pub fn try_json_pritty_print(&self) -> Vec<String> {
        let Ok(json) = serde_json::from_str::<serde_json::Value>(&self.content) else {
            return vec![self.print()];
        };

        let Ok(pretty) = serde_json::to_string_pretty(&json) else {
            return vec![self.print()];
        };

        pretty
            .lines()
            .map(|line| format!("{}  {}", self.prefix, line))
            .collect()
    }
}
