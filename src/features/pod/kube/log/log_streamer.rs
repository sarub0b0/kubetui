use std::{
    collections::hash_map::DefaultHasher,
    hash::Hasher,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::{AsyncBufReadExt, TryStreamExt};
use k8s_openapi::api::core::v1::Pod;
use kube::{api::LogParams, Api};
use regex::Regex;
use tokio::time;

use crate::{
    kube::KubeClient,
    logger,
    workers::kube::{color::fg::Color, AbortWorker},
};

use super::{log_collector::LogBuffer, log_content::LogContent};

#[derive(Debug, Clone, Copy)]
pub enum LogPrefixType {
    OnlyContainer,
    PodAndContainer,
    All,
}

impl Default for LogPrefixType {
    fn default() -> Self {
        Self::PodAndContainer
    }
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

#[derive(Default, Clone)]
pub struct LogStreamerOptions {
    pub prefix_type: LogPrefixType,
    pub include_log: Option<Vec<Regex>>,
    pub exclude_log: Option<Vec<Regex>>,
}

#[derive(Clone)]
pub struct LogStreamerTarget {
    pub namespace: String,
    pub pod_name: String,
    pub container_name: String,
}

#[derive(Clone)]
pub struct LogStreamer {
    client: KubeClient,
    log_buffer: LogBuffer,
    is_terminated: Arc<AtomicBool>,
    target: LogStreamerTarget,
    options: LogStreamerOptions,
}

#[async_trait]
impl AbortWorker for LogStreamer {
    async fn run(&self) {
        let mut interval = tokio::time::interval(time::Duration::from_secs(3));

        let mut last_timestamp: Option<DateTime<Utc>> = None;

        let prefix = self.log_prefix();

        self.send_started_message().await;

        loop {
            interval.tick().await;

            let result = self.fetch(&prefix, &mut last_timestamp).await;

            if let Err(err) = result {
                logger!(error, "{}", err)
            } else if self.is_terminated.load(Ordering::Relaxed) {
                // 正常終了は下記2パターン確認しているため、
                // コンテナ終了時のみループを抜ける処理を組み込む。
                //   - コンテナが終了している
                //   - 長時間実行
                break;
            }
        }

        self.send_finished_message().await;
    }
}

impl LogStreamer {
    pub fn new(
        client: KubeClient,
        log_buffer: LogBuffer,
        is_terminated: Arc<AtomicBool>,
        target: LogStreamerTarget,
    ) -> Self {
        Self {
            client,
            log_buffer,
            is_terminated,
            target,
            options: LogStreamerOptions::default(),
        }
    }

    pub fn options(mut self, options: LogStreamerOptions) -> Self {
        self.options = options;
        self
    }

    async fn fetch(&self, prefix: &str, last_timestamp: &mut Option<DateTime<Utc>>) -> Result<()> {
        let log_params = self.log_params(last_timestamp);

        let api: Api<Pod> = Api::namespaced(self.client.to_client(), self.namespace());

        let mut logs = api.log_stream(self.pod_name(), &log_params).await?.lines();

        while let Some(line) = logs.try_next().await? {
            let mut buf = self.log_buffer.lock().await;

            if let Ok((dt, content)) = chrono::DateTime::parse_and_remainder(&line, "%+") {
                let dt: DateTime<Utc> = dt.into();

                if last_timestamp.is_some_and(|lts| dt <= lts) {
                    continue;
                }

                let content = content.strip_prefix(' ').unwrap_or(content);

                if self.is_exclude(content) || !self.is_include(content) {
                    continue;
                }

                buf.push(LogContent {
                    prefix: prefix.to_string(),
                    content: content.to_string(),
                });

                *last_timestamp = Some(dt);
            } else {
                if self.is_exclude(&line) || !self.is_include(&line) {
                    continue;
                }

                buf.push(LogContent {
                    prefix: prefix.to_string(),
                    content: line.to_string(),
                });
            }
        }

        Ok(())
    }

    fn is_exclude(&self, s: &str) -> bool {
        self.options
            .exclude_log
            .as_ref()
            .is_some_and(|exclude| exclude.iter().any(|re| re.is_match(s)))
    }

    fn is_include(&self, s: &str) -> bool {
        let Some(include) = &self.options.include_log else {
            return true;
        };

        include.iter().any(|include| include.is_match(s))
    }

    async fn send_started_message(&self) {
        let sign = Color::LightGreen.wrap("+");

        let mut buf = self.log_buffer.lock().await;

        buf.push(LogContent {
            prefix: sign,
            content: self.log_prefix_content(),
        });
    }

    async fn send_finished_message(&self) {
        let sign = Color::LightRed.wrap("-");

        let mut buf = self.log_buffer.lock().await;

        buf.push(LogContent {
            prefix: sign,
            content: self.log_prefix_content(),
        });
    }

    fn log_prefix_content(&self) -> String {
        use LogPrefixType::*;

        let prefix_color = self.log_prefix_color();

        match self.options.prefix_type {
            OnlyContainer => prefix_color.container.wrap(self.container_name()),
            PodAndContainer => {
                let container_name = prefix_color.container.wrap(self.container_name());
                let pod_name = prefix_color.pod.wrap(self.pod_name());

                prefix_color
                    .pod
                    .wrap(format!("{} {}", pod_name, container_name))
            }
            All => {
                let container_name = prefix_color.container.wrap(self.container_name());
                let pod_name = prefix_color.pod.wrap(self.pod_name());

                prefix_color.pod.wrap(format!(
                    "{} {} {}",
                    self.namespace(),
                    pod_name,
                    container_name
                ))
            }
        }
    }

    fn log_prefix(&self) -> String {
        use LogPrefixType::*;

        let prefix_color = self.log_prefix_color();
        match self.options.prefix_type {
            OnlyContainer => {
                let open_bracket = prefix_color.container.wrap("[");
                let close_bracket = prefix_color.container.wrap("]");

                format!(
                    "{}{}{}",
                    open_bracket,
                    self.log_prefix_content(),
                    close_bracket
                )
            }
            PodAndContainer | All => {
                let open_bracket = prefix_color.pod.wrap("[");
                let close_bracket = prefix_color.pod.wrap("]");
                format!(
                    "{}{}{}",
                    open_bracket,
                    self.log_prefix_content(),
                    close_bracket
                )
            }
        }
    }

    fn log_prefix_color(&self) -> PrefixColor {
        use LogPrefixType::*;

        let index = match self.options.prefix_type {
            OnlyContainer => {
                let mut hash = DefaultHasher::new();
                hash.write(self.container_name().as_bytes());
                hash.write_u8(0xff);

                hash.finish() as usize
            }
            PodAndContainer | All => {
                let mut hash = DefaultHasher::new();
                hash.write(self.pod_name().as_bytes());
                hash.write_u8(0xff);

                hash.finish() as usize
            }
        };

        PREFIX_COLOR_LIST[index % PREFIX_COLOR_LIST.len()]
    }

    fn log_params(&self, last_timestamp: &Option<DateTime<Utc>>) -> LogParams {
        LogParams {
            follow: true,
            container: Some(self.container_name().to_string()),
            timestamps: true,
            since_time: *last_timestamp,
            ..Default::default()
        }
    }

    fn namespace(&self) -> &str {
        &self.target.namespace
    }

    fn pod_name(&self) -> &str {
        &self.target.pod_name
    }

    fn container_name(&self) -> &str {
        &self.target.container_name
    }
}
