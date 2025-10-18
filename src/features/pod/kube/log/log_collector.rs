use std::sync::Arc;

use async_trait::async_trait;
use crossbeam::channel::Sender;
use jaq_core::{Ctx, RcIter};
use jaq_json::Val;
use tokio::{sync::Mutex, time};

use crate::{
    features::pod::kube::filter::JsonFilter, message::Message, send_response, workers::kube::Worker,
};

use super::log_content::LogContent;

pub type LogBuffer = Arc<Mutex<Vec<LogContent>>>;

#[derive(Clone)]
pub struct LogCollector {
    tx: Sender<Message>,
    buffer: LogBuffer,
    json_pretty_print: bool,
    /// JSONログに適用するフィルター（jq、JMESPathなど）
    json_filter: Option<JsonFilter>,
}

impl LogCollector {
    pub fn new(
        tx: Sender<Message>,
        buffer: LogBuffer,
        json_pretty_print: bool,
        json_filter: Option<JsonFilter>,
    ) -> Self {
        Self {
            tx,
            buffer,
            json_pretty_print,
            json_filter,
        }
    }

    fn render_content(&self, content: LogContent) -> Vec<String> {
        if !self.json_pretty_print && self.json_filter.is_none() {
            return vec![print_content(&content.prefix, &content.content)];
        }

        let Ok(json) = serde_json::from_str::<serde_json::Value>(&content.content) else {
            return vec![print_content(&content.prefix, &content.content)];
        };

        let prefix = content.prefix.clone();
        let original_content = content.content.clone();

        if let Some(JsonFilter::Jq(jq)) = &self.json_filter {
            let inputs = RcIter::new(core::iter::empty());

            jq.program
                .run((Ctx::new([], &inputs), Val::from(json)))
                .flat_map(|v| match v {
                    Ok(v) => {
                        let json_value: serde_json::Value = v.into();
                        self.print_json(&prefix, &json_value)
                    }

                    Err(e) => vec![
                        print_content(&prefix, format!("jq evaluation error: {}", e)),
                        print_content(&prefix, "(showing original log below)"),
                        print_content(&prefix, &original_content),
                    ],
                })
                .collect::<Vec<_>>()
        } else {
            self.print_json(&prefix, &json)
        }
    }

    fn print_json(&self, prefix: &str, json: &serde_json::Value) -> Vec<String> {
        if self.json_pretty_print {
            pretty_print_json(prefix, json)
        } else {
            vec![print_content(prefix, json)]
        }
    }
}

fn print_content(prefix: &str, content: impl std::fmt::Display) -> String {
    format!("{}  {}", prefix, content)
}

fn pretty_print_json(prefix: &str, json: &serde_json::Value) -> Vec<String> {
    format!("{:#}", json)
        .lines()
        .map(|line| print_content(prefix, line))
        .collect()
}

/// 将来的にはチャネルにしたい
#[async_trait]
impl Worker for LogCollector {
    type Output = ();
    async fn run(&self) -> Self::Output {
        let mut interval = tokio::time::interval(time::Duration::from_millis(200));

        loop {
            interval.tick().await;

            let mut buf = self.buffer.lock().await;

            let contents = std::mem::take(&mut *buf);

            if contents.is_empty() {
                continue;
            }

            let logs: Vec<String> = contents
                .into_iter()
                .flat_map(|content| self.render_content(content))
                .collect();

            if logs.is_empty() {
                continue;
            }

            send_response!(self.tx, Ok(logs));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::pod::kube::filter::Filter;
    use crossbeam::channel;

    #[test]
    fn test_render_content_with_jq_filter_field_extraction() {
        // jqフィルターでフィールド抽出
        let (tx, _rx) = channel::unbounded();
        let buffer = Arc::new(Mutex::new(Vec::new()));

        let filter = Filter::parse("jq:.message").unwrap();
        let collector = LogCollector::new(tx, buffer, false, filter.json_filter);

        let content = LogContent {
            prefix: "[pod]".to_string(),
            content: r#"{"level":"error","message":"test error"}"#.to_string(),
        };

        let result = collector.render_content(content);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], r#"[pod]  "test error""#);
    }

    #[test]
    fn test_render_content_with_jq_filter_restructure() {
        // jqフィルターでデータ構造の再構築
        let (tx, _rx) = channel::unbounded();
        let buffer = Arc::new(Mutex::new(Vec::new()));

        let filter = Filter::parse("jq:{lvl:.level,msg:.message}").unwrap();
        let collector = LogCollector::new(tx, buffer, false, filter.json_filter);

        let content = LogContent {
            prefix: "[test]".to_string(),
            content: r#"{"level":"info","message":"hello","timestamp":"2024-01-01"}"#.to_string(),
        };

        let result = collector.render_content(content);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], r#"[test]  {"lvl":"info","msg":"hello"}"#);
    }

