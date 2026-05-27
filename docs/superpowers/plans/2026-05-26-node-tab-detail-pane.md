# Node 詳細ペイン Implementation Plan (Plan 4 of #920)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Node 一覧で選択中のノードについて、Node YAML（`metadata.managedFields` を除去）と関連 Pod（そのノード上の全 namespace の Pod）を 1 つの Text ウィジェットに表示する詳細ペインを追加し、3 秒間隔で自動更新する。

**Architecture:** Network の `NetworkDescriptionWorker`（`request 駆動 InfiniteWorker`、`INTERVAL=3`、選択ごとに前ワーカーを abort して新規 spawn）に倣う。新規 `NodeDetailWorker` を `src/features/node/kube/detail.rs` に追加し、`Kube::NodeDetail(NodeDetailMessage)` でメッセージを流す。`NodeTab` を縦 2 ペイン化（一覧 + Text）。一覧の `on_select` でノード名を `NodeDetailMessage::Request{name}` として送信し、controller がワーカーを spawn／abort、ワーカーの `NodeDetailMessage::Response(Result<Vec<String>>)` を render が Text ウィジェットへ反映。

**Tech Stack:** tokio (interval/abort), kube-rs (`Resource::url_path`), k8s-openapi (`Node`, `Pod`), serde_json/serde_yaml（managedFields 除去・YAML 整形）, ratatui Text widget。

**前提:** Plan 1〜3 完了（`920-node-label-columns` = PR #975）。新規ブランチ `920-node-detail-pane` を `920-node-label-columns` 上にスタック。

**設計スペック:** `docs/superpowers/specs/2026-05-22-node-tab-design.md` の「UI / レイアウト（案1: コンパクト 2 ペイン）」「データフロー > 詳細（NodeDetailWorker）」節に準拠。

---

## ファイル構成

**新規:**
- `src/features/node/kube/detail.rs` — `NodeDetailWorker`（`InfiniteWorker`）と整形ヘルパ。
- `src/features/node/view/widgets/detail.rs` — 詳細用 Text ウィジェット。

**変更:**
- `src/features/node/message.rs` — `NodeDetailMessage { Request{name}, Response(Result<Vec<String>>) }` と `From<…> for Message`。
- `src/features/node/kube.rs` — `pub use detail::NodeDetailWorker;` を追加。
- `src/features/node/view/widgets.rs` — `pub use detail::node_detail_widget;`。
- `src/features/node/view/tab.rs` — `NodeTab` を 2 ペイン化（縦分割: 一覧/詳細）。`node_widget` の `on_select` で `NodeDetailMessage::Request{name}` を送る。
- `src/features/component_id.rs` — `NODE_DETAIL_WIDGET_ID` 追加。
- `src/workers/kube/message.rs` — `Kube::NodeDetail(NodeDetailMessage)` バリアント追加。
- `src/workers/kube/controller.rs` — `Kube::NodeDetail(Request)` で `NodeDetailWorker` を spawn／前回 handle を abort（Network と同型）。
- `src/workers/render/action.rs` — `Kube::NodeDetail(Response)` ハンドラ追加。

**触らない:**
- `NodePoller`、`NodeColumns` 周り、列ダイアログ、設定（このプランの範囲外）。

---

## Task 1: メッセージ型・コンポーネント ID（土台）

**Files:** `src/features/node/message.rs`, `src/features/component_id.rs`, `src/workers/kube/message.rs`

- [ ] **Step 1: 失敗テストを書く**（`src/features/node/message.rs` の `#[cfg(test)] mod tests`）。

```rust
#[test]
fn node_detail_message_request_into_kube() {
    let msg: crate::message::Message = NodeDetailMessage::Request {
        name: "node-a".to_string(),
    }
    .into();
    assert!(matches!(
        msg,
        crate::message::Message::Kube(crate::workers::kube::Kube::NodeDetail(
            NodeDetailMessage::Request { .. }
        ))
    ));
}
```

- [ ] **Step 2: 失敗確認** — `cargo test features::node::message` → コンパイルエラー（`NodeDetailMessage` 未定義）。

- [ ] **Step 3: 実装**（`message.rs`）。

