# WorkerResult リファクタリング実装計画

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** `WorkerResult` enum を `ChangedContext` 構造体に変換し、ポーラーを `InfiniteWorker` trait に移行することで、型安全性の向上とパニックの除去を実現する。

**Architecture:** `AbortWorker` を `InfiniteWorker` にリネームし、無限ループのポーラー5つを `Worker` から `InfiniteWorker` に移行。`EventController` のみ `Worker<Output = ChangedContext>` を維持。コントローラーのメインループを `select_all` から `EventController` 単独 await + abort パターンに変更。

**Tech Stack:** Rust, async_trait, tokio, crossbeam channel

---

### Task 1: `AbortWorker` → `InfiniteWorker` リネーム（trait 定義）

**Files:**
- Modify: `src/workers/kube/worker.rs:21` — trait 名を変更

**Step 1: trait 名をリネーム**

`src/workers/kube/worker.rs` の `AbortWorker` を `InfiniteWorker` に変更:

```rust
#[async_trait]
pub trait InfiniteWorker {
    async fn run(&self);

    fn spawn(&self) -> AbortHandle
    where
        Self: Clone + Send + Sync + 'static,
    {
        let worker = self.clone();
        tokio::spawn(async move { worker.run().await }).abort_handle()
    }
}
```

**Step 2: 既存の `AbortWorker` 利用箇所をすべてリネーム**

以下のファイルで `AbortWorker` → `InfiniteWorker` に置換:

- `src/workers/kube/controller.rs:55` — `AbortWorker as _` → `InfiniteWorker as _`
- `src/features/get/kube/yaml.rs:24` — import
- `src/features/get/kube/yaml.rs:74` — `impl AbortWorker for GetYamlWorker`
- `src/features/network/kube/description.rs:20` — import
- `src/features/network/kube/description.rs:80` — `impl AbortWorker for NetworkDescriptionWorker`
- `src/features/config/kube/raw_data.rs:12` — import
- `src/features/config/kube/raw_data.rs:31` — `impl AbortWorker for ConfigsDataWorker`
- `src/features/yaml/kube/worker.rs:13` — import
- `src/features/yaml/kube/worker.rs:51` — `impl AbortWorker for YamlWorker`
- `src/features/pod/kube/log.rs:25` — import
- `src/features/pod/kube/log.rs:138` — `impl AbortWorker for LogWorker`
- `src/features/pod/kube/log/pod_watcher.rs:25` — import
- `src/features/pod/kube/log/log_streamer.rs:22` — import
- `src/features/pod/kube/log/log_streamer.rs:92` — `impl AbortWorker for LogStreamer`

**Step 3: ビルド確認**

Run: `cargo check 2>&1 | head -30`
Expected: ビルド成功（警告のみ OK）

**Step 4: コミット**

```bash
git add -A && git commit -m "refactor: rename AbortWorker to InfiniteWorker"
```

---

### Task 2: `WorkerResult` enum → `ChangedContext` 構造体

**Files:**
- Modify: `src/workers/kube/controller.rs:128-137` — enum → struct
- Modify: `src/workers/kube/controller.rs:471` — `EventController` の `type Output`
- Modify: `src/workers/kube/controller.rs:641` — `return WorkerResult::ChangedContext {..}` → `return ChangedContext {..}`
- Modify: `src/workers/kube/controller.rs:361-362` — `select_all` のマッチパターン（Task 3 で `select_all` 自体を削除するため、ここでは型のみ変更）

**Step 1: enum を struct に変換**

`src/workers/kube/controller.rs` の `WorkerResult` を以下に変更:

```rust
#[derive(Clone)]
pub struct ChangedContext {
    /// 切り替え後のコンテキスト
    pub target_context: String,

    /// 切り替え後のターゲットネームスペース
    pub target_namespaces: Option<TargetNamespaces>,
}
```

**Step 2: `EventController` の Output 型を変更**

`src/workers/kube/controller.rs:471`:

```rust
impl Worker for EventController {
    type Output = ChangedContext;
```

**Step 3: return 箇所を変更**

`src/workers/kube/controller.rs:641`:

```rust
return ChangedContext {
    target_context: name.clone(),
    target_namespaces,
};
```

**Step 4: `select_all` のマッチパターンを変更**

`src/workers/kube/controller.rs:361-365`:

```rust
Ok(ChangedContext {
    target_context,
    target_namespaces,
}) => {
```

**Step 5: import から `WorkerResult` を削除・利用箇所を更新**

ポーラーのファイルではまだ `WorkerResult` を import しているが、次の Task 3 で `Worker` → `InfiniteWorker` に変更するため、ここではまだ触れない。`controller.rs` 内のみ変更。

**Step 6: ビルド確認**

Run: `cargo check 2>&1 | head -30`
Expected: ポーラーが `WorkerResult` を参照しているためエラーが出る（期待通り、Task 3 で解消）

**Step 7: コミットしない（Task 3 と合わせてコミット）**

---

### Task 3: ポーラーを `InfiniteWorker` に移行

