# Error Handling 実行計画

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `NotifyError` 型を導入し、エラーの発生源情報を構造化。サイレント障害の通知、チャンネル送信のパニック削減を行う。

**Architecture:** 新規 `src/error.rs` に `NotifyError` / `ErrorSource` を定義し、`Message::Error` のペイロードを置換。render スレッドで `last_error` 状態を保持する。UI 表示は後続タスク。

**Tech Stack:** Rust, anyhow, crossbeam

**Design doc:** `docs/plans/2026-04-06-error-handling-design.md`

---

## ファイル構成

| 操作 | ファイル | 責務 |
|------|---------|------|
| 新規作成 | `src/error.rs` | `NotifyError`, `ErrorSource` の定義 |
| 変更 | `src/main.rs` | `mod error` 追加 |
| 変更 | `src/message.rs` | `Message::Error` のペイロード変更 |
| 変更 | `src/workers/render.rs` | `last_error` 状態の作成と受け渡し |
| 変更 | `src/workers/render/action.rs` | `Message::Error` 受信時に `last_error` へ保存 |
| 変更 | `src/workers/kube/controller.rs` | `NotifyError` 送信、ns フォールバック通知、`.context()` 追加 |
| 変更 | `src/features/api_resources/kube/api_resources.rs` | `.context()` 追加、Worker の TODO コメント |
| 変更 | `src/features/yaml/kube/worker.rs` | `.expect()` → graceful return |
| 変更 | `src/features/get/kube/yaml.rs` | `.expect()` → graceful return |
| 変更 | `src/features/pod/kube/log.rs` | `send_response!` マクロの `.expect()` → graceful return |
| 変更 | `src/features/pod/kube/pod.rs` | Worker の TODO コメント |
| 変更 | `src/features/config/kube/config.rs` | Worker の TODO コメント |
| 変更 | `src/features/event/kube/event.rs` | Worker の TODO コメント |
| 変更 | `src/features/network/kube/network.rs` | Worker の TODO コメント |

---

### Task 1: NotifyError と ErrorSource の実装

**Files:**
- Create: `src/error.rs`
- Modify: `src/main.rs:1`

- [ ] **Step 1: `src/error.rs` を作成**

```rust
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
```

- [ ] **Step 2: `src/main.rs` に `mod error` を追加**

`mod message;` の前に追加:

```rust
mod error;
```

- [ ] **Step 3: テストを作成**

`src/error.rs` の末尾に追加:

```rust
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
```

- [ ] **Step 4: テスト実行**

Run: `cargo test -p kubetui error`
Expected: 3 テストが PASS

- [ ] **Step 5: コミット**

```bash
git add src/error.rs src/main.rs
git commit -m "feat: add NotifyError and ErrorSource types"
```

---

### Task 2: Message::Error のペイロード変更

**Files:**
- Modify: `src/message.rs:42`
- Modify: `src/workers/kube/controller.rs:378-381`

- [ ] **Step 1: `src/message.rs` を変更**

import を追加し、`Error` バリアントを変更:

```rust
// 追加 import
use crate::error::NotifyError;
```

```rust
// 変更前
Error(anyhow::Error),

// 変更後
Error(NotifyError),
```

- [ ] **Step 2: `src/workers/kube/controller.rs:378-381` を変更**

import を追加:

```rust
use crate::error::{ErrorSource, NotifyError};
```

送信箇所を変更:

```rust
// 変更前
tx.send(Message::Error(anyhow!("KubeProcess Error: {:?}", e)))?;

// 変更後
tx.send(Message::Error(NotifyError::from_anyhow(
    ErrorSource::Worker,
    &e.context("KubeProcess Error"),
)))?;
```

注: `anyhow!` マクロの import が不要になった場合は削除する。他の箇所で使用されていれば維持。

- [ ] **Step 3: コンパイル確認**

Run: `cargo build 2>&1 | head -30`
Expected: コンパイル通過（`Message::Error` の型変更によりパターンマッチ箇所でエラーが出る場合は Task 3 で解消）

