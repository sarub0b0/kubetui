# Namespace Config Fallback Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** namespace一覧の取得権限がない環境で、設定ファイルに定義したnamespace一覧をフォールバックとして表示し、UIからnamespace切り替えを可能にする。

**Architecture:** `Config` に `fallback_namespaces` フィールドを追加し、`KubeWorkerConfig` → `KubeController` へ伝達する。`fetch_all_namespaces` 失敗時にフォールバックを使用し、`NamespaceResponse::GetFallback` バリアントで UI に通知。UI側ではリストタイトルを `Items (from config)` に変更する。

**Tech Stack:** Rust, serde (YAML deserialization), figment (config loading), ratatui (TUI), enum_dispatch (widget trait dispatch)

**Design:** `docs/plans/2026-04-05-namespace-config-fallback-design.md`

---

### Task 1: `Config` に `fallback_namespaces` フィールドを追加

**Files:**
- Modify: `src/config.rs:27-32`

**Step 1: `Config` 構造体にフィールドを追加**

`src/config.rs` の `Config` 構造体に `fallback_namespaces` を追加する。

```rust
#[derive(Default, Debug, Deserialize, Serialize)]
pub struct Config {
    pub theme: ThemeConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
    #[serde(default)]
    pub fallback_namespaces: Option<Vec<String>>,
}
```

**Step 2: ビルドして確認**

Run: `cargo build 2>&1 | head -20`
Expected: コンパイル成功

**Step 3: コミット**

```bash
git add src/config.rs
git commit -m "feat: add fallback_namespaces field to Config struct"
```

---

### Task 2: `Config.fallback_namespaces` のデシリアライズテスト

**Files:**
- Modify: `src/config.rs`

**Step 1: テストモジュールを追加**

