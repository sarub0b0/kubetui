use std::{collections::hash_map::DefaultHasher, hash::Hasher};

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, FixedOffset, Utc};
use futures::{AsyncBufReadExt, TryStreamExt};
use k8s_openapi::api::core::v1::Pod;
use kube::{api::LogParams, Api};
use tokio::time;

use crate::{
    event::kubernetes::{client::KubeClient, color::fg::Color, worker::AbortWorker},
    logger,
};

use super::collector::LogBuffer;

#[derive(Debug, Clone, Copy)]
pub enum LogStreamPrefixType {
    OnlyContainer,
    PodAndContainer,
    All,
}

#[derive(Clone, Copy)]
struct PrefixColor {
    pub pod: Color,
    pub container: Color,
}

const PREFIX_COLOR_LIST: [PrefixColor; 6] = [
    PrefixColor {
        pod: Color::LightGreen,
        container: Color::Green,
    },
    PrefixColor {
        pod: Color::LightYellow,
        container: Color::Yellow,
    },
    PrefixColor {
        pod: Color::LightBlue,
        container: Color::Blue,
    },
    PrefixColor {
        pod: Color::LightMagenta,
        container: Color::Magenta,
    },
    PrefixColor {
        pod: Color::LightCyan,
        container: Color::Cyan,
    },
    PrefixColor {
        pod: Color::White,
        container: Color::Gray,
    },
];

#[derive(Clone)]
pub struct ContainerLogStreamerOptions {
    pub prefix_type: LogStreamPrefixType,
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
impl AbortWorker for ContainerLogStreamer {
    async fn run(&self) {
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
                // TODO: 長時間実行でここに到達しないことを確認する
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

    async fn fetch(
        &self,
        prefix: &str,
        last_timestamp: &mut Option<DateTime<FixedOffset>>,
    ) -> Result<()> {
        let log_params = self.log_params(last_timestamp);

        let api: Api<Pod> = Api::namespaced(self.client.to_client(), &self.namespace);

        let mut logs = api.log_stream(&self.pod_name, &log_params).await?.lines();

        while let Some(line) = logs.try_next().await? {
            let mut buf = self.log_buffer.lock().await;

            if let Ok((dt, content)) = chrono::DateTime::parse_and_remainder(&line, "%+ ") {
                buf.push(format!("{}{}", prefix, content));

                *last_timestamp = Some(dt);
            } else {
                buf.push(format!("{}{}", prefix, line));
            }
        }

        Ok(())
    }

    async fn send_started_message(&self) {
        let sign = Color::LightGreen.wrap("+");

        let mut buf = self.log_buffer.lock().await;

        buf.push(format!("{} {}", sign, self.log_prefix_content()));
    }

    async fn send_finished_message(&self) {
        let sign = Color::LightRed.wrap("-");

        let mut buf = self.log_buffer.lock().await;

        buf.push(format!("{} {}", sign, self.log_prefix_content()));
    }

    fn log_prefix_content(&self) -> String {
        use LogStreamPrefixType::*;

        let prefix_color = self.log_prefix_color();

        match self.options.prefix_type {
            OnlyContainer => prefix_color.container.wrap(&self.container_name),
            PodAndContainer => {
                let container_name = prefix_color.container.wrap(&self.container_name);
                let pod_name = prefix_color.pod.wrap(&self.pod_name);

                prefix_color
                    .pod
                    .wrap(format!("{} {}", pod_name, container_name))
            }
            All => {
                let container_name = prefix_color.container.wrap(&self.container_name);
                let pod_name = prefix_color.pod.wrap(&self.pod_name);

                prefix_color.pod.wrap(format!(
                    "{} {} {}",
                    self.namespace, pod_name, container_name
                ))
            }
        }
    }

    fn log_prefix(&self) -> String {
        use LogStreamPrefixType::*;

        let prefix_color = self.log_prefix_color();
        match self.options.prefix_type {
            OnlyContainer => {
                let open_bracket = prefix_color.container.wrap("[");
                let close_bracket = prefix_color.container.wrap("]");

                format!(
                    "{}{}{} ",
                    open_bracket,
                    self.log_prefix_content(),
                    close_bracket
                )
            }
            PodAndContainer | All => {
                let open_bracket = prefix_color.pod.wrap("[");
                let close_bracket = prefix_color.pod.wrap("]");
                format!(
                    "{}{}{} ",
                    open_bracket,
                    self.log_prefix_content(),
                    close_bracket
                )
            }
        }
    }

    fn log_prefix_color(&self) -> PrefixColor {
        use LogStreamPrefixType::*;

        let index = match self.options.prefix_type {
            OnlyContainer => {
                let mut hash = DefaultHasher::new();
                hash.write(self.container_name.as_bytes());
                hash.write_u8(0xff);

                hash.finish() as usize
            }
            PodAndContainer | All => {
                let mut hash = DefaultHasher::new();
                hash.write(self.pod_name.as_bytes());
                hash.write_u8(0xff);

                hash.finish() as usize
            }
        };

        PREFIX_COLOR_LIST[index % PREFIX_COLOR_LIST.len()]
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

    fn since_seconds(
        now: DateTime<FixedOffset>,
        last_timestamp: &Option<DateTime<FixedOffset>>,
    ) -> Option<i64> {
        last_timestamp
            .map(|last| (now - last).num_seconds())
            .filter(|time| *time > 0)
    }
}
