pub mod resources;
pub mod worker;

use crate::event::kubernetes::api_resources::ApiResource;

#[derive(Debug, Clone)]
pub struct SelectedYaml {
    pub kind: ApiResource,
    pub name: String,
    pub namespace: String,
}