```rust
#[derive(Debug, Clone)]
pub enum NodeDetailMessage {
    Request { name: String },
    Response(anyhow::Result<Vec<String>>),
}

impl From<NodeDetailMessage> for crate::message::Message {
    fn from(m: NodeDetailMessage) -> Self {
        crate::message::Message::Kube(crate::workers::kube::Kube::NodeDetail(m))
    }
}
```

ただし `anyhow::Result` は `Clone` でないので、`NodeMessage` と同じく `enum` 自体を `Clone` 派生せず `Debug` のみにする：

```rust
#[derive(Debug)]
pub enum NodeDetailMessage {
    Request { name: String },
    Response(anyhow::Result<Vec<String>>),
}
```

（`NodeMessage` の Plan 1 実装と合わせる。`Clone` が不要なことを確認。）

- [ ] **Step 4: `Kube` 拡張**（`src/workers/kube/message.rs`）。

```rust
// 既存の `Kube::Node(NodeMessage)` の隣に追加
NodeDetail(crate::features::node::message::NodeDetailMessage),
```

- [ ] **Step 5: `component_id` 追加**（`src/features/component_id.rs`）。

```rust
pub const NODE_DETAIL_WIDGET_ID: &str = "node-detail-widget";
```

- [ ] **Step 6: ビルド・テスト** — `cargo build` → green、`cargo test features::node::message` → PASS。

- [ ] **Step 7: コミット**

```bash
git add -A
git commit -m "feat(node): add NodeDetailMessage and detail widget id"
```

---

## Task 2: `NodeDetailWorker` 雛形（fetch とフォーマット骨格、TDD）

**Files:** `src/features/node/kube/detail.rs`（新規）, `src/features/node/kube.rs`

- [ ] **Step 1: モジュール登録**（`src/features/node/kube.rs`）

```rust
mod detail;
mod node;
// ... existing exports ...
pub use detail::NodeDetailWorker;
```

**実装方針:** kube-rs の `Api<K>` + `ListParams` を使う（既存の `src/features/pod/kube/log/log_streamer.rs`、`pod_watcher.rs`、`src/cmd/subcommand.rs` と同じパターン）。URL を手組みしない代わりに `KubeClientRequest` の mock では fetch 全体を検証できないため、**フォーマット関数を純粋関数として分離**してそこを単体テストする。`fetch_for` は薄いラッパで実機 Task 7 で確認する。

