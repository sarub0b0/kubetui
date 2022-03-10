use k8s_openapi::{api::core::v1::Service, List};
use serde_yaml::Value;

pub type FetchedServiceList = List<Service>;

pub struct FetchedService(pub Vec<Service>);

impl FetchedService {
    pub fn to_value(&self) -> Option<Value> {
        let ret: Vec<Value> = self
            .0
            .iter()
            .cloned()
            .filter_map(|svc| svc.metadata.name)
            .map(Value::String)
            .collect();

        if !ret.is_empty() {
            Some(ret.into())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
    use pretty_assertions::assert_eq;

    #[test]
    fn 複数のserviceを持つときリソース名を含むvalueを返す() {
        let actual = FetchedService(vec![
            Service {
                metadata: ObjectMeta {
                    name: Some("foo".into()),
                    ..Default::default()
                },
                ..Default::default()
            },
            Service {
                metadata: ObjectMeta {
                    name: Some("bar".into()),
                    ..Default::default()
                },
                ..Default::default()
            },
        ])
        .to_value();

        let expected: Vec<Value> = vec!["foo".into(), "bar".into()];

        assert_eq!(actual, Some(expected.into()));
    }

    #[test]
    fn serviceを持たないときnoneを返す() {
        let actual = FetchedService(vec![]).to_value();

        let expected = None;

        assert_eq!(actual, expected);
    }
}
