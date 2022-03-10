use k8s_openapi::{api::networking::v1::Ingress, List};
use serde_yaml::{Mapping, Value};

pub type FetchedIngressList = List<Ingress>;

pub struct FetchedIngress(pub Vec<Ingress>);

impl FetchedIngress {
    pub fn to_value(&self) -> Option<Value> {
        let ret: Vec<Value> = self
            .0
            .iter()
            .cloned()
            .filter_map(|ing| ing.metadata.name)
            .map(Value::String)
            .collect();

        if !ret.is_empty() {
            let mut map = Mapping::new();
            map.insert("ingresses".into(), ret.into());
            Some(map.into())
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
    fn 複数のingressを持つときリソース名を含むvalueを返す() {
        let actual = FetchedIngress(vec![
            Ingress {
                metadata: ObjectMeta {
                    name: Some("foo".into()),
                    ..Default::default()
                },
                ..Default::default()
            },
            Ingress {
                metadata: ObjectMeta {
                    name: Some("bar".into()),
                    ..Default::default()
                },
                ..Default::default()
            },
        ])
        .to_value();

        let mut expected = Mapping::new();

        let ingresses: Vec<Value> = vec!["foo".into(), "bar".into()];

        expected.insert("ingresses".into(), ingresses.into());

        assert_eq!(actual, Some(expected.into()));
    }

    #[test]
    fn ingressを持たないときnoneを返す() {
        let actual = FetchedIngress(vec![]).to_value();

        let expected = None;

        assert_eq!(actual, expected);
    }
}