`src/config.rs` の末尾にテストを追加する。

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;

    #[test]
    fn fallback_namespaces_が設定されている場合() {
        let yaml = indoc! {"
            fallback_namespaces:
              - production
              - staging
              - dev
        "};
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            config.fallback_namespaces,
            Some(vec![
                "production".to_string(),
                "staging".to_string(),
                "dev".to_string(),
            ])
        );
    }

    #[test]
    fn fallback_namespaces_が未設定の場合() {
        let yaml = indoc! {"
            logging:
              max_lines: 1000
        "};
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.fallback_namespaces, None);
    }

    #[test]
    fn fallback_namespaces_が空配列の場合() {
        let yaml = indoc! {"
            fallback_namespaces: []
        "};
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.fallback_namespaces, Some(vec![]));
    }
}
```

**Step 2: テスト実行**

Run: `cargo test --lib config::tests`
Expected: 3件すべてPASS

**Step 3: コミット**

```bash
git add src/config.rs
git commit -m "test: add deserialization tests for fallback_namespaces"
```

---

### Task 3: `KubeWorkerConfig` に `fallback_namespaces` を追加し `app.rs` で接続

**Files:**
- Modify: `src/workers/kube/config.rs:12-23`
- Modify: `src/app.rs:25-60`

**Step 1: `KubeWorkerConfig` にフィールドを追加**

`src/workers/kube/config.rs`:

```rust
#[derive(Debug, Default, Clone)]
pub struct KubeWorkerConfig {
    pub kubeconfig: Option<PathBuf>,
    pub target_namespaces: Option<TargetNamespaces>,
    pub context: Option<String>,
    pub all_namespaces: bool,
    pub fallback_namespaces: Option<Vec<String>>,
    pub pod_config: PodConfig,
    pub event_config: EventConfig,
    pub api_config: ApiConfig,
    pub apis_config: ApisConfig,
    pub yaml_config: YamlConfig,
}
```

**Step 2: `app.rs` で `Config.fallback_namespaces` を重複除去して `KubeWorkerConfig` に渡す**

`src/app.rs` の `App::run` メソッド内、`kube_worker_config.yaml_config = ...` の行の後に追加:

```rust
kube_worker_config.fallback_namespaces = config.fallback_namespaces.map(|namespaces| {
    let mut seen = std::collections::HashSet::new();
    namespaces
        .into_iter()
        .filter(|ns| seen.insert(ns.clone()))
        .collect()
});
```

空の `Vec` は `None` と同じ扱い（フォールバックなし）にするため、空チェックも行う:

```rust
kube_worker_config.fallback_namespaces = config.fallback_namespaces.and_then(|namespaces| {
    let mut seen = std::collections::HashSet::new();
    let deduped: Vec<String> = namespaces
        .into_iter()
        .filter(|ns| seen.insert(ns.clone()))
        .collect();
    if deduped.is_empty() {
        None
    } else {
        Some(deduped)
    }
});
```

**Step 3: ビルドして確認**

Run: `cargo build 2>&1 | head -20`
Expected: コンパイル成功（`KubeController::new` で未使用の destructure warning が出る可能性あり）

**Step 4: コミット**

```bash
git add src/workers/kube/config.rs src/app.rs
git commit -m "feat: wire fallback_namespaces from Config to KubeWorkerConfig"
```

---

### Task 4: `NamespaceResponse::GetFallback` バリアントを追加

**Files:**
- Modify: `src/features/namespace/message.rs`

**Step 1: 新しいバリアントを追加**

`src/features/namespace/message.rs` の `NamespaceResponse` に `GetFallback` を追加:

```rust
#[derive(Debug)]
pub enum NamespaceResponse {
    Get(Result<TargetNamespaces>),
    GetFallback(TargetNamespaces),
    Set(TargetNamespaces),
}
```

**Step 2: ビルドして確認**

Run: `cargo build 2>&1 | head -30`
Expected: `NamespaceResponse::GetFallback` に対する match の非網羅性エラーが `action.rs` で出る。これは次のタスクで修正するので、この時点では想定通り。

**Step 3: コミット**

```bash
git add src/features/namespace/message.rs
git commit -m "feat: add NamespaceResponse::GetFallback variant"
```

---

### Task 5: `KubeController` にフォールバックロジックを実装

**Files:**
- Modify: `src/workers/kube/controller.rs:138-149` (KubeController構造体)
- Modify: `src/workers/kube/controller.rs:151-205` (KubeController::new)
- Modify: `src/workers/kube/controller.rs:207-215` (KubeController::run の destructure)
- Modify: `src/workers/kube/controller.rs:491-496` (NamespaceRequest::Get ハンドラ)

**Step 1: `KubeController` 構造体に `fallback_namespaces` を追加**

`src/workers/kube/controller.rs` の `KubeController` 構造体:

```rust
pub struct KubeController {
    tx: Sender<Message>,
    rx: Receiver<Message>,
    kubeconfig: Kubeconfig,
    context: String,
    store: KubeStore,
    fallback_namespaces: Option<Vec<String>>,
    pod_config: PodConfig,
    event_config: EventConfig,
    api_config: ApiConfig,
    apis_config: ApisConfig,
    yaml_config: YamlConfig,
}
```

**Step 2: `KubeController::new` で `fallback_namespaces` を受け取り保持**

`new` メソッドの destructure に `fallback_namespaces` を追加し、`Ok(Self { ... })` に含める:

```rust
let KubeWorkerConfig {
    kubeconfig,
    target_namespaces,
    context,
    all_namespaces,
    fallback_namespaces,
    pod_config,
    event_config,
    api_config,
    apis_config,
    yaml_config,
} = config;
```

```rust
Ok(Self {
    tx,
    rx,
    kubeconfig,
    context: context.to_string(),
    store,
    fallback_namespaces,
    pod_config,
    event_config,
    api_config,
    apis_config,
    yaml_config,
})
```

**Step 3: `run` メソッドの destructure に追加**

```rust
let Self {
    tx,
    rx,
    kubeconfig,
    mut context,
    mut store,
    fallback_namespaces,
    pod_config,
    ...
```

**Step 4: `NamespaceRequest::Get` ハンドラにフォールバックロジックを実装**

重複除去は `app.rs` で済んでいるため、ここではフォールバックの分岐のみ:

```rust
NamespaceRequest::Get => {
    let ns = fetch_all_namespaces(kube_client.clone()).await;
    match ns {
        Ok(namespaces) => {
            tx.send(NamespaceResponse::Get(Ok(namespaces)).into())
                .expect("Failed to send NamespaceResponse::Get");
        }
        Err(err) => {
            match &fallback_namespaces {
                Some(fb) => {
                    tx.send(NamespaceResponse::GetFallback(fb.clone()).into())
                        .expect("Failed to send NamespaceResponse::GetFallback");
                }
                None => {
                    tx.send(NamespaceResponse::Get(Err(err)).into())
                        .expect("Failed to send NamespaceResponse::Get");
                }
            }
        }
    }
}
```

**Step 5: ビルドして確認**

Run: `cargo build 2>&1 | head -30`
Expected: `action.rs` の match 非網羅性エラーのみ残る

**Step 6: コミット**

```bash
git add src/workers/kube/controller.rs
git commit -m "feat: implement namespace fallback logic in KubeController"
```

---

### Task 6: SelectForm にリストタイトル更新メソッドを追加

**Files:**
- Modify: `src/ui/widget/single_select/select.rs:102-109` (SelectForm impl)
- Modify: `src/ui/widget/single_select.rs` (SingleSelect impl)
- Modify: `src/ui/widget/multiple_select/select.rs:142-153` (SelectForm impl)
- Modify: `src/ui/widget/multiple_select.rs` (MultipleSelect impl)

**Step 1: SingleSelect の SelectForm にタイトル更新メソッドを追加**

`src/ui/widget/single_select/select.rs` の `SelectForm` impl ブロックに追加:

```rust
pub fn update_items_title(&mut self, title: impl Into<String>) {
    *self.list_widget.widget_base_mut().title_mut() = title.into().into();
}
```

**Step 2: SingleSelect にタイトル更新メソッドを公開**

`src/ui/widget/single_select.rs` の `SingleSelect` impl ブロックに追加:

```rust
pub fn update_items_title(&mut self, title: impl Into<String>) {
    self.select_form.update_items_title(title);
}
```

**Step 3: MultipleSelect の SelectForm にタイトル更新メソッドを追加**

`src/ui/widget/multiple_select/select.rs` の `SelectForm` impl ブロックに追加:

```rust
pub fn update_items_title(&mut self, title: impl Into<String>) {
    *self.unselected_widget.widget_base_mut().title_mut() = title.into().into();
}
```

注意: MultipleSelect の SelectForm では `list_widget` ではなく `unselected_widget` がItemsリストに対応する。

**Step 4: MultipleSelect にタイトル更新メソッドを公開**

`src/ui/widget/multiple_select.rs` の `MultipleSelect` impl ブロックに追加:

```rust
pub fn update_items_title(&mut self, title: impl Into<String>) {
    self.select_form.update_items_title(title);
}
```

**Step 5: `WidgetTrait` にメソッドを追加**

`src/ui/widget.rs` の `WidgetTrait` に追加:

```rust
fn update_items_title(&mut self, _title: &str) {}
```

`enum_dispatch` のデフォルト実装の制約に注意。もしデフォルト実装が使えない場合は、List, Text, Table, Input, CheckList の各型に空の実装を追加する:

```rust
fn update_items_title(&mut self, _title: &str) {}
```

SingleSelect と MultipleSelect では既存の `pub fn update_items_title` を trait の実装として接続する:

```rust
fn update_items_title(&mut self, title: &str) {
    self.select_form.update_items_title(title);
}
```

**Step 6: ビルドして確認**

Run: `cargo build 2>&1 | head -30`
Expected: `action.rs` の match 非網羅性エラーのみ残る

**Step 7: コミット**

```bash
git add src/ui/widget.rs src/ui/widget/single_select.rs src/ui/widget/single_select/select.rs src/ui/widget/multiple_select.rs src/ui/widget/multiple_select/select.rs
git commit -m "feat: add update_items_title method to select widgets"
```

---

### Task 7: Render action でフォールバックレスポンスを処理

**Files:**
- Modify: `src/workers/render/action.rs:239-267`

**Step 1: `NamespaceResponse::GetFallback` のハンドラを追加し、成功時にタイトルをリセット**

`src/workers/render/action.rs` の namespace レスポンス処理を変更:

```rust
Kube::Namespace(NamespaceMessage::Response(res)) => match res {
    NamespaceResponse::Get(res) => match res {
        Ok(namespaces) => {
            window
                .find_widget_mut(MULTIPLE_NAMESPACES_DIALOG_ID)
                .update_widget_item(Item::Array(
                    namespaces.iter().cloned().map(LiteralItem::from).collect(),
                ));
            window
                .find_widget_mut(MULTIPLE_NAMESPACES_DIALOG_ID)
                .update_items_title("Items");
            window
                .find_widget_mut(SINGLE_NAMESPACE_DIALOG_ID)
                .update_widget_item(Item::Array(
                    namespaces.iter().cloned().map(LiteralItem::from).collect(),
                ));
            window
                .find_widget_mut(SINGLE_NAMESPACE_DIALOG_ID)
                .update_items_title("Items");
        }
        Err(err) => {
            let err = error_lines!(err);
            window
                .find_widget_mut(MULTIPLE_NAMESPACES_DIALOG_ID)
                .update_widget_item(Item::Array(err.to_vec()));

            window
                .find_widget_mut(SINGLE_NAMESPACE_DIALOG_ID)
                .update_widget_item(Item::Array(err));
        }
    },
    NamespaceResponse::GetFallback(namespaces) => {
        window
            .find_widget_mut(MULTIPLE_NAMESPACES_DIALOG_ID)
            .update_widget_item(Item::Array(
                namespaces.iter().cloned().map(LiteralItem::from).collect(),
            ));
        window
            .find_widget_mut(MULTIPLE_NAMESPACES_DIALOG_ID)
            .update_items_title("Items (from config)");
        window
            .find_widget_mut(SINGLE_NAMESPACE_DIALOG_ID)
            .update_widget_item(Item::Array(
                namespaces.iter().cloned().map(LiteralItem::from).collect(),
            ));
        window
            .find_widget_mut(SINGLE_NAMESPACE_DIALOG_ID)
            .update_items_title("Items (from config)");
    },
    NamespaceResponse::Set(res) => {
        namespace.update(res);
    }
},
```

**Step 2: ビルドして確認**

Run: `cargo build 2>&1 | head -20`
Expected: コンパイル成功

**Step 3: 全テスト実行**

Run: `cargo test`
Expected: 全テストPASS

**Step 4: コミット**

```bash
git add src/workers/render/action.rs
git commit -m "feat: handle GetFallback response with title update in render"
```

---

### Task 8: example config を更新

**Files:**
- Modify: `example/config.yaml`

**Step 1: `example/config.yaml` に `fallback_namespaces` の例を追加**

ファイル先頭のコメントセクションの後（`theme:` の前）に追加:

```yaml
# fallback_namespaces:
#   Namespaces to show when the Kubernetes API cannot list namespaces
#   (e.g. due to RBAC restrictions). Only used as a fallback.
#   - production
#   - staging
#   - dev

theme:
```

**Step 2: コミット**

```bash
git add example/config.yaml
git commit -m "docs: add fallback_namespaces example to config.yaml"
```

---

### Task 9: 最終確認

**Step 1: clippy チェック**

Run: `cargo clippy 2>&1 | head -30`
Expected: 新規の warning なし

**Step 2: 全テスト実行**

Run: `cargo test`
Expected: 全テストPASS

**Step 3: ビルド確認**

Run: `cargo build --release 2>&1 | head -20`
Expected: コンパイル成功
