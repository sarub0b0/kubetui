use crate::features::api_resources::kube::ApiResource;

pub mod resources;
pub mod worker;

#[derive(Debug, Clone)]
pub struct SelectedYaml {
    pub kind: ApiResource,
    pub name: String,
    pub namespace: String,
}
