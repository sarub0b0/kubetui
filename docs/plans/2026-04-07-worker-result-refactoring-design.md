# WorkerResult リファクタリング設計

## 動機

1. **パニック問題の解消:** 無限ループのポーラーが `Worker<Output = WorkerResult>` を実装しているが、正常に return できないため `tx.send().expect()` でパニックしている
2. **enum 構造の改善:** `WorkerResult` はバリアントが `ChangedContext` の1つだけで、enum として不自然

## アプローチ

`AbortWorker` を `InfiniteWorker` にリネームし、ポーラーを `InfiniteWorker` に移行。`WorkerResult` enum を `ChangedContext` 構造体に変換する。

## 設計詳細

### 1. trait の変更

`AbortWorker` を `InfiniteWorker` にリネーム。シグネチャの変更はなし。

```rust
// worker.rs
pub trait Worker {
    type Output;
    async fn run(&self) -> Self::Output;
    fn spawn(&self) -> JoinHandle<Self::Output>;
}

pub trait InfiniteWorker {  // AbortWorker → リネーム
    async fn run(&self);
    fn spawn(&self) -> AbortHandle;
}
```

### 2. `WorkerResult` enum → `ChangedContext` 構造体

```rust
// 変更前
#[derive(Clone)]
pub enum WorkerResult {
    ChangedContext {
        target_context: String,
        target_namespaces: Option<TargetNamespaces>,
    },
}

// 変更後
#[derive(Clone)]
pub struct ChangedContext {
    pub target_context: String,
    pub target_namespaces: Option<TargetNamespaces>,
}
```

`EventController` は `Worker<Output = ChangedContext>` を実装。

### 3. ポーラーの trait 移行

対象5つ:
- `PodPoller` (`src/features/pod/kube/pod.rs`)
- `EventPoller` (`src/features/event/kube/event.rs`)
- `ConfigPoller` (`src/features/config/kube/config.rs`)
- `NetworkPoller` (`src/features/network/kube/network.rs`)
- `ApiPoller` (`src/features/api_resources/kube/api_resources.rs`)

各ポーラーを `Worker` から `InfiniteWorker` に変更。`tx.send().expect()` を `if let Err(e)` でログ出力後 `return` に置き換え。

```rust
// 変更前
#[async_trait]
impl Worker for PodPoller {
    type Output = WorkerResult;
    async fn run(&self) -> Self::Output {
        loop {
            tx.send(PodMessage::Poll(pod_info).into())
                .expect("Failed to Kube::Pod");
        }
    }
}

// 変更後
#[async_trait]
impl InfiniteWorker for PodPoller {
    async fn run(&self) {
        loop {
            if let Err(e) = tx.send(PodMessage::Poll(pod_info).into()) {
                logger!(error, "Failed to send PodMessage::Poll: {}", e);
                return;
            }
        }
    }
}
```

### 4. コントローラーのハンドル管理

ポーラーは `AbortHandle`、`EventController` だけ `JoinHandle<ChangedContext>` で管理。`select_all` が不要になる。

```rust
// ポーラーは AbortHandle
let pod_handle: AbortHandle = pod_poller.spawn();
let config_handle: AbortHandle = config_poller.spawn();
// ...

// EventController だけ JoinHandle
let event_controller_handle = event_controller.spawn();
let result = event_controller_handle.await;

match result {
    Ok(ChangedContext { target_context, target_namespaces }) => {
        // ポーラーを abort して、コンテキスト切り替え処理
    }
    Err(join_error) => { /* パニック等のハンドリング */ }
}
```

### 決定事項

- コンテキスト切り替えフローは現状を維持
- チャンネル切断時は `if let Err(e)` でログ出力後 `return`（伝搬先がないため）
- ロギングはプロジェクト既存の `logger!` マクロを使用