- [ ] **Step 4: コミット**

```bash
git add src/message.rs src/workers/kube/controller.rs
git commit -m "feat: change Message::Error payload to NotifyError"
```

---

### Task 3: render での last_error 保持

**Files:**
- Modify: `src/workers/render.rs:90-137`
- Modify: `src/workers/render/action.rs:69-98`

- [ ] **Step 1: `src/workers/render/action.rs` を変更**

import に `NotifyError` を追加:

```rust
use crate::error::NotifyError;
```

`window_action` のシグネチャと `Message::Error` ハンドラを変更:

```rust
// 変更前
pub fn window_action(window: &mut Window, rx: &Receiver<Message>) -> WindowAction {
    match rx.recv().expect("Failed to recv") {
        // ...
        Message::Error(err) => {
            logger!(error, "Error: {:?}", err);
        }
    }

// 変更後
pub fn window_action(
    window: &mut Window,
    rx: &Receiver<Message>,
    last_error: &Rc<RefCell<Option<NotifyError>>>,
) -> WindowAction {
    match rx.recv().expect("Failed to recv") {
        // ...
        Message::Error(err) => {
            logger!(error, "Error: {:?}", err);
            *last_error.borrow_mut() = Some(err);
        }
    }
```

import に `Rc`, `RefCell` を追加（既存の `use std::collections::BTreeMap` の付近）:

```rust
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};
```

- [ ] **Step 2: `src/workers/render.rs` を変更**

`render` 関数内で `last_error` を作成し、`window_action` に渡す:

```rust
// 変更前 (line 90-91)
fn render(&self) -> Result<()> {
    let namespace = Rc::new(RefCell::new(Namespace::new()));

// 変更後
fn render(&self) -> Result<()> {
    let last_error: Rc<RefCell<Option<NotifyError>>> = Rc::new(RefCell::new(None));
    let namespace = Rc::new(RefCell::new(Namespace::new()));
```

import に `NotifyError` を追加:

```rust
use crate::error::NotifyError;
```

`window_action` 呼び出し箇所を変更:

```rust
// 変更前 (line 120)
match window_action(&mut window, &self.rx) {

// 変更後
match window_action(&mut window, &self.rx, &last_error) {
```

- [ ] **Step 3: ビルド確認**

Run: `cargo build`
Expected: コンパイル通過

- [ ] **Step 4: テスト実行**

Run: `cargo test`
Expected: 既存テストすべて PASS

- [ ] **Step 5: コミット**

```bash
git add src/workers/render.rs src/workers/render/action.rs
git commit -m "feat: store last_error in render thread on Message::Error"
```

---

### Task 4: ns 切替フォールバックの通知

**Files:**
- Modify: `src/workers/kube/controller.rs:241-253`

- [ ] **Step 1: ns フォールバック時に `Message::Error` を送信**

`controller.rs` の namespace バリデーション部分を変更。`found_namespaces.is_empty()` の分岐内にエラー通知を追加:

```rust
// 変更前 (line 241-244)
if found_namespaces.is_empty() {
    // まったく存在しない場合：ストアにフォールバック
    crate::logger!(warn, "No namespaces found: {not_found_namespaces:?}. Falling back to stored namespaces: {stored_target_namespaces:?}");
    // stored_target_namespaces はそのまま（ストアの値を使用）

// 変更後
if found_namespaces.is_empty() {
    // まったく存在しない場合：ストアにフォールバック
    crate::logger!(warn, "No namespaces found: {not_found_namespaces:?}. Falling back to stored namespaces: {stored_target_namespaces:?}");
    let _ = tx.send(Message::Error(NotifyError::new(
        ErrorSource::Namespace,
        format!("Namespaces {:?} not found, using stored namespaces", not_found_namespaces),
    )));
    // stored_target_namespaces はそのまま（ストアの値を使用）
```

一部見つからなかったケース（line 247-248）にも追加:

