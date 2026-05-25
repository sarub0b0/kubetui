use anyhow::Result;

/// Parse `--node-columns` into a list of column name references.
///
/// Resolution (builtin vs label) happens later in `app.rs` against the label
/// registry, so this only splits/trims the names.
pub fn parse_node_columns(input: &str) -> Result<Vec<String>> {
    let entries: Vec<String> = input
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();

    if entries.is_empty() {
        return Err(anyhow::anyhow!("Columns list must not be empty"));
    }

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn splits_and_trims_names() {
        assert_eq!(
            parse_node_columns("name, status ,roles").unwrap(),
            vec![
                "name".to_string(),
                "status".to_string(),
                "roles".to_string()
            ]
        );
    }

    #[test]
    fn keeps_label_names_as_is() {
        assert_eq!(
            parse_node_columns("name,mig").unwrap(),
            vec!["name".to_string(), "mig".to_string()]
        );
    }

    #[test]
    fn empty_is_error() {
        assert!(parse_node_columns("  ").is_err());
    }
}
