use k8s_openapi::api::core::v1::Namespace;

use kube::{
    api::{ListParams, ResourceExt},
    config::Kubeconfig,
    Api, Client,
};

pub async fn namespace_list(client: Client) -> Vec<String> {
    let namespaces: Api<Namespace> = Api::all(client);
    let lp = ListParams::default();
    let ns_list = namespaces.list(&lp).await.unwrap();

    ns_list.iter().map(|ns| ns.name()).collect()
}

pub fn context_list(config: &Kubeconfig) -> Vec<String> {
    config
        .contexts
        .iter()
        .cloned()
        .map(|ctx| ctx.name)
        .collect()
}