```rust
// 変更前
if !not_found_namespaces.is_empty() {
    crate::logger!(warn, "Some namespaces not found: {not_found_namespaces:?}. Using available namespaces: {found_namespaces:?}");

// 変更後
if !not_found_namespaces.is_empty() {
    crate::logger!(warn, "Some namespaces not found: {not_found_namespaces:?}. Using available namespaces: {found_namespaces:?}");
    let _ = tx.send(Message::Error(NotifyError::new(
        ErrorSource::Namespace,
        format!("Some namespaces not found: {:?}, using: {:?}", not_found_namespaces, found_namespaces),
    )));
```

- [ ] **Step 2: ビルド確認**

Run: `cargo build`
Expected: コンパイル通過

- [ ] **Step 3: コミット**

```bash
git add src/workers/kube/controller.rs
git commit -m "feat: notify namespace fallback via Message::Error"
```

---

### Task 5: エラーコンテキストの追加

**Files:**
- Modify: `src/workers/kube/controller.rs:224,233`
- Modify: `src/features/api_resources/kube/api_resources.rs:355`

- [ ] **Step 1: `controller.rs` に `.context()` を追加**

`anyhow::Context` の import を確認し、必要なら追加:

```rust
use anyhow::Context as _;
```

```rust
// 変更前 (line 224)
store.ensure_context(&kubeconfig, &context).await?;

// 変更後
store.ensure_context(&kubeconfig, &context).await.context("Failed to initialize context")?;
```

```rust
// 変更前 (line 233)
let fetched_namespaces = fetch_all_namespaces(client.clone()).await?;

// 変更後
let fetched_namespaces = fetch_all_namespaces(client.clone()).await.context("Failed to fetch namespaces")?;
```

- [ ] **Step 2: `api_resources.rs` に `.context()` を追加**

```rust
use anyhow::Context as _;
```

```rust
// 変更前 (line 355)
let discovery = Discovery::new(client.to_client()).run().await?;

// 変更後
let discovery = Discovery::new(client.to_client()).run().await.context("Failed to discover API resources")?;
```

- [ ] **Step 3: ビルド確認**

Run: `cargo build`
Expected: コンパイル通過

- [ ] **Step 4: コミット**

```bash
git add src/workers/kube/controller.rs src/features/api_resources/kube/api_resources.rs
git commit -m "feat: add error context to high-impact operations"
```

---

### Task 6: AbortWorker の `.expect()` を graceful return に変更

**Files:**
- Modify: `src/features/yaml/kube/worker.rs:75-77`
- Modify: `src/features/get/kube/yaml.rs:123-132`
- Modify: `src/features/pod/kube/log.rs:37-45,154-156`

- [ ] **Step 1: `src/features/yaml/kube/worker.rs` を変更**

```rust
// 変更前 (line 75-77)
self.tx
    .send(YamlResponse::Yaml(fetched_data).into())
    .expect("Failed to send YamlResponse::Yaml");

// 変更後
if let Err(e) = self.tx.send(YamlResponse::Yaml(fetched_data).into()) {
    logger!(error, "Failed to send YamlResponse::Yaml: {}", e);
    return;
}
```

- [ ] **Step 2: `src/features/get/kube/yaml.rs` を変更**

```rust
// 変更前 (line 123-132)
self.tx
    .send(
        GetResponse {
            yaml,
            kind: kind.to_string(),
            name: name.to_string(),
        }
        .into(),
    )
    .expect("Failed to send YamlResponse::Yaml");

// 変更後
if let Err(e) = self.tx.send(
    GetResponse {
        yaml,
        kind: kind.to_string(),
        name: name.to_string(),
    }
    .into(),
) {
    logger!(error, "Failed to send GetResponse: {}", e);
    return;
}
```

- [ ] **Step 3: `src/features/pod/kube/log.rs` の `send_response!` マクロを変更**

