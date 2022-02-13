use crossbeam::channel::Sender;

use crate::{
    error::Result,
    event::{kubernetes::client::KubeClient, Event},
};

use super::DescriptionWorker;

pub(super) struct ServiceDescriptionWorker<'a> {
    client: &'a KubeClient,
    tx: &'a Sender<Event>,
    namespace: String,
    name: String,
    url: String,
}

#[async_trait::async_trait]
impl<'a> DescriptionWorker<'a> for ServiceDescriptionWorker<'a> {
    fn new(_: &'a KubeClient, _: &'a Sender<Event>, _: String, _: String) -> Self {
        todo!()
    }

    async fn run(&self) -> Result<()> {
        todo!()
    }
}
