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

- [ ] **Step 2: 失敗テストを書く**（`detail.rs` の `#[cfg(test)] mod tests`）。Network の description impl テスト（例: `ingress.rs`, `pod.rs`）と同じく **k8s_openapi の型付きレスポンス**を mock に返させる。

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use k8s_openapi::{
        api::core::v1::{Node, Pod, PodStatus},
        apimachinery::pkg::apis::meta::v1::{ManagedFieldsEntry, ObjectMeta, Time},
        chrono::Utc,
        List,
    };
    use mockall::predicate::eq;
    use std::collections::BTreeMap;

    fn sample_node() -> Node {
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

    #[tokio::test]
    async fn strips_managed_fields_from_node_yaml() {
        use crate::kube::mock::MockTestKubeClient;
        let mut client = MockTestKubeClient::new();
        let node = sample_node();
        let empty_pods: List<Pod> = List { items: vec![], ..Default::default() };

        mock_expect!(
            client,
            request,
            [
                (
                    Node,
                    eq("/api/v1/nodes/node-a".to_string()),
                    Ok(node)
                ),
                (
                    List<Pod>,
                    eq("/api/v1/pods?fieldSelector=spec.nodeName=node-a".to_string()),
                    Ok(empty_pods)
                )
            ]
        );

        let lines = NodeDetailWorker::fetch_for("node-a", &client).await.unwrap();
        let joined = lines.join("\n");

        assert!(joined.contains("name: node-a"));
        assert!(joined.contains("role: worker"));
        assert!(!joined.contains("managedFields"));
        // 関連 Pod 0 件のときは Related Pods セクションを出さない（spec 通り）
        assert!(!joined.contains("Related Pods"));
    }

    #[tokio::test]
    async fn lists_related_pods_when_present() {
        use crate::kube::mock::MockTestKubeClient;
        let mut client = MockTestKubeClient::new();
        let node = Node {
            metadata: ObjectMeta { name: Some("node-a".to_string()), ..Default::default() },
            ..Default::default()
        };
        let pods: List<Pod> = List {
            items: vec![
                sample_pod("ns1", "pod-a", "Running"),
                sample_pod("ns2", "pod-b", "Pending"),
            ],
            ..Default::default()
        };

        mock_expect!(
            client,
            request,
            [
                (Node, eq("/api/v1/nodes/node-a".to_string()), Ok(node)),
                (
                    List<Pod>,
                    eq("/api/v1/pods?fieldSelector=spec.nodeName=node-a".to_string()),
                    Ok(pods)
                )
            ]
        );

        let lines = NodeDetailWorker::fetch_for("node-a", &client).await.unwrap();
        let joined = lines.join("\n");

        assert!(joined.contains("# Related Pods"));
        assert!(joined.contains("ns1") && joined.contains("pod-a") && joined.contains("Running"));
        assert!(joined.contains("ns2") && joined.contains("pod-b") && joined.contains("Pending"));
    }
}
```

注: `mock_expect!` の正確な型付き呼び出し構文は実装時に既存テスト（例: `src/features/network/kube/description/` の各 fetch テスト、もしくは `src/features/network/kube/network.rs` の poller テスト）で確認し、それに合わせる。重要なのは **mock が型付き値（`Node` / `List<Pod>`）を返し、`fetch_for` がそれを直接受け取る**こと。

- [ ] **Step 3: 失敗確認** — `cargo test features::node::kube::detail` → コンパイルエラー（`NodeDetailWorker` 未定義）。

- [ ] **Step 4: 実装**（`detail.rs`）。**k8s_openapi 型付きで取得**し、typed フィールドアクセスで整形する。Network description impl（`ingress.rs` 等）と同じパターン。

```rust
use anyhow::{Context, Result};
use crossbeam::channel::Sender;
use k8s_openapi::{api::core::v1::{Node, Pod}, List};
use kube::Resource;

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

    /// Pure fetch + format. Tested directly with a mocked client; the
    /// InfiniteWorker `run` (Task 3) just calls this on every tick.
    pub async fn fetch_for(name: &str, client: &C) -> Result<Vec<String>> {
        let mut lines = fetch_node_yaml(name, client).await?;

        let pod_rows = fetch_related_pods(name, client).await?;
        if !pod_rows.is_empty() {
            lines.push("---".to_string());
            lines.push(format!("# Related Pods (spec.nodeName={})", name));
            lines.push("# NAMESPACE  NAME  STATUS".to_string());
            lines.extend(pod_rows);
        }

        Ok(lines)
    }
}

