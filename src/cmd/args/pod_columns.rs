use std::str::FromStr as _;

use anyhow::Result;
use strum::IntoEnumIterator;

use crate::features::pod::{PodColumn, PodColumns};

fn valid_columns() -> String {
    PodColumn::iter()
        .map(|column| column.normalize())
        .collect::<Vec<_>>()
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

    let has_full = entries
        .iter()
        .any(|e| PodColumn::normalize_column(e) == "full");

    if has_full && entries.len() > 1 {
        return Err(anyhow::anyhow!(
            "Cannot specify 'full' with other columns. Use 'full' alone to get all columns."
        ));
    }

    if entries.len() == 1 && has_full {
        return Ok(PodColumns {
            columns: PodColumn::iter().collect::<Vec<_>>(),
        });
    }

    let mut columns = Vec::new();

    for column in entries {
        let normalized = PodColumn::normalize_column(column);

        if let Ok(display_name) = PodColumn::from_str(normalized.as_str()) {
            columns.push(display_name);
        } else {
            return Err(anyhow::anyhow!(
                "Invalid column name: {}. Valid options are: {}",
                column,
                valid_columns()
            ));
        }
    }

    if !columns.contains(&PodColumn::Name) {
        columns.insert(0, PodColumn::Name);
    }

    Ok(PodColumns { columns })
}

#[cfg(test)]
mod tests {
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
        let expected: Vec<PodColumn> = PodColumn::iter().collect();
        assert_eq!(actual.columns, expected);
    }

    #[test]
    fn カンマ区切りのカラム名を渡すと対応するカラム名を返す() {
        let input = "name, ready, status";
        let actual = parse_pod_columns(input).unwrap();
        let expected = vec![PodColumn::Name, PodColumn::Ready, PodColumn::Status];
        assert_eq!(actual.columns, expected);
    }

    #[test]
    fn カラム名に空白が含まれていても正しく処理される() {
        let input = "  name ,  ready , status ";
        let actual = parse_pod_columns(input).unwrap();
        let expected = vec![PodColumn::Name, PodColumn::Ready, PodColumn::Status];
        assert_eq!(actual.columns, expected);
    }

    #[test]
    fn アンダースコアやハイフンを含むカラム名も正しく処理される() {
        let input = "name, nominated_node, readiness-gates";
        let actual = parse_pod_columns(input).unwrap();
        let expected = vec![
            PodColumn::Name,
            PodColumn::NominatedNode,
            PodColumn::ReadinessGates,
        ];
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
        assert!(actual.columns.contains(&PodColumn::Name));
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
        let actual = parse_pod_columns(input);
        assert_eq!(actual.is_ok(), true);
    }

    #[test]
    fn 空要素が含まれていても無視される() {
        let input = "ready,,status";
        let actual = parse_pod_columns(input).unwrap();
        let expected = vec![PodColumn::Name, PodColumn::Ready, PodColumn::Status];
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
