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
}
