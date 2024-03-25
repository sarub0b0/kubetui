pub mod metrics;
pub mod networking;
pub mod v1_table;

pub use k8s_openapi::{
    apimachinery, merge_strategies, serde, ClusterResourceScope, DeepMerge, ListableResource,
    Metadata, NamespaceResourceScope, Resource,
};