    #[test]
    fn test_render_content_non_json_with_jq_filter() {
        // 非JSON入力の場合は生ログを返す
        let (tx, _rx) = channel::unbounded();
        let buffer = Arc::new(Mutex::new(Vec::new()));

        let filter = Filter::parse("jq:.message").unwrap();
        let collector = LogCollector::new(tx, buffer, false, filter.json_filter);

        let content = LogContent {
            prefix: "[pod]".to_string(),
            content: "plain text log".to_string(),
        };

        let result = collector.render_content(content);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "[pod]  plain text log");
    }

    #[test]
    fn test_render_content_without_jq_filter() {
        // jqフィルターなしの場合
        let (tx, _rx) = channel::unbounded();
        let buffer = Arc::new(Mutex::new(Vec::new()));

        let collector = LogCollector::new(tx, buffer, false, None);

        let content = LogContent {
            prefix: "[pod]".to_string(),
            content: r#"{"level":"error","message":"test"}"#.to_string(),
        };

        let result = collector.render_content(content);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], r#"[pod]  {"level":"error","message":"test"}"#);
    }

    #[test]
    fn test_render_content_with_pretty_print_and_jq() {
        // pretty printとjqフィルターの組み合わせ
        let (tx, _rx) = channel::unbounded();
        let buffer = Arc::new(Mutex::new(Vec::new()));

        let filter = Filter::parse("jq:{level:.level,message:.message}").unwrap();
        let collector = LogCollector::new(tx, buffer, true, filter.json_filter);

        let content = LogContent {
            prefix: "[test]".to_string(),
            content: r#"{"level":"warn","message":"warning message","extra":"data"}"#.to_string(),
        };

        let result = collector.render_content(content);

        let expected = indoc::indoc! {r#"
            [test]  {
            [test]    "level": "warn",
            [test]    "message": "warning message"
            [test]  }
        "#}
        .trim();

        assert_eq!(result.join("\n"), expected);
    }

    #[test]
    fn test_render_content_with_pretty_print_no_jq() {
        // pretty printのみ（jqフィルターなし）
        let (tx, _rx) = channel::unbounded();
        let buffer = Arc::new(Mutex::new(Vec::new()));

        let collector = LogCollector::new(tx, buffer, true, None);

        let content = LogContent {
            prefix: "[pod]".to_string(),
            content: r#"{"level":"info","message":"test message"}"#.to_string(),
        };

        let result = collector.render_content(content);

        let expected = indoc::indoc! {r#"
            [pod]  {
            [pod]    "level": "info",
            [pod]    "message": "test message"
            [pod]  }
        "#}
        .trim();

        assert_eq!(result.join("\n"), expected);
    }

    #[test]
    fn test_render_content_jq_returns_multiple_values() {
        // jqが複数の値を返す場合
        let (tx, _rx) = channel::unbounded();
        let buffer = Arc::new(Mutex::new(Vec::new()));

        // .[] は配列の各要素を返す
        let filter = Filter::parse("jq:.items[]").unwrap();
        let collector = LogCollector::new(tx, buffer, false, filter.json_filter);

        let content = LogContent {
            prefix: "[test]".to_string(),
            content: r#"{"items":["a","b","c"]}"#.to_string(),
        };

        let result = collector.render_content(content);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], r#"[test]  "a""#);
        assert_eq!(result[1], r#"[test]  "b""#);
        assert_eq!(result[2], r#"[test]  "c""#);
    }

    #[test]
    fn test_render_content_jq_error_shows_original_log() {
        // jqエラー時は元のログを表示
        let (tx, _rx) = channel::unbounded();
        let buffer = Arc::new(Mutex::new(Vec::new()));

        // 配列にアクセスしようとするが、実際はオブジェクト
        let filter = Filter::parse("jq:.items[]").unwrap();
        let collector = LogCollector::new(tx, buffer, false, filter.json_filter);

        let content = LogContent {
            prefix: "[test]".to_string(),
            content: r#"{"level":"error","message":"test"}"#.to_string(),
        };

        let result = collector.render_content(content);

        // エラーメッセージ + 注釈 + 元のログの3行が期待される
        assert_eq!(result.len(), 3);
        assert!(result[0].contains("jq evaluation error"));
        assert_eq!(result[1], "[test]  (showing original log below)");
        assert_eq!(result[2], r#"[test]  {"level":"error","message":"test"}"#);
    }
}
