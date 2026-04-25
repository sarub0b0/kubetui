# Error Handling Design

## Background

kubetui のエラー処理は場当たり的に実装されており、以下の問題がある:

1. **`Message::Error` が UI に表示されない** — ワーカークラッシュ時、ログに記録されるだけでユーザーに見えない (`src/workers/render/action.rs:93-95`)
2. **エラー型が `anyhow::Error` のみ** — エラーの発生源を判別できない（`FilterError` 以外にカスタム型なし）
3. **サイレント障害** — ログストリーム失敗、ns 切替フォールバックなどでエラーが消失
4. **チャンネル送信でパニック** — ポーリングワーカーの `tx.send().expect()` が複数箇所にある
5. **エラーコンテキスト不足** — `.context()` 未使用で、何の操作で失敗したか不明

この改善は、将来のステータスバー/トースト等の UI エラー表示の土台となる。

### 現在のエラー経路

| 経路 | 現状 | 問題 |
|------|------|------|
| Feature ポーリングエラー (`Result<T>` in messages) | ウィジェットにインライン表示 | 動作している（改善は別途） |
| `Message::Error` (ワーカークラッシュ) | `logger!` でログのみ | UI に表示されない |
| ns 切替フォールバック | サイレントに旧 ns に戻る | ユーザーに通知されない |
| ログストリームエラー | `logger!` でログのみ | ユーザーに通知されない |
| チャンネル送信失敗 | `.expect()` でパニック | 不適切な終了 |

### エラーの分類方針

- **アプリ終了すべきエラー**: 初期化失敗（ランタイム生成、kubeconfig 読み込み、ターミナル初期化）  
  → 既存の `?` 伝搬で対応済み。変更不要
- **UI 表示すべきエラー**: 動作中に発生するすべてのエラー（API エラー、ワーカークラッシュ等）  
  → `NotifyError` を導入し、発生源情報を付与する

## Design

### NotifyError と ErrorSource

UI に通知するエラー情報を構造化する。エラーの発生源 (`ErrorSource`) と
ユーザー向けメッセージ (`message`) を持つ。

```rust
// src/error.rs (新規ファイル)

#[derive(Debug, Clone)]
pub struct NotifyError {
    pub source: ErrorSource,
    pub message: String,
}

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
```

メソッド:
- `NotifyError::new(source, impl Display)` — 通常のエラー生成
- `NotifyError::from_anyhow(source, &anyhow::Error)` — anyhow からの変換。`format!("{:#}", err)` でエラーチェーンを読みやすい形式で保持する（`{:?}` は Debug 表示でノイズが多いため `{:#}` を使用）
- `ErrorSource` に `Display` trait を実装（バリアント名をそのまま出力）:

```rust
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

`anyhow::Error` を直接保持しない理由:
- `anyhow::Error` は `Clone` を実装しておらず、render レイヤーでの取り回しが難しい
- エラーチェーン情報は生成時に文字列化して保持すれば十分

### Message::Error の変更

`Message::Error` のペイロードを `anyhow::Error` から `NotifyError` に変更する。

```rust
// src/message.rs
pub enum Message {
    Kube(Kube),
    User(UserEvent),
    Tick,
    Error(NotifyError),  // Changed from anyhow::Error
}
```

送信元（`src/workers/kube/controller.rs:378-381`）:

```rust
// Before
tx.send(Message::Error(anyhow!("KubeProcess Error: {:?}", e)))?;

// After
tx.send(Message::Error(NotifyError::from_anyhow(
    ErrorSource::Worker,
    &e.context("KubeProcess Error"),
)))?;
```

### エラーの共有状態と受信

render スレッド内で `Rc<RefCell<Option<NotifyError>>>` を作成し、
`window_action` 関数に渡す。`Message::Error` 受信時にこの状態に保存する。

```rust
// src/workers/render.rs
fn render(&self) -> Result<()> {
    let last_error: Rc<RefCell<Option<NotifyError>>> = Rc::new(RefCell::new(None));
    // ...
    loop {
        // ...
        match window_action(&mut window, &self.rx, &last_error) {
            // ...
        }
    }
}

