use k8s_openapi::{
    api::core::v1::{Service, ServiceSpec, ServiceStatus},
    List,
};

use super::*;

pub type FetchedServiceList = List<Service>;

pub struct FetchedService(pub Service);

impl FetchedService {
    pub fn to_string_vec(&self) -> Vec<String> {
        let mut ret = vec!["Service:".to_string()];
        ret
    }
}