```rust
// 変更前 (line 38-45)
#[macro_export]
macro_rules! send_response {
    ($tx:expr, $msg:expr) => {
        use $crate::features::pod::message::LogMessage;

        $tx.send(LogMessage::Response($msg).into())
            .expect("Failed to send LogMessage::Response");
    };
}

// 変更後
#[macro_export]
macro_rules! send_response {
    ($tx:expr, $msg:expr) => {
        use $crate::features::pod::message::LogMessage;

        if let Err(e) = $tx.send(LogMessage::Response($msg).into()) {
            $crate::logger!(error, "Failed to send LogMessage::Response: {}", e);
            return;
        }
    };
}
```

- [ ] **Step 4: `src/features/pod/kube/log.rs:154-156` の `SetMaxLines` 送信を変更**

```rust
// 変更前 (line 154-156)
self.tx
    .send(LogMessage::SetMaxLines(filter.limit).into())
    .expect("Failed to send LogMessage::SetMaxLines");

// 変更後
if let Err(e) = self.tx.send(LogMessage::SetMaxLines(filter.limit).into()) {
    logger!(error, "Failed to send LogMessage::SetMaxLines: {}", e);
    return;
}
```

- [ ] **Step 5: ビルド確認**

Run: `cargo build`
Expected: コンパイル通過

- [ ] **Step 6: テスト実行**

Run: `cargo test`
Expected: 既存テストすべて PASS

- [ ] **Step 7: コミット**

```bash
git add src/features/yaml/kube/worker.rs src/features/get/kube/yaml.rs src/features/pod/kube/log.rs
git commit -m "fix: replace expect with graceful return in AbortWorker sends"
```

---

### Task 7: Worker の `.expect()` に TODO コメント追加

**Files:**
- Modify: `src/features/pod/kube/pod.rs:95-96`
- Modify: `src/features/config/kube/config.rs:62-63`
- Modify: `src/features/event/kube/event.rs:87-88`
- Modify: `src/features/network/kube/network.rs:286-287`
- Modify: `src/features/api_resources/kube/api_resources.rs:293-294,319-320,324-325,348-349`

- [ ] **Step 1: 各ファイルの `.expect()` の直前に TODO コメントを追加**

以下のコメントを各 `.expect()` の直前の行に追加する:

```rust
// TODO: Worker trait の改善時に、チャンネル切断を graceful に処理する。
// 現状は Worker::run() が WorkerResult を返す設計で、チャンネル切断時の
// 適切な戻り値がないため、パニックで対応している。
```

対象箇所:
- `src/features/pod/kube/pod.rs:95` — `tx.send(PodMessage::Poll(...).into()).expect(...)`
- `src/features/config/kube/config.rs:62` — `tx.send(ConfigResponse::Table(...).into()).expect(...)`
- `src/features/event/kube/event.rs:87` — `tx.send(Message::Kube(Kube::Event(...))).expect(...)`
- `src/features/network/kube/network.rs:286` — `tx.send(NetworkResponse::List(...).into()).expect(...)`
- `src/features/api_resources/kube/api_resources.rs:293,319,324,348` — `tx.send(ApiResponse::Poll(...).into()).expect(...)` × 4箇所

- [ ] **Step 2: ビルド確認**

Run: `cargo build`
Expected: コンパイル通過（コメントのみのため確実）

- [ ] **Step 3: コミット**

```bash
git add src/features/pod/kube/pod.rs src/features/config/kube/config.rs src/features/event/kube/event.rs src/features/network/kube/network.rs src/features/api_resources/kube/api_resources.rs
git commit -m "chore: add TODO comments for Worker channel send expect calls"
```

---

### Task 8: 最終検証

- [ ] **Step 1: clippy 実行**

Run: `cargo clippy -- -D warnings`
Expected: 警告なし

- [ ] **Step 2: 全テスト実行**

Run: `cargo test`
Expected: すべて PASS（Task 1 で追加した 3 テスト含む）

- [ ] **Step 3: 未使用 import の確認**

`controller.rs` で `anyhow!` マクロが不要になった場合、`use anyhow::anyhow` を削除する。
clippy が警告を出すので Step 1 で検出可能。