**Files:**
- Modify: `src/features/pod/kube/pod.rs:21,81-101`
- Modify: `src/features/event/kube/event.rs:19,68-93`
- Modify: `src/features/config/kube/config.rs:11,42-68`
- Modify: `src/features/network/kube/network.rs:31,267-292`
- Modify: `src/features/api_resources/kube/api_resources.rs:28-31,273-363`

**Step 1: `PodPoller` を移行**

`src/features/pod/kube/pod.rs`:
- import: `Worker, WorkerResult` → `InfiniteWorker`
- `logger` マクロの import を追加（未 import の場合）

```rust
use crate::{
    // ...
    logger,
    workers::kube::{SharedPodColumns, SharedTargetNamespaces, InfiniteWorker},
};

#[async_trait]
impl InfiniteWorker for PodPoller {
    async fn run(&self) {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));

        let Self { tx, .. } = self;

        loop {
            interval.tick().await;

            let pod_info = self.get_pod_info().await;

            if let Err(e) = tx.send(PodMessage::Poll(pod_info).into()) {
                logger!(error, "Failed to send PodMessage::Poll: {}", e);
                return;
            }
        }
    }
}
```

**Step 2: `EventPoller` を移行**

`src/features/event/kube/event.rs`:
- import: `Worker, WorkerResult` → `InfiniteWorker`

```rust
use crate::{
    // ...
    logger,
    workers::kube::{message::Kube, SharedTargetNamespaces, InfiniteWorker},
};

#[async_trait]
impl InfiniteWorker for EventPoller {
    async fn run(&self) {
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

            if let Err(e) = tx.send(Message::Kube(Kube::Event(event_list))) {
                logger!(error, "Failed to send Kube::Event: {}", e);
                return;
            }
        }
    }
}
```

**Step 3: `ConfigPoller` を移行**

`src/features/config/kube/config.rs`:
- import: `Worker, WorkerResult` → `InfiniteWorker`

```rust
use crate::{
    // ...
    logger,
    workers::kube::{SharedTargetNamespaces, InfiniteWorker},
};

#[async_trait]
impl InfiniteWorker for ConfigPoller {
    async fn run(&self) {
        let mut interval = tokio::time::interval(time::Duration::from_secs(1));

        let Self {
            tx,
            shared_target_namespaces,
            kube_client,
        } = self;

        loop {
            interval.tick().await;

            let target_namespaces = shared_target_namespaces.read().await;

            let table = fetch_configs(kube_client, &target_namespaces).await;

            if let Err(e) = tx.send(ConfigResponse::Table(table).into()) {
                logger!(error, "Failed to send ConfigResponse::Table: {}", e);
                return;
            }
        }
    }
}
```

**Step 4: `NetworkPoller` を移行**

`src/features/network/kube/network.rs`:
- import: `Worker, WorkerResult` → `InfiniteWorker`

```rust
use crate::{
    // ...
    logger,
    workers::kube::{SharedTargetNamespaces, InfiniteWorker},
};

#[async_trait()]
impl InfiniteWorker for NetworkPoller {
    async fn run(&self) {
        let mut interval = tokio::time::interval(time::Duration::from_secs(1));

        let tx = &self.tx;

        loop {
            interval.tick().await;

            let target_resources = {
                let apis = self.api_resources.read().await;
                target_resources(&apis)
            };

            let table = self.polling(&target_resources).await;

            if let Err(e) = tx.send(NetworkResponse::List(table).into()) {
                logger!(error, "Failed to send NetworkResponse::List: {}", e);
                return;
            }
        }
    }
}
```

**Step 5: `ApiPoller` を移行**

`src/features/api_resources/kube/api_resources.rs`:
- import: `Worker, WorkerResult` → `InfiniteWorker`
- `ApiPoller` は `send` 箇所が4つあるため、すべて `if let Err(e)` に変更