async fn fetch_node_yaml<C>(name: &str, client: &C) -> Result<Vec<String>>
where
    C: KubeClientRequest,
{
    // kube::Resource::url_path で `/api/v1/nodes` を取り、name を付ける。
    let url = format!("{}/{}", Node::url_path(&(), None), name);
    let mut node: Node = client
        .request(&url)
        .await
        .with_context(|| format!("failed to fetch node {}", name))?;

    // 型付きで managedFields を除去。
    node.metadata.managed_fields = None;

    let yaml = serde_yaml::to_string(&node)
        .with_context(|| "failed to serialize node as YAML")?;

    Ok(yaml.lines().map(String::from).collect())
}

async fn fetch_related_pods<C>(node_name: &str, client: &C) -> Result<Vec<String>>
where
    C: KubeClientRequest,
{
    // 全 namespace 横断のクラスタスコープリスト＋fieldSelector で絞り込み。
    let url = format!(
        "{}?fieldSelector=spec.nodeName={}",
        Pod::url_path(&(), None),
        node_name
    );
    let list: List<Pod> = client
        .request(&url)
        .await
        .with_context(|| format!("failed to fetch pods for node {}", node_name))?;

    Ok(list
        .items
        .iter()
        .map(|pod| {
            let ns = pod.metadata.namespace.as_deref().unwrap_or("");
            let name = pod.metadata.name.as_deref().unwrap_or("");
            let status = pod
                .status
                .as_ref()
                .and_then(|s| s.phase.as_deref())
                .unwrap_or("");
            format!("# {}  {}  {}", ns, name, status)
        })
        .collect())
}
```

注: ノード名は DNS-1123 サブドメイン規則（小文字英数・`-`・`.`）で URL 安全な文字のみのため、`fieldSelector` 値の URL エンコードは不要（`format!("...nodeName={}", name)` で良い）。

- [ ] **Step 5: テスト・ビルド** — `cargo test features::node::kube::detail` → PASS、`cargo build` → green。

- [ ] **Step 6: コミット**

```bash
git add -A
git commit -m "feat(node): NodeDetailWorker fetch_for (typed YAML + related pods)"
```

---

## Task 3: `InfiniteWorker` 実装＋ controller spawn／abort

**Files:** `src/features/node/kube/detail.rs`, `src/workers/kube/controller.rs`

- [ ] **Step 1: `InfiniteWorker` 実装**（`detail.rs` に追加）

```rust
use crate::workers::kube::InfiniteWorker;

#[async_trait::async_trait]
impl<C> InfiniteWorker for NodeDetailWorker<C>
where
    C: KubeClientRequest + Send + Sync + 'static,
{
    async fn run(&self) {
        let mut interval =
            tokio::time::interval(std::time::Duration::from_secs(INTERVAL));

        loop {
            interval.tick().await;

            let result = Self::fetch_for(&self.name, &self.client).await;

            if self
                .tx
                .send(
                    crate::features::node::message::NodeDetailMessage::Response(result).into(),
                )
                .is_err()
            {
                break;
            }
        }
    }
}
```

（Network の `description.rs` と同じ骨格。`InfiniteWorker` trait と `spawn()` は既存。）

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

**Files:** `src/features/node/view/widgets/detail.rs`（新規）, `src/features/node/view/widgets.rs`, `src/features/node/view/tab.rs`

- [ ] **Step 1: Text ウィジェット**（`detail.rs`）。Network の description ウィジェットを参考に最小実装。

```rust
use crate::{
    config::theme::WidgetThemeConfig,
    features::component_id::NODE_DETAIL_WIDGET_ID,
    ui::widget::{Text, Widget, WidgetBase, WidgetTheme},
};

