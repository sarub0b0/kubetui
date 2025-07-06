use std::time;

use anyhow::Result;
use async_trait::async_trait;
use crossbeam::channel::Sender;
use futures::future::try_join_all;
use ratatui::style::{Color, Style};

use crate::{
    kube::{
        KubeClient,
        apis::v1_table::{TableRow, ToTime as _},
        table::{KubeTableRow, get_resource_per_namespace, insert_ns},
    },
    message::Message,
    ui::widget::ansi_color::style_to_ansi,
    workers::kube::{SharedTargetNamespaces, Worker, WorkerResult, message::Kube},
};

#[derive(Default, Debug, Clone)]
pub struct EventConfig {
    pub highlight_rules: Vec<EventHighlightRule>,
}

impl EventConfig {
    fn get_style(&self, ty: &str) -> (Style, Style) {
        self.highlight_rules
            .iter()
            .find(|rule| rule.ty.is_match(ty))
            .map(|rule| (rule.summary, rule.message))
            .unwrap_or_else(|| (Style::default(), Style::default().fg(Color::DarkGray)))
    }
}

#[derive(Debug, Clone)]
pub struct EventHighlightRule {
    pub ty: regex::Regex,
    pub summary: Style,
    pub message: Style,
}

#[derive(Clone)]
pub struct EventPoller {
    tx: Sender<Message>,
    shared_target_namespaces: SharedTargetNamespaces,
    kube_client: KubeClient,
    config: EventConfig,
}

impl EventPoller {
    pub fn new(
        tx: Sender<Message>,
        shared_target_namespaces: SharedTargetNamespaces,
        kube_client: KubeClient,
        config: EventConfig,
    ) -> Self {
        Self {
            tx,
            shared_target_namespaces,
            kube_client,
            config,
        }
    }
}

#[async_trait]
impl Worker for EventPoller {
    type Output = WorkerResult;
    async fn run(&self) -> Self::Output {
        let Self {
            tx,
            shared_target_namespaces,
            kube_client,
            config,
        } = self;

        let mut interval = tokio::time::interval(time::Duration::from_millis(1000));

        loop {
            interval.tick().await;
            let target_namespaces = shared_target_namespaces.read().await;

            let event_list = get_event_table(config, kube_client, &target_namespaces).await;

            tx.send(Message::Kube(Kube::Event(event_list)))
                .expect("Failed to send Kube::Event");
        }
    }
}

struct Event {
    last_seen: String,
    ty: String,
    object: String,
    reason: String,
    message: String,
    namespace: Option<String>,
}

const TARGET_LEN: usize = 5;
const TARGET: [&str; TARGET_LEN] = ["Last Seen", "Type", "Object", "Reason", "Message"];

async fn get_event_per_namespace(
    client: &KubeClient,
    namespace: &str,
    insert_ns: bool,
) -> Result<Vec<Event>> {
    let tables = get_resource_per_namespace(
        client,
        format!("api/v1/namespaces/{}/{}", namespace, "events"),
        &TARGET,
        move |row: &TableRow, indexes: &[usize]| {
            let row: Vec<String> = indexes.iter().map(|i| row.cells[*i].to_string()).collect();

            KubeTableRow {
                namespace: namespace.to_string(),
                row,
                ..Default::default()
            }
        },
    )
    .await?;

    let ret = tables
        .into_iter()
        .map(|table| Event {
            last_seen: table.row[0].clone(),
            ty: table.row[1].clone(),
            object: table.row[2].clone(),
            reason: table.row[3].clone(),
            message: table.row[4].clone(),
            namespace: if insert_ns {
                Some(namespace.to_string())
            } else {
                None
            },
        })
        .collect();

    Ok(ret)
}

async fn get_event_table(
    config: &EventConfig,
    client: &KubeClient,
    namespaces: &[String],
) -> Result<Vec<String>> {
    let insert_ns = insert_ns(namespaces);

    let jobs = try_join_all(
        namespaces
            .iter()
            .map(|ns| get_event_per_namespace(client, ns, insert_ns)),
    )
    .await?;

    let mut ok_only: Vec<Event> = jobs.into_iter().flatten().collect();

    ok_only.sort_by_key(|ev| ev.last_seen.to_time());

    Ok(ok_only
        .iter()
        .flat_map(|ev| {
            let (summary_style, message_style) = config.get_style(&ev.ty);

            let mut summary = style_to_ansi(summary_style);

            summary.push_str(&format!("{:<4}  {:<4}", ev.last_seen, ev.ty));

            if let Some(ns) = &ev.namespace {
                summary.push_str(&format!("  {ns:<4}"));
            }

            summary.push_str(&format!("  {:<4}  {:<4}", ev.object, ev.reason));

            let mut message: Vec<String> = ev
                .message
                .lines()
                .map(|line| format!("{}> {}", style_to_ansi(message_style), line))
                .collect();

            message.push("\x1b[0m".to_string());

            [summary]
                .into_iter()
                .chain(message.into_iter())
                .collect::<Vec<_>>()
        })
        .collect())
}