```rust
use crate::{
    // ...
    logger,
    workers::kube::{
        SharedTargetApiResources, SharedTargetNamespaces, TargetApiResources, TargetNamespaces,
        InfiniteWorker,
    },
};

#[async_trait]
impl InfiniteWorker for ApiPoller {
    async fn run(&self) {
        let Self {
            tx,
            shared_target_namespaces,
            kube_client,
            shared_target_api_resources,
            shared_api_resources,
            config,
        } = self;

        match fetch_api_resources(kube_client).await {
            Ok(fetched) => {
                let mut api_resources = shared_api_resources.write().await;
                *api_resources = fetched;
            }
            Err(err) => {
                if let Err(e) = tx.send(ApiResponse::Poll(Err(err)).into()) {
                    logger!(error, "Failed to send ApiResponse::Poll: {}", e);
                    return;
                }
            }
        }

        let mut interval = tokio::time::interval(time::Duration::from_millis(1000));

        let mut last_tick = Instant::now();
        let tick_rate = time::Duration::from_secs(10);

        let mut is_error = false;

        loop {
            interval.tick().await;

            if tick_rate < last_tick.elapsed() {
                last_tick = Instant::now();

                match fetch_api_resources(kube_client).await {
                    Ok(fetched) => {
                        let mut api_resources = shared_api_resources.write().await;
                        *api_resources = fetched;

                        if is_error {
                            is_error = false;
                            if let Err(e) = tx.send(ApiResponse::Poll(Ok(Default::default())).into()) {
                                logger!(error, "Failed to send ApiResponse::Poll: {}", e);
                                return;
                            }
                        }
                    }
                    Err(err) => {
                        if let Err(e) = tx.send(ApiResponse::Poll(Err(err)).into()) {
                            logger!(error, "Failed to send ApiResponse::Poll: {}", e);
                            return;
                        }
                        is_error = true;
                        continue;
                    }
                }
            }

            let target_namespaces = shared_target_namespaces.read().await;
            let target_api_resources = shared_target_api_resources.read().await;

            if target_api_resources.is_empty() {
                continue;
            }

            let result = FetchTargetApiResources::new(
                kube_client,
                &target_api_resources,
                &target_namespaces,
                config,
            )
            .fetch_table()
            .await;

            if let Err(e) = tx.send(ApiResponse::Poll(result).into()) {
                logger!(error, "Failed to send ApiResponse::Poll: {}", e);
                return;
            }
        }
    }
}
```

**Step 6: ビルド確認**

Run: `cargo check 2>&1 | head -30`
Expected: ビルド成功（controller.rs のハンドル型不整合は Task 4 で解消するかもしれないが、ここで確認）

**Step 7: コミット（Task 2 の変更と合わせて）**

```bash
git add -A && git commit -m "refactor: replace WorkerResult enum with ChangedContext struct and migrate pollers to InfiniteWorker"
```

---

### Task 4: コントローラーのハンドル管理を変更

**Files:**
- Modify: `src/workers/kube/controller.rs:305-403` — メインループを `select_all` から `EventController` 単独 await に変更

**Step 1: import の整理**

`src/workers/kube/controller.rs` の import から不要なものを削除:
- `futures::future::select_all` を削除
- `JoinHandle` は `EventController` で引き続き使うため残す

**Step 2: ハンドル管理パターンを変更**

ポーラーの `spawn()` は `AbortHandle` を返すようになったので、`Vec<JoinHandle>` に入れられない。
`EventController` の `JoinHandle` だけ `await` し、ポーラーは `AbortHandle` で管理:

```rust
let event_controller_handle = EventController::new(event_controller_args).spawn();

let pod_handle = PodPoller::new(/* ... */).spawn();
let config_handle = ConfigPoller::new(/* ... */).spawn();
let network_handle = NetworkPoller::new(/* ... */).spawn();
let event_handle = EventPoller::new(/* ... */).spawn();
let api_handle = ApiPoller::new(/* ... */).spawn();

let abort_handles = vec![
    pod_handle,
    config_handle,
    network_handle,
    event_handle,
    api_handle,
];

let result = event_controller_handle.await;

match result {
    Ok(ChangedContext {
        target_context,
        target_namespaces,
    }) => {
        for h in &abort_handles {
            h.abort();
        }

        let shared_target_namespaces = shared_target_namespaces.read().await;
        let shared_api_resources = shared_target_api_resources.read().await;

        store.insert(
            context.to_string(),
            KubeState::new(
                client.clone(),
                shared_target_namespaces.to_vec(),
                shared_api_resources.to_vec(),
            ),
        );

        context = target_context;

        if let Some(ns) = target_namespaces {
            override_namespaces = Some(ns);
        }
    }
    Err(e) => {
        for h in &abort_handles {
            h.abort();
        }
        tx.send(Message::Error(NotifyError::from_anyhow(
            ErrorSource::Worker,
            &anyhow::Error::from(e).context("KubeProcess Error"),
        )))?;
    }
}
```

**Step 3: `Self::abort` メソッドの更新**

`Self::abort` は `&[JoinHandle<T>]` を受け取っていたが、`AbortHandle` 用に変更するか、直接 for ループで abort する。上のコードで直接 for ループを使っているため、`Self::abort` メソッドが不要なら削除。`EventController` 内でも `Self::abort` 相当のことをしているか確認して判断する。

**Step 4: ビルド確認**

Run: `cargo check 2>&1 | head -30`
Expected: ビルド成功

**Step 5: コミット**

```bash
git add -A && git commit -m "refactor: simplify controller handle management by awaiting EventController only"
```

---

### Task 5: 最終確認・クリーンアップ

**Files:**
- 全体

**Step 1: テスト実行**

Run: `cargo test 2>&1 | tail -20`
Expected: 全テスト PASS

**Step 2: clippy 確認**

Run: `cargo clippy 2>&1 | head -30`
Expected: エラーなし

**Step 3: 不要な import・未使用コードの確認**

- `WorkerResult` への参照が完全に除去されていることを確認
- `select_all` の import が除去されていることを確認
- `Self::abort` メソッドが不要なら削除されていることを確認

**Step 4: 修正があればコミット**

```bash
git add -A && git commit -m "refactor: clean up unused imports and dead code"
```