pub fn node_detail_widget(theme: WidgetThemeConfig) -> Widget<'static> {
    let widget_theme = WidgetTheme::from(theme.clone());
    let widget_base = WidgetBase::builder()
        .title("Node Detail")
        .theme(widget_theme)
        .build();

    Text::builder()
        .id(NODE_DETAIL_WIDGET_ID)
        .widget_base(widget_base)
        .theme(theme.into())  // Text 用 theme 変換が必要なら適宜
        .build()
        .into()
}
```

（実装時に Text ウィジェットの実際の builder API を `src/features/network/view/widgets/description.rs` で確認し、それに合わせる。）

- [ ] **Step 2: re-export**（`widgets.rs`）。

```rust
mod detail;
// existing...
pub use detail::node_detail_widget;
```

- [ ] **Step 3: NodeTab を 2 ペイン化**（`tab.rs`）。`split_direction` を受け取り、Network/Pod タブと同じく分割方向に追従できるようにする（spec の図は縦分割、ユーザーが `S` で切替可能）。

```rust
pub fn new(
    title: &'static str,
    tx: &Sender<Message>,
    split_direction: Direction,
    default_columns: Option<NodeColumns>,
    label_registry: Vec<NodeLabelColumn>,
    theme: WidgetThemeConfig,
) -> Self {
    let node_widget = node_widget(tx.clone(), theme.clone()); // tx を渡せるよう node_widget も拡張（次タスク）
    let detail_widget = node_detail_widget(theme.clone());
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
    NestedWidgetLayout::default()
        .direction(split_direction)
        .nested_widget_layout([
            NestedLayoutElement(Constraint::Percentage(50), LayoutElement::WidgetIndex(0)),
            NestedLayoutElement(Constraint::Percentage(50), LayoutElement::WidgetIndex(1)),
        ])
}
```

（既存の Pod/Network タブの `layout` 関数を参考に正確な API に合わせる。`NestedWidgetLayout` のメソッド名・引数を実装時に確認。）

- [ ] **Step 4: `WindowInit::tabs_dialogs`** — `NodeTab::new` に `split_direction` を渡すよう更新（Pod/Network と同様）。

- [ ] **Step 5: テスト・ビルド** — `cargo test` → green、`cargo build` → green。

- [ ] **Step 6: コミット**

```bash
git add -A
git commit -m "feat(node): add detail widget and 2-pane NodeTab"
```

---

## Task 5: 一覧の `on_select` で `NodeDetailMessage::Request` を送る

**Files:** `src/features/node/view/widgets/node.rs`

- [ ] **Step 1: 失敗テストを書く**（`node.rs` の `#[cfg(test)] mod tests`）。`on_select` コールバックを直接呼び出して `tx.recv()` で `Kube::NodeDetail(Request{name})` を検証。

```rust
#[test]
fn on_select_sends_node_detail_request() {
    let (tx, rx) = crossbeam::channel::bounded(8);
    // node_widget の構築（tx を渡す形に拡張済み前提）
    let _widget = node_widget(tx, WidgetThemeConfig::default());
    // 実際の on_select 発火は内部 API のためここでは「on_select 関数の戻り値クロージャに
    // TableItem を渡したら Request が送られる」ことを検証する単体ユニットに切り出す。
    // -> 実装側で `fn on_select(tx: Sender<Message>) -> impl Fn(&mut Window, &TableItem) -> EventResult`
    //    のような形に切り出し、それを直接 invoke するテストにする。
}
```

実装の都合上、Window 引数が必要なので素直に呼ぶのは難しい。**実装で `on_select` ハンドラを純粋なヘルパ関数に切り出す**（`fn build_detail_request(item: &TableItem) -> NodeDetailMessage` のような）ことで純粋テストにする。

簡略化したテスト案:

```rust
#[test]
fn build_detail_request_from_table_item() {
    use crate::ui::widget::TableItem;
    let item = TableItem {
        item: vec!["node-a".to_string()],
        metadata: Some(std::collections::BTreeMap::from([
            ("name".to_string(), "node-a".to_string()),
        ])),
    };
    let req = build_detail_request(&item).expect("name should be present");
    match req {
        NodeDetailMessage::Request { name } => assert_eq!(name, "node-a"),
        _ => panic!("expected Request"),
    }
}
```

- [ ] **Step 2: 失敗確認** — `cargo test features::node::view::widgets::node` → コンパイルエラー。

- [ ] **Step 3: 実装**（`node.rs`）。`node_widget` シグネチャに `tx: Sender<Message>` を追加し、Table builder に `on_select` を渡す。

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
    move |_w, item| {
        if let Some(req) = build_detail_request(item) {
            let _ = tx.send(req.into());
        }
        EventResult::Nop
    }
}
```

注: `get_node_table` は既に `KubeTableRow.metadata` に `{"kind": "Node"}` のみを入れている。`name` も入れるよう poller 側を更新する（`name: row.cells[name_pos].clone()` を `metadata` にも追加）。これは Plan 4 の範囲内の小修正として `kube/node.rs` も合わせて変更する。

- [ ] **Step 4: poller の metadata に name を追加**（`src/features/node/kube/node.rs`）。

```rust
metadata: Some(BTreeMap::from([
    ("kind".to_string(), Node::KIND.to_string()),
    ("name".to_string(), name.clone()),
])),
```

- [ ] **Step 5: テスト・ビルド** — `cargo test features::node` → PASS。

- [ ] **Step 6: コミット**

```bash
git add -A
git commit -m "feat(node): send NodeDetailRequest on row selection"
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
