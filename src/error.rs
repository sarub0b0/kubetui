// 後続タスク（ステータスバー等）で全フィールド・バリアントが使用される
#![allow(dead_code)]

use std::fmt;

/// UI に通知するエラー情報
#[derive(Debug, Clone)]
pub struct NotifyError {
    pub source: ErrorSource,
    pub message: String,
}

impl NotifyError {
    pub fn new(source: ErrorSource, message: impl fmt::Display) -> Self {
        Self {
            source,
            message: message.to_string(),
        }
    }

    pub fn from_anyhow(source: ErrorSource, err: &anyhow::Error) -> Self {
        Self {
            source,
            message: format!("{:#}", err),
        }
    }
}

/// エラーの発生源
#[derive(Debug, Clone, Copy)]
pub enum ErrorSource {
    /// Pod feature (一覧取得等)
    Pod,
    /// ログストリーミング
    Log,
    /// ConfigMap/Secret feature
    Config,
    /// Service/Ingress 等ネットワーク feature
    Network,
    /// Kubernetes Event feature
    Event,
    /// API リソース検出 (api_resources feature)
    Api,
    /// YAML 表示 feature
    Yaml,
    /// kubeconfig コンテキスト操作
    Context,
    /// Namespace 切替・検証
    Namespace,
    /// ワーカープロセス自体のクラッシュ
    Worker,
}

impl fmt::Display for ErrorSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pod => write!(f, "Pod"),
            Self::Log => write!(f, "Log"),
            Self::Config => write!(f, "Config"),
            Self::Network => write!(f, "Network"),
            Self::Event => write!(f, "Event"),
            Self::Api => write!(f, "Api"),
            Self::Yaml => write!(f, "Yaml"),
            Self::Context => write!(f, "Context"),
            Self::Namespace => write!(f, "Namespace"),
            Self::Worker => write!(f, "Worker"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notify_error_new() {
        let err = NotifyError::new(ErrorSource::Pod, "test error");
        assert_eq!(err.message, "test error");
        assert!(matches!(err.source, ErrorSource::Pod));
    }

    #[test]
    fn notify_error_from_anyhow() {
        let anyhow_err = anyhow::anyhow!("root cause").context("operation failed");
        let err = NotifyError::from_anyhow(ErrorSource::Worker, &anyhow_err);
        assert!(err.message.contains("operation failed"));
        assert!(err.message.contains("root cause"));
    }

    #[test]
    fn error_source_display() {
        assert_eq!(ErrorSource::Pod.to_string(), "Pod");
        assert_eq!(ErrorSource::Namespace.to_string(), "Namespace");
        assert_eq!(ErrorSource::Worker.to_string(), "Worker");
    }
}
