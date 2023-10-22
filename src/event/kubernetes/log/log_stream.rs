use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, FixedOffset, Utc};
use futures::{AsyncBufReadExt, TryStreamExt};
use k8s_openapi::api::core::v1::Pod;
use kube::{api::LogParams, Api};
use tokio::time;

use crate::{
    event::kubernetes::{client::KubeClient, worker::Worker},
    logger,
};

use super::{log_collector::LogBuffer, Color};

pub const COLOR_LIST: [Color; 6] = [
    Color::Green,
    Color::Yellow,
    Color::Blue,
    Color::Magenta,
    Color::Cyan,
    Color::Gray,
];

#[derive(Clone)]
pub struct ContainerLogStreamerOptions {
    pub color: Color,
}

#[derive(Clone)]
pub struct ContainerLogStreamer {
    client: KubeClient,
    namespace: String,
    pod_name: String,
    container_name: String,
    log_buffer: LogBuffer,
    options: ContainerLogStreamerOptions,
}

#[async_trait]
impl Worker for ContainerLogStreamer {
    type Output = ();
    async fn run(&self) -> Self::Output {
        let mut interval = tokio::time::interval(time::Duration::from_secs(5));

        let mut last_timestamp: Option<DateTime<FixedOffset>> = None;

        let prefix = self.log_prefix();

        self.send_started_message().await;

        loop {
            interval.tick().await;

            let result = self.fetch(&prefix, &mut last_timestamp).await;

            if let Err(err) = result {
                logger!(error, "{}", err)
            } else {
                break;
            }
        }

        self.send_finished_message().await;
    }
}

impl ContainerLogStreamer {
    pub fn new(
        client: KubeClient,
        namespace: String,
        pod_name: String,
        container_name: String,
        log_buffer: LogBuffer,
        options: ContainerLogStreamerOptions,
    ) -> Self {
        Self {
            client,
            namespace,
            pod_name,
            container_name,
            log_buffer,
            options,
        }
    }

    async fn send_started_message(&self) {
        let mut buf = self.log_buffer.write().await;

        let sign = Color::LightGreen.wrap("+");
        let container_name = self.log_prefix_color().wrap(&self.container_name);

        buf.push(format!("{} {}", sign, container_name));
    }

    async fn send_finished_message(&self) {
        let mut buf = self.log_buffer.write().await;
        let sign = Color::LightRed.wrap("-");
        let container_name = self.log_prefix_color().wrap(&self.container_name);

        buf.push(format!("{} {}", sign, container_name));
    }

    async fn fetch(
        &self,
        prefix: &str,
        last_timestamp: &mut Option<DateTime<FixedOffset>>,
    ) -> Result<()> {
        let log_params = self.log_params(last_timestamp);

        let api: Api<Pod> = Api::namespaced(self.client.to_client(), &self.namespace);

        let mut logs = api.log_stream(&self.pod_name, &log_params).await?.lines();

        while let Some(line) = logs.try_next().await? {
            let mut buf = self.log_buffer.write().await;

            if let Ok((dt, content)) = chrono::DateTime::parse_and_remainder(&line, "%+ ") {
                buf.push(format!("{}{}", prefix, content));

                *last_timestamp = Some(dt);
            } else {
                buf.push(format!("{}{}", prefix, line));
            }
        }

        Ok(())
    }

    fn log_params(&self, last_timestamp: &Option<DateTime<FixedOffset>>) -> LogParams {
        LogParams {
            follow: true,
            container: Some(self.container_name.to_string()),
            timestamps: true,
            since_seconds: Self::since_seconds(Utc::now().fixed_offset(), last_timestamp),
            ..Default::default()
        }
    }

    fn log_prefix(&self) -> String {
        format!(
            "{} ",
            self.log_prefix_color()
                .wrap(format!("[{}]", self.container_name))
        )
    }

    fn log_prefix_color(&self) -> Color {
        self.options.color
    }

    fn since_seconds(
        now: DateTime<FixedOffset>,
        last_timestamp: &Option<DateTime<FixedOffset>>,
    ) -> Option<i64> {
        last_timestamp
            .map(|last| (now - last).num_seconds())
            .filter(|time| *time > 0)
    }
}