// src/workers/render/action.rs
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
    WindowAction::Continue
}
```

この時点では `last_error` は保存のみで UI 表示しない。
UI 表示（ステータスバー等）は後続タスクで対応する。

**注意**: 現設計では `Option<NotifyError>` で最新エラー1件のみ保持する。
クラスタ接続断等で複数ワーカーが短時間にエラーを送信した場合、先行エラーは上書きされる。
UI 表示の実装時に `Vec<NotifyError>` やリングバッファへの変更を検討すること。

### サイレント障害の修正

#### ns 切替フォールバックの通知

namespace が見つからない場合に `Message::Error(NotifyError)` を送信する。

```rust
// src/workers/kube/controller.rs (ns バリデーション部分)
let _ = tx.send(Message::Error(NotifyError::new(
    ErrorSource::Namespace,
    format!("Namespaces {:?} not found, using stored namespaces", not_found),
)));
```

#### ログストリームエラー

ログバッファへの直接追加は、接続断絶時に 3 秒ごとに大量のエラー行が
ログウィジェットに混入するため不適切。
`NotifyError` 経由でステータスバー等に表示する方針とし、
ステータスバー実装時に対応する（今回は対象外）。

### エラーコンテキストの追加

高インパクトな箇所に限定して `.context()` を追加する:

| ファイル | 箇所 | 追加内容 |
|---------|------|---------|
| `src/workers/kube/controller.rs` | `store.ensure_context()` | `.context("Failed to initialize context")` |
| `src/workers/kube/controller.rs` | `fetch_all_namespaces()` | `.context("Failed to fetch namespaces")` |
| `src/features/api_resources/kube/api_resources.rs` | `Discovery::new().run()` | `.context("Failed to discover API resources")` |

### チャンネル送信の安全化

#### AbortWorker 実装（戻り値 `()`）

`.expect()` を明示的なエラーハンドリングに置換する。
送信失敗時はエラー内容をログに記録し、`return` でワーカーを終了する。

```rust
// Before
tx.send(msg).expect("Failed to send ...");

// After
if let Err(e) = tx.send(msg) {
    logger!(error, "Failed to send message: {}", e);
    return;
}
```

対象:
- `src/features/yaml/kube/worker.rs` — YamlWorker::run
- `src/features/get/kube/yaml.rs` — GetYamlWorker::run
- `src/features/pod/kube/log.rs` — send_response! マクロ内

注: `src/features/pod/kube/log/log_streamer.rs` はチャンネル送信ではなく
`LogBuffer`（`Arc<Mutex>`）を直接操作しているため `.expect()` の問題はない。
ただし fetch エラー時に `logger!` でログするのみ（line 107-108）であり、
`NotifyError` 経由でユーザーに通知する改善は検討に値する（後述のログストリームエラーの項を参照）。

#### Worker 実装（無限ループ、戻り値 `WorkerResult`）

Worker trait は無限ループで `WorkerResult` を返す設計のため、
ループを `break` しても適切な戻り値がない。
`.expect()` によるパニック動作を維持し、後続タスク用にコメントを残す。

```rust
// TODO: Worker trait の改善時に、チャンネル切断を graceful に処理する。
// 現状は Worker::run() が WorkerResult を返す設計で、チャンネル切断時の
// 適切な戻り値がないため、パニックで対応している。
tx.send(msg).expect("Failed to send ...");
```

対象:
- `src/features/pod/kube/pod.rs` — PodPoller::run
- `src/features/config/kube/config.rs` — ConfigPoller::run
- `src/features/event/kube/event.rs` — EventPoller::run
- `src/features/network/kube/network.rs` — NetworkPoller::run
- `src/features/api_resources/kube/api_resources.rs` — ApiPoller::run

#### `.expect()` を維持する箇所

チャンネルが壊れている場合はアプリとして致命的な状態であるため、
以下の箇所は `.expect()` を維持する:

**ユーザー操作へのレスポンス送信:**
- `src/workers/render/window.rs` — ダイアログからの送信
- `src/features/*/view/` 以下 — UI コールバックからの送信
- `src/workers/kube/controller.rs` — ユーザーリクエストへの応答送信（NamespaceResponse::Get/Set, ApiResponse::Get, ContextResponse::Get, YamlResponse::APIs/Resource）

**メッセージ受信:**
- `src/workers/render/action.rs` — `rx.recv().expect("Failed to recv")`（render スレッドのメッセージ受信。チャンネル切断はアプリ終了を意味する）

## 検証方法

1. `cargo build` — コンパイル通過
2. `cargo test` — 既存テスト通過
3. `cargo clippy` — 警告なし
4. `NotifyError` のユニットテスト: `new`, `from_anyhow`, `ErrorSource` の `Display` の基本動作を確認
5. 手動テスト: `--logging` フラグ付きで起動し、クラスタ接続断時に `Message::Error` 受信の既存ログ出力（`"Error: {:?}"`）で `NotifyError` の `source` と `message` が正しく表示されることを確認（`last_error` への保存は同タイミングで行われる）

## 対象外（後続タスク）

- ステータスバー/トースト等の UI エラー表示
- `ErrorSeverity` (Transient/Persistent/Fatal) の分類
- ログストリームエラーの `NotifyError` 経由での通知（`log_streamer.rs` の fetch エラー含む。ステータスバー実装時に対応）
- `last_error` の複数エラー保持（`Vec<NotifyError>` やリングバッファへの変更検討）
- `ContextResponse::Get` の `Result` 追加（現状 infallible のため不要）
- EventController 内の `.expect()` 修正（Worker trait の変更が必要で影響範囲が大きい）
- Worker trait の改善（チャンネル切断時に graceful に終了できるよう WorkerResult の拡張等）