- [ ] **Step 2: 失敗テストを書く**（`detail.rs` の `#[cfg(test)] mod tests`）。純粋関数 `strip_and_serialize_node` と `format_related_pods` を検証。

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use k8s_openapi::{
        api::core::v1::{Node, Pod, PodStatus},
        apimachinery::pkg::apis::meta::v1::{ManagedFieldsEntry, ObjectMeta, Time},
        chrono::Utc,
    };
    use kube::core::ObjectList;
    use pretty_assertions::assert_eq;
    use std::collections::BTreeMap;

    fn sample_node_with_managed_fields() -> Node {
        Node {
            metadata: ObjectMeta {
                name: Some("node-a".to_string()),
                labels: Some(BTreeMap::from([(
                    "role".to_string(),
                    "worker".to_string(),
                )])),
                managed_fields: Some(vec![ManagedFieldsEntry {
                    manager: Some("kubelet".to_string()),
                    time: Some(Time(Utc::now())),
                    ..Default::default()
                }]),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn sample_pod(ns: &str, name: &str, phase: &str) -> Pod {
        Pod {
            metadata: ObjectMeta {
                namespace: Some(ns.to_string()),
                name: Some(name.to_string()),
                ..Default::default()
            },
            status: Some(PodStatus {
                phase: Some(phase.to_string()),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    fn pod_list(items: Vec<Pod>) -> ObjectList<Pod> {
        ObjectList {
            metadata: Default::default(),
            items,
            types: Default::default(),
        }
    }

    #[test]
    fn strip_and_serialize_node_removes_managed_fields() {
        let node = sample_node_with_managed_fields();
        let lines = strip_and_serialize_node(node).unwrap();
        let joined = lines.join("\n");

        assert!(joined.contains("name: node-a"));
        assert!(joined.contains("role: worker"));
        assert!(!joined.contains("managedFields"));
        assert!(!joined.contains("kubelet"));
    }

    #[test]
    fn format_related_pods_yields_one_row_per_pod() {
        let list = pod_list(vec![
            sample_pod("ns1", "pod-a", "Running"),
            sample_pod("ns2", "pod-b", "Pending"),
        ]);

        let rows = format_related_pods(&list);

        assert_eq!(
            rows,
            vec![
                "# ns1  pod-a  Running".to_string(),
                "# ns2  pod-b  Pending".to_string(),
            ]
        );
    }

    #[test]
    fn format_related_pods_empty_list_returns_empty() {
        let list = pod_list(vec![]);
        assert!(format_related_pods(&list).is_empty());
    }
}
```

注: `kube::core::ObjectList` のフィールド構成は kube-rs のバージョンに依存する。実装時にコンパイラのエラーで確認して構築方法を合わせる（`types` フィールドが無い場合などは省略）。

- [ ] **Step 3: 失敗確認** — `cargo test features::node::kube::detail` → コンパイルエラー（関数未定義）。

- [ ] **Step 4: 実装**（`detail.rs`）。kube-rs の `Api<K>` を使ってリソース取得、純粋関数で整形。

```rust
use anyhow::{Context, Result};
use crossbeam::channel::Sender;
use k8s_openapi::api::core::v1::{Node, Pod};
use kube::{api::ListParams, core::ObjectList, Api};

use crate::{kube::KubeClientRequest, message::Message};

const INTERVAL: u64 = 3;

pub struct NodeDetailWorker<C> {
    tx: Sender<Message>,
    client: C,
    name: String,
}

impl<C> NodeDetailWorker<C>
where
    C: KubeClientRequest + Send + Sync + 'static,
{
    pub fn new(tx: Sender<Message>, client: C, name: String) -> Self {
        Self { tx, client, name }
    }

    /// Fetch Node + related Pods and combine into a single line array.
    /// The fetch is a thin delegation to `kube::Api` (matches log_streamer /
    /// pod_watcher in this codebase); the pure formatters below are what the
    /// unit tests target.
    pub async fn fetch_for(name: &str, client: &C) -> Result<Vec<String>> {
        let kube_client = client.client().clone();

        // 1) Node: typed get via kube::Api.
        let node_api: Api<Node> = Api::all(kube_client.clone());
        let node = node_api
            .get(name)
            .await
            .with_context(|| format!("failed to fetch node {}", name))?;
        let mut lines = strip_and_serialize_node(node)?;

        // 2) Related Pods: typed list across all namespaces with field selector.
        let pod_api: Api<Pod> = Api::all(kube_client);
        let lp = ListParams::default().fields(&format!("spec.nodeName={}", name));
        let pods = pod_api
            .list(&lp)
            .await
            .with_context(|| format!("failed to list pods on node {}", name))?;

        let pod_rows = format_related_pods(&pods);
        if !pod_rows.is_empty() {
            lines.push("---".to_string());
            lines.push(format!("# Related Pods (spec.nodeName={})", name));
            lines.push("# NAMESPACE  NAME  STATUS".to_string());
            lines.extend(pod_rows);
        }

        Ok(lines)
    }
}

/// Strip `metadata.managedFields` and serialize the Node as YAML lines.
fn strip_and_serialize_node(mut node: Node) -> Result<Vec<String>> {
    node.metadata.managed_fields = None;
    let yaml = serde_yaml::to_string(&node)
        .with_context(|| "failed to serialize node as YAML")?;
    Ok(yaml.lines().map(String::from).collect())
}

/// Format the related-Pods list as `# <ns>  <name>  <phase>` rows.
fn format_related_pods(pods: &ObjectList<Pod>) -> Vec<String> {
    pods.items
        .iter()
        .map(|pod| {
            let ns = pod.metadata.namespace.as_deref().unwrap_or("");
            let name = pod.metadata.name.as_deref().unwrap_or("");
            let phase = pod
                .status
                .as_ref()
                .and_then(|s| s.phase.as_deref())
                .unwrap_or("");
            format!("# {}  {}  {}", ns, name, phase)
        })
        .collect()
}
```

- [ ] **Step 5: テスト・ビルド** — `cargo test features::node::kube::detail` → PASS、`cargo build` → green。

- [ ] **Step 6: コミット**

```bash
git add -A
git commit -m "feat(node): NodeDetailWorker fetch_for via kube::Api (typed get/list)"
```

---

## Task 3: `InfiniteWorker` 実装＋ controller spawn／abort

**Files:** `src/features/node/kube/detail.rs`, `src/workers/kube/controller.rs`

- [ ] **Step 1: `InfiniteWorker` 実装**（`detail.rs` に追加）。Network の `description.rs` と同じく、外側の `run` で最終エラーをログし、内側のループ helper で `?` を使う。

```rust
use crate::{logger, workers::kube::InfiniteWorker};

#[async_trait::async_trait]
impl<C> InfiniteWorker for NodeDetailWorker<C>
where
    C: KubeClientRequest + Send + Sync + 'static,
{
    async fn run(&self) {
        if let Err(e) = self.fetch_loop().await {
            logger!(error, "node detail worker exited: {:?}", e);
        }
    }
}

impl<C> NodeDetailWorker<C>
where
    C: KubeClientRequest + Send + Sync + 'static,
{
    async fn fetch_loop(&self) -> Result<()> {
        let mut interval =
            tokio::time::interval(std::time::Duration::from_secs(INTERVAL));

        loop {
            interval.tick().await;

            let result = Self::fetch_for(&self.name, &self.client).await;

            self.tx
                .send(
                    crate::features::node::message::NodeDetailMessage::Response(result).into(),
                )?;
        }
    }
}
```

毎ティックの fetch エラーは `Response(Result<_>)` で送られるためループは継続。`tx.send` が失敗（受信側 drop）したときだけループを抜ける（Network と同型）。

- [ ] **Step 2: controller wire**（`src/workers/kube/controller.rs`、Network ハンドラの隣）

```rust
// Kube::Node(NodeMessage::Request(...)) の隣に追加
Kube::NodeDetail(NodeDetailMessage::Request { name }) => {
    if let Some(handler) = node_detail_handler {
        handler.abort();
    }
    node_detail_handler = Some(
        NodeDetailWorker::new(tx.clone(), kube_client.clone(), name).spawn(),
    );
    task::yield_now().await;
}
```

`node_detail_handler: Option<tokio::task::AbortHandle>` をループ前で `None` として宣言（Network の `network_handler` と同型）。imports に `NodeDetailWorker` と `NodeDetailMessage` を追加。

- [ ] **Step 3: テスト・ビルド** — `cargo test` → 全 PASS、`cargo build` → green。

- [ ] **Step 4: コミット**

```bash
git add -A
git commit -m "feat(node): wire NodeDetailWorker spawn/abort in controller"
```

---

## Task 4: 詳細 Text ウィジェット＋ NodeTab を 2 ペイン化

**Files:** `src/features/node/view/widgets/detail.rs`（新規）, `src/features/node/view/widgets.rs`, `src/features/node/view/tab.rs`, `src/workers/render/window.rs`

- [ ] **Step 1: Text ウィジェット**（`detail.rs`）。`src/features/network/view/widgets/description.rs` と同じ構成（`Text` + `SearchForm` + `block_injection` でスクロール位置をタイトルに表示 + `clipboard` でコピー）。

```rust
use std::{cell::RefCell, rc::Rc};

use ratatui::widgets::Block;

use crate::{
    clipboard::Clipboard,
    config::theme::WidgetThemeConfig,
    features::component_id::NODE_DETAIL_WIDGET_ID,
    ui::widget::{
        SearchForm,
        SearchFormTheme,
        Text,
        TextTheme,
        Widget,
        WidgetBase,
        WidgetTheme,
        WidgetTrait as _,
    },
};

pub fn node_detail_widget(
    clipboard: &Option<Rc<RefCell<Clipboard>>>,
    theme: WidgetThemeConfig,
) -> Widget<'static> {
    let widget_theme = WidgetTheme::from(theme.clone());
    let search_theme = SearchFormTheme::from(theme.clone());
    let text_theme = TextTheme::from(theme);

    let widget_base = WidgetBase::builder()
        .title("Node Detail")
        .theme(widget_theme)
        .build();

    let search_form = SearchForm::builder().theme(search_theme).build();

    let builder = Text::builder()
        .id(NODE_DETAIL_WIDGET_ID)
        .widget_base(widget_base)
        .search_form(search_form)
        .theme(text_theme)
        .block_injection(block_injection());

    if let Some(cb) = clipboard {
        builder.clipboard(cb.clone())
    } else {
        builder
    }
    .build()
    .into()
}

fn block_injection() -> impl Fn(&Text, bool, bool) -> Block<'static> {
    |text: &Text, is_active: bool, is_mouse_over: bool| {
        let (index, size) = text.state();
        let mut base = text.widget_base().clone();
        *base.title_mut() = format!("Node Detail [{}/{}]", index, size).into();
        base.render_block(text.can_activate() && is_active, is_mouse_over)
    }
}
```

- [ ] **Step 2: re-export**（`widgets.rs`）。

```rust
mod detail;
// existing...
pub use detail::node_detail_widget;
```

- [ ] **Step 3: NodeTab を 2 ペイン化**（`tab.rs`）。`split_direction` と `clipboard` を受け取る（Pod/Network と同様）。`node_widget` には `tx` を渡せるよう拡張（次タスク）。

```rust
pub fn new(
    title: &'static str,
    tx: &Sender<Message>,
    clipboard: &Option<Rc<RefCell<Clipboard>>>,
    split_direction: Direction,
    default_columns: Option<NodeColumns>,
    label_registry: Vec<NodeLabelColumn>,
    theme: WidgetThemeConfig,
) -> Self {
    let node_widget = node_widget(tx.clone(), theme.clone());
    let detail_widget = node_detail_widget(clipboard, theme.clone());
    let node_columns_dialog = node_columns_dialog(tx, default_columns, label_registry, theme);

    let tab = Tab::new(
        NODE_TAB_ID,
        title,
        [node_widget, detail_widget],
        TabLayout::new(layout, split_direction),
    );
    // ...
}

fn layout(split_direction: Direction) -> NestedWidgetLayout {
    // 既存の Pod/Network/Network description タブの layout 関数のうち
    // 2 ペイン構成の実装（例: src/features/network/view/tab.rs 内）に倣う。
    // 正確な NestedWidgetLayout の API は実装時に既存コードで確認する。
}
```

- [ ] **Step 4: `WindowInit::tabs_dialogs`** — `NodeTab::new` に `&clipboard`、`self.split_mode` を渡すよう更新（Pod/Network 呼び出しの真似）。

- [ ] **Step 5: テスト・ビルド** — `cargo test` → green、`cargo build` → green。

- [ ] **Step 6: コミット**

```bash
git add -A
git commit -m "feat(node): add detail Text widget (search/clipboard/scroll title) and 2-pane NodeTab"
```

---

## Task 5: 一覧の `on_select` で `NodeDetailMessage::Request` を送る

**Files:** `src/features/node/view/widgets/node.rs`

**前提（実装時に確認）:** `KubeTableRow` には `pub name: String` フィールドがあり、`src/workers/render/action.rs` がそれを `TableItem.metadata["name"]` として埋めている（Pod・Network 等で既に動いている既存の経路）。Plan 1 で実装した `get_node_table` も `KubeTableRow.name` を設定済みのため、**poller の変更は不要**（前バージョンのプランで「poller の metadata に name を追加」と書いていたが、これは action.rs が既に行っているので二重作業だった）。

- [ ] **Step 1: 失敗テストを書く**（`node.rs` の `#[cfg(test)] mod tests`）。`on_select` クロージャは Window を要するため、**選択名から `NodeDetailMessage::Request` を作る純粋関数 `build_detail_request`** に切り出し、それを直接テストする（Plan 3 の `collect_columns` と同じ責務分離）。

```rust
#[test]
fn build_detail_request_from_table_item_name_metadata() {
    use crate::ui::widget::TableItem;
    let item = TableItem {
        item: vec!["node-a".to_string()],
        metadata: Some(std::collections::BTreeMap::from([
            ("name".to_string(), "node-a".to_string()),
        ])),
    };

    let req = build_detail_request(&item).expect("name metadata should be present");
    match req {
        NodeDetailMessage::Request { name } => assert_eq!(name, "node-a"),
        _ => panic!("expected Request"),
    }
}

#[test]
fn build_detail_request_returns_none_without_name() {
    use crate::ui::widget::TableItem;
    let item = TableItem { item: vec![], metadata: None };
    assert!(build_detail_request(&item).is_none());
}
```

- [ ] **Step 2: 失敗確認** — `cargo test features::node::view::widgets::node` → コンパイルエラー（`build_detail_request` 未定義）。

- [ ] **Step 3: 実装**（`node.rs`）。`node_widget` シグネチャに `tx: Sender<Message>` を追加し、Table builder に `on_select` を渡す。Network の on_select の慣習に倣い `widget_clear` ＋ `append_title_mut` も行う。

```rust
pub fn node_widget(tx: Sender<Message>, theme: WidgetThemeConfig) -> Widget<'static> {
    // ...既存の builder
        .on_select(on_select(tx))
    // ...
}

fn build_detail_request(item: &TableItem) -> Option<NodeDetailMessage> {
    let name = item.metadata.as_ref()?.get("name")?.clone();
    Some(NodeDetailMessage::Request { name })
}

fn on_select(tx: Sender<Message>) -> impl Fn(&mut Window, &TableItem) -> EventResult {
    move |w: &mut Window, item: &TableItem| {
        // 選択変更時は前ノードの詳細をクリアして、再フェッチを待つ。
        w.widget_clear(NODE_DETAIL_WIDGET_ID);

        let Some(req) = build_detail_request(item) else {
            return EventResult::Ignore;
        };

        // タイトルに選択中のノード名を表示（Network description と同じ慣習）。
        if let NodeDetailMessage::Request { name } = &req {
            *(w.find_widget_mut(NODE_DETAIL_WIDGET_ID)
                .widget_base_mut()
                .append_title_mut()) = Some((format!(" : {}", name)).into());
        }

        tx.send(req.into())
            .expect("Failed to send NodeDetailMessage::Request");
        EventResult::Nop
    }
}
```

- [ ] **Step 4: テスト・ビルド** — `cargo test features::node` → PASS、`cargo build` → green。

- [ ] **Step 5: コミット**

```bash
git add -A
git commit -m "feat(node): request node detail on row selection (clear + title)"
```

---

## Task 6: `action.rs` で Response を受けて Text ウィジェット更新

**Files:** `src/workers/render/action.rs`

- [ ] **Step 1: ハンドラ追加**（既存の `Kube::Node(NodeMessage::Poll(...))` の隣）

```rust
Kube::NodeDetail(NodeDetailMessage::Response(result)) => match result {
    Ok(lines) => {
        window.clear_widget_error(NODE_DETAIL_WIDGET_ID);
        let widget = window.find_widget_mut(NODE_DETAIL_WIDGET_ID);
        widget.update_widget_item(Item::Array(
            lines.into_iter().map(LiteralItem::from).collect(),
        ));
    }
    Err(e) => {
        window.set_widget_error(NODE_DETAIL_WIDGET_ID, &e);
    }
},
```

import に `NODE_DETAIL_WIDGET_ID` と `NodeDetailMessage` を追加。

- [ ] **Step 2: ビルド・テスト** — `cargo build` → green、`cargo test` → 全 PASS。

- [ ] **Step 3: コミット**

```bash
git add -A
git commit -m "feat(node): render NodeDetail response in detail widget"
```

---

## Task 7: 仕上げ（fmt / clippy / 全テスト ＋ 実機）

- [ ] `cargo +nightly fmt` （差分が出れば反映）
- [ ] `cargo clippy --all-targets` — 新規警告なし
- [ ] `cargo test` — 全 PASS
- [ ] 実機 `cargo run -- --config-file example/config.yaml`:
  - Node タブ（キー 5）を開き、一覧で行を移動 → 詳細ペインに対象ノードの YAML（managedFields なし）が出る
  - 関連 Pod がいるノードでは末尾に `# Related Pods` セクションが出る、いないノードでは出ない
  - 3 秒後に自動更新される（タイムスタンプ的な値で確認）
  - 別のノードに移動すると即時に内容が切り替わる
- [ ] fmt 差分があればコミット

---

## 後続プラン

- **Plan 5**: フィルタ（`node:`/`!node:`/`label:`、nom パーサ、`shared_node_filter`、`labelSelector`、フィルタ入力ウィジェット＋ヘルプダイアログ）。
