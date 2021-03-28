use super::Event;
use super::Kube;

use crossbeam::channel::Sender;

use k8s_openapi::api::core::v1::Namespace;

use kube::{
    api::{ListParams, Meta},
    config::{KubeConfigOptions, Kubeconfig},
    Api, Client,
};

fn _context_list(tx: Sender<Event>) {
    let kubeconfig = Kubeconfig::read().unwrap();

    let ret = kubeconfig
        .contexts
        .iter()
        .cloned()
        .map(|c| c.name)
        .collect();

    tx.send(Event::Kube(Kube::GetContextsResponse(ret)))
        .unwrap();
}
fn _change_context() {}

pub async fn namespace_list(client: Client) -> Vec<String> {
    let namespaces: Api<Namespace> = Api::all(client);
    let lp = ListParams::default();
    let ns_list = namespaces.list(&lp).await.unwrap();

    ns_list.iter().map(|ns| ns.name()).collect()
}

fn _get_kubeconfig_option(context: String) -> Option<KubeConfigOptions> {
    let kubeconfig = Kubeconfig::read().unwrap();

    let ret = kubeconfig
        .contexts
        .iter()
        .cloned()
        .find(|c| c.name == context);

    if let Some(k) = ret {
        Some(KubeConfigOptions {
            context: Some(k.name),
            cluster: Some(k.context.cluster),
            user: Some(k.context.user),
        })
    } else {
        None
    }
}
