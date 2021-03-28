use super::Event;
use super::Kube;
use crate::kubernetes::config::{configs_loop, get_config};
use crate::kubernetes::log::log_stream;
use crate::kubernetes::pod::pod_loop;

use std::sync::{Arc, RwLock};

use crossbeam::channel::{Receiver, Sender};
use tokio::{
    runtime::Runtime,
    task::{self, JoinHandle},
};

use k8s_openapi::api::core::v1::Namespace;

use kube::{
    api::{ListParams, Meta},
    config::{KubeConfigOptions, Kubeconfig},
    Api, Client,
};

fn context_list(tx: Sender<Event>) {
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
fn change_context() {}

// fn get_kubeconfig_option(context: String) -> Option<KubeConfigOptions> {
//     let kubeconfig = Kubeconfig::read().unwrap();

//     let ret = kubeconfig
//         .contexts
//         .iter()
//         .cloned()
//         .find(|c| c.name == context);

//     if let Some(k) = ret {
//         Some(KubeConfigOptions {
//             context: Some(k.name),
//             cluster: Some(k.context.cluster),
//             user: Some(k.context.user),
//         })
//     } else {
//         None
//     }
// }
