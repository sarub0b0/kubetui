use std::collections::BTreeMap;

use k8s_openapi::apimachinery::pkg::apis::meta::v1::{LabelSelector, LabelSelectorRequirement};
use kube::ResourceExt;

pub trait ExtractNamespace {
    fn extract_namespace(&self) -> String;
}

impl<K> ExtractNamespace for K
where
    K: ResourceExt,
{
    fn extract_namespace(&self) -> String {
        self.namespace().unwrap_or_else(|| "default".to_string())
    }
}

pub fn label_selector_to_query(selector: Option<LabelSelector>) -> String {
    let Some(LabelSelector {
        match_labels,
        match_expressions,
    }) = selector
    else {
        return "".into();
    };

    let mut query = Vec::new();

    if let Some(match_labels) = match_labels {
        query.append(&mut match_labels_to_query(match_labels));
    }

    if let Some(match_expressions) = match_expressions {
        query.append(&mut match_expressions_to_query(match_expressions));
    }

    query.join(",")
}

/// matchLabelsをクエリパラメーターに変換する
pub fn match_labels_to_query(match_labels: BTreeMap<String, String>) -> Vec<String> {
    match_labels
        .into_iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
}

/// matchExpressionsをクエリパラメーターに変換する
pub fn match_expressions_to_query(match_expressions: Vec<LabelSelectorRequirement>) -> Vec<String> {
    match_expressions
        .into_iter()
        .map(|requirement| {
            let LabelSelectorRequirement {
                key,
                operator,
                values,
            } = requirement;

            // InとNotInのとき、valuesはかならずSomeである
            match operator.as_str() {
                "In" => {
                    format!(
                        "{} in ({})",
                        key,
                        values.map(|values| values.join(", ")).unwrap_or_default()
                    )
                }
                "NotIn" => {
                    format!(
                        "{} notin ({})",
                        key,
                        values.map(|values| values.join(", ")).unwrap_or_default()
                    )
                }
                "Exists" => key.to_string(),
                "DoesNotExist" => {
                    format!("!{key}")
                }
                _ => {
                    unreachable!()
                }
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    mod match_labels_to_query {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn empty() {
            let match_labels: BTreeMap<String, String> = BTreeMap::new();
            let result = super::match_labels_to_query(match_labels);
            assert!(
                result.is_empty(),
                "Result should be empty for an empty input"
            );
        }

        #[test]
        fn single() {
            let mut match_labels: BTreeMap<String, String> = BTreeMap::new();
            match_labels.insert("key".to_string(), "value".to_string());
            let result = match_labels_to_query(match_labels);
            assert_eq!(
                result,
                vec!["key=value"],
                "Result should contain one key-value pair"
            );
        }

        #[test]
        fn multiple() {
            let mut match_labels: BTreeMap<String, String> = BTreeMap::new();
            match_labels.insert("key1".to_string(), "value1".to_string());
            match_labels.insert("key2".to_string(), "value2".to_string());
            let result = match_labels_to_query(match_labels);
            assert_eq!(
                result,
                vec!["key1=value1", "key2=value2"],
                "Result should contain two key-value pairs"
            );
        }
    }

    mod match_expressions_to_query {
        use super::*;

        #[test]
        fn empty() {
            let match_expressions: Vec<LabelSelectorRequirement> = Vec::new();
            let result = match_expressions_to_query(match_expressions);
            assert!(
                result.is_empty(),
                "Result should be empty for an empty input"
            );
        }

        #[test]
        fn in_operator() {
            let match_expressions: Vec<LabelSelectorRequirement> = vec![LabelSelectorRequirement {
                key: "key".to_string(),
                operator: "In".to_string(),
                values: Some(vec!["value1".to_string(), "value2".to_string()]),
            }];
            let result = match_expressions_to_query(match_expressions);
            assert_eq!(
                result,
                vec!["key in (value1, value2)"],
                "Result should contain one 'In' expression"
            );
        }

        #[test]
        fn not_in_operator() {
            let match_expressions: Vec<LabelSelectorRequirement> = vec![LabelSelectorRequirement {
                key: "key".to_string(),
                operator: "NotIn".to_string(),
                values: Some(vec!["value1".to_string(), "value2".to_string()]),
            }];
            let result = match_expressions_to_query(match_expressions);
            assert_eq!(
                result,
                vec!["key notin (value1, value2)"],
                "Result should contain one 'NotIn' expression"
            );
        }

        #[test]
        fn exists_operator() {
            let match_expressions: Vec<LabelSelectorRequirement> = vec![LabelSelectorRequirement {
                key: "key".to_string(),
                operator: "Exists".to_string(),
                values: None,
            }];
            let result = match_expressions_to_query(match_expressions);
            assert_eq!(
                result,
                vec!["key"],
                "Result should contain one 'Exists' expression"
            );
        }

        #[test]
        fn does_not_exist_operator() {
            let match_expressions: Vec<LabelSelectorRequirement> = vec![LabelSelectorRequirement {
                key: "key".to_string(),
                operator: "DoesNotExist".to_string(),
                values: None,
            }];
            let result = match_expressions_to_query(match_expressions);
            assert_eq!(
                result,
                vec!["!key"],
                "Result should contain one 'DoesNotExist' expression"
            );
        }
    }
}
