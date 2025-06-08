use std::collections::HashMap;

use anyhow::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PodColumns {
    pub columns: Vec<&'static str>,
}

impl PodColumns {
    #[allow(dead_code)]
    pub fn new(columns: impl IntoIterator<Item = &'static str>) -> Self {
        Self {
            columns: columns.into_iter().collect(),
        }
    }
}

const COLUMN_MAP: [(&str, &str); 9] = [
    ("name", "Name"),
    ("ready", "Ready"),
    ("status", "Status"),
    ("restarts", "Restarts"),
    ("age", "Age"),
    ("ip", "IP"),
    ("node", "Node"),
    ("nominatednode", "Nominated Node"),
    ("readinessgates", "Readiness Gates"),
];

fn valid_columns() -> String {
    COLUMN_MAP
        .iter()
        .map(|(k, _)| *k)
        .collect::<Vec<&str>>()
        .join(", ")
}

pub fn parse_pod_columns(input: &str) -> Result<PodColumns> {
    let entries: Vec<&str> = input
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();

    if entries.is_empty() {
        return Err(anyhow::anyhow!("Columns list must not be empty",));
    }

    let has_full = entries.iter().any(|e| normalize_column(e) == "full");
    if has_full && entries.len() > 1 {
        return Err(anyhow::anyhow!(
            "Cannot specify 'full' with other columns. Use 'full' alone to get all columns."
        ));
    }

    if entries.len() == 1 && has_full {
        return Ok(PodColumns {
            columns: COLUMN_MAP.iter().map(|(_, v)| *v).collect::<Vec<&str>>(),
        });
    }

    let column_map: HashMap<&str, &str> = COLUMN_MAP.into_iter().collect();

    let mut result = Vec::new();

    for column in entries {
        let normalized = normalize_column(column);

        if let Some(&display_name) = column_map.get(normalized.as_str()) {
            result.push(display_name);
        } else {
            return Err(anyhow::anyhow!(
                "Invalid column name: {}. Valid options are: {}",
                column,
                valid_columns()
            ));
        }
    }

    if !result.contains(&"Name") {
        result.insert(0, "Name");
    }

    Ok(PodColumns { columns: result })
}

fn normalize_column(column: &str) -> String {
    column.to_lowercase().replace([' ', '_', '-'], "")
}

#[cfg(test)]
mod tests {
    use super::*;

    mod parse_pod_columns {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn 空文字列を渡すとパニックする() {
            let input = "";
            let result = parse_pod_columns(input);
            assert!(result.is_err());
        }

        #[test]
        fn フルを渡すと全カラムを返す() {
            let input = "full";
            let actual = parse_pod_columns(input).unwrap();
            let expected: Vec<String> = COLUMN_MAP.iter().map(|(_, v)| v.to_string()).collect();
            assert_eq!(actual.columns, expected);
        }

        #[test]
        fn カンマ区切りのカラム名を渡すと対応するカラム名を返す() {
            let input = "name, ready, status";
            let actual = parse_pod_columns(input).unwrap();
            let expected = vec!["Name", "Ready", "Status"];
            assert_eq!(actual.columns, expected);
        }

        #[test]
        fn カラム名に空白が含まれていても正しく処理される() {
            let input = "  name ,  ready , status ";
            let actual = parse_pod_columns(input).unwrap();
            let expected = vec!["Name", "Ready", "Status"];
            assert_eq!(actual.columns, expected);
        }

        #[test]
        fn アンダースコアやハイフンを含むカラム名も正しく処理される() {
            let input = "name, nominated_node, readiness-gates";
            let actual = parse_pod_columns(input).unwrap();
            let expected = vec!["Name", "Nominated Node", "Readiness Gates"];
            assert_eq!(actual.columns, expected);
        }

        #[test]
        fn 無効なカラム名が含まれているとエラーを返す() {
            let input = "name, invalid_column";
            let result = parse_pod_columns(input);
            assert!(result.is_err());
            assert_eq!(
                result.unwrap_err().to_string(),
                "Invalid column name: invalid_column. Valid options are: name, ready, status, restarts, age, ip, node, nominatednode, readinessgates"
            );
        }

        #[test]
        #[allow(non_snake_case)]
        fn Nameカラムが常に含まれる() {
            let input = "ready, status";
            let actual = parse_pod_columns(input).unwrap();
            assert!(actual.columns.contains(&"Name"));
        }

        #[test]
        fn fullと他のカラムを同時に指定するとエラー() {
            let input = "full, ready";
            let result = parse_pod_columns(input);
            assert!(result.is_err());
            assert_eq!(
                result.unwrap_err().to_string(),
                "Cannot specify 'full' with other columns. Use 'full' alone to get all columns."
            );
        }

        #[test]
        #[allow(non_snake_case)]
        fn full単体ならOK() {
            let input = "full";
            let actual = parse_pod_columns(input).unwrap();
            let expected: Vec<&str> = COLUMN_MAP.iter().map(|(_, v)| *v).collect();
            assert_eq!(actual.columns, expected);
        }

        #[test]
        fn 空要素が含まれていても無視される() {
            let input = "ready,,status";
            let actual = parse_pod_columns(input).unwrap();
            let expected = vec!["Name", "Ready", "Status"];
            assert_eq!(actual.columns, expected);
        }

        #[test]
        fn 空要素だけだとエラーになる() {
            let input = ", , ";
            let result = parse_pod_columns(input);
            assert!(result.is_err());
            assert_eq!(
                result.unwrap_err().to_string(),
                "Columns list must not be empty"
            );
        }
    }

    mod normalize_column {
        use super::*;

        #[test]
        fn 空白を削除して小文字に変換する() {
            let name = "  Name  ";
            let actual = normalize_column(name);
            assert_eq!(actual, "name");
        }

        #[test]
        fn アンダースコアを削除して小文字に変換する() {
            let name = "Nominated_Node";
            let actual = normalize_column(name);
            assert_eq!(actual, "nominatednode");
        }

        #[test]
        fn ハイフンを削除して小文字に変換する() {
            let name = "Readiness-Gates";
            let actual = normalize_column(name);
            assert_eq!(actual, "readinessgates");
        }
    }
}
