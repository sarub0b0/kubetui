# Node タブ — Plan 1: 一覧タブ（ビルトイン列）実装プラン

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** トップレベルに「Node」タブを追加し、`kubectl get nodes`（-o wide 相当の列も選択可）に相当する Node 一覧を 1 秒間隔で表示する。列は設定ファイル（`theme.node` のプリセット）で指定。

**Architecture:** 既存 Pod タブを踏襲した独立モジュール `features/node/`。`KubeClient::request_table` で `/api/v1/nodes`（Table API）を取得し、サーバ印字済みのセルを `KubeTable` に変換して `NodePoller`（`InfiniteWorker`）が送出、`update_contents` が一覧 Table ウィジェットを更新する。列はクラスタスコープ（namespace 非依存）。

**Tech Stack:** Rust 2021, tokio, crossbeam channel, ratatui, kube-rs / k8s-openapi, mockall, rstest, pretty_assertions, serde/figment。

**Scope（このプラン）:** ビルトイン列（Name/Status/Roles/Age/Version）＋wide 列（InternalIP/ExternalIP/OSImage/KernelVersion/ContainerRuntime）。設定の `column_presets` / `default_preset` で列指定。**含まない**（後続プラン）: ランタイム列ダイアログ（`t`）、ラベル列、詳細ペイン、フィルタ。

**設計スペック:** `docs/superpowers/specs/2026-05-22-node-tab-design.md`

---

## ファイル構成

新規:
- `src/features/node.rs` — モジュールルート（`src/features/pod.rs` に倣う）
- `src/features/node/node_columns.rs` — `NodeColumn` / `NodeColumns`
- `src/features/node/message.rs` — `NodeMessage`
- `src/features/node/kube.rs` — kube 集約
- `src/features/node/kube/node.rs` — `NodeConfig` / `NodePoller`
- `src/features/node/view.rs` — view 集約
- `src/features/node/view/tab.rs` — `NodeTab`
- `src/features/node/view/widgets.rs` — widgets 集約
- `src/features/node/view/widgets/node.rs` — 一覧 Table ウィジェット
- `src/config/theme/node.rs` — `NodeThemeConfig` / `NodeColumnConfig`

変更:
- `src/features.rs` — `pub mod node;`
- `src/features/component_id.rs` — `node_tab`, `node_widget`
- `src/config/theme.rs` — `ThemeConfig` に `node` フィールド、`pub use node::*`
- `src/workers/kube/message.rs` — `Kube::Node(NodeMessage)` 追加
- `src/workers/kube/config.rs` — `KubeWorkerConfig` に `node_config: NodeConfig`
- `src/workers/kube/controller.rs` — `node_config` 保持、`shared_node_columns`、`NodePoller` spawn
- `src/workers/render/action.rs` — `Kube::Node(NodeMessage::Poll)` ハンドラ
- `src/workers/render/window.rs` — `NodeTab` を Event の右隣に挿入、`WindowInit` に `default_node_columns`
- `src/app.rs` — `build_node_columns`、`node_config` 設定、`default_node_columns` を `WindowInit` へ
- `example/config.yaml` — `theme.node` の設定例（列プリセット）を追記

---

## Task 1: NodeColumn / NodeColumns

**Files:**
- Create: `src/features/node/node_columns.rs`

`as_str()` は Table API の `columnDefinitions[].name` と一致させる必要がある（`find_indexes` で名前照合）。標準列は `Name/Status/Roles/Age/Version`、wide 列は `Internal-IP/External-IP/OS-Image/Kernel-Version/Container-Runtime`。

- [ ] **Step 1: 失敗するテストを書く**

`src/features/node/node_columns.rs` の末尾に追加:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr as _;
    use pretty_assertions::assert_eq;

    #[test]
    fn default_columns_are_name_status_roles_age_version() {
        let actual = NodeColumns::default();
        let expected = vec![
            NodeColumn::Name,
            NodeColumn::Status,
            NodeColumn::Roles,
            NodeColumn::Age,
            NodeColumn::Version,
        ];
        assert_eq!(actual.columns(), expected.as_slice());
    }

    #[test]
    fn from_str_normalizes_case_and_separators() {
        assert_eq!(NodeColumn::from_str("internal-ip").unwrap(), NodeColumn::InternalIP);
        assert_eq!(NodeColumn::from_str("OS_Image").unwrap(), NodeColumn::OSImage);
        assert_eq!(NodeColumn::from_str(" Version ").unwrap(), NodeColumn::Version);
        assert!(NodeColumn::from_str("bogus").is_err());
    }

    #[test]
    fn as_str_matches_table_column_definition_names() {
        assert_eq!(NodeColumn::InternalIP.as_str(), "Internal-IP");
        assert_eq!(NodeColumn::ContainerRuntime.as_str(), "Container-Runtime");
        assert_eq!(NodeColumn::Roles.as_str(), "Roles");
    }

    #[test]
    fn ensure_name_column_prepends_name_when_missing() {
        let cols = NodeColumns::new([NodeColumn::Status]).ensure_name_column();
        assert_eq!(cols.columns(), &[NodeColumn::Name, NodeColumn::Status]);
    }
}
```

- [ ] **Step 2: テストが失敗することを確認**

Run: `cargo test features::node::node_columns`
Expected: コンパイルエラー（`NodeColumn` 未定義）。

- [ ] **Step 3: 実装を書く**

`src/features/node/node_columns.rs` の先頭に追加（`pod_columns.rs` を踏襲）:

```rust
use strum::{EnumIter, IntoEnumIterator};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeColumns {
    columns: Vec<NodeColumn>,
}

impl Default for NodeColumns {
    fn default() -> Self {
        NodeColumns {
            columns: DEFAULT_NODE_COLUMNS.to_vec(),
        }
    }
}

impl NodeColumns {
    pub fn new(columns: impl IntoIterator<Item = NodeColumn>) -> Self {
        NodeColumns {
            columns: columns.into_iter().collect(),
        }
    }

    pub fn columns(&self) -> &[NodeColumn] {
        &self.columns
    }

    pub fn ensure_name_column(mut self) -> Self {
        if self.columns.contains(&NodeColumn::Name) {
            return self;
        }
        self.columns.insert(0, NodeColumn::Name);
        self
    }

    pub fn dedup_columns(self) -> Self {
        let mut unique = Vec::new();
        for c in self.columns {
            if !unique.contains(&c) {
                unique.push(c);
            }
        }
        NodeColumns { columns: unique }
    }
}

pub const DEFAULT_NODE_COLUMNS: &[NodeColumn] = &[
    NodeColumn::Name,
    NodeColumn::Status,
    NodeColumn::Roles,
    NodeColumn::Age,
    NodeColumn::Version,
];

#[derive(EnumIter, PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy, Hash)]
pub enum NodeColumn {
    Name,
    Status,
    Roles,
    Age,
    Version,
    InternalIP,
    ExternalIP,
    OSImage,
    KernelVersion,
    ContainerRuntime,
}

impl NodeColumn {
    /// Table API の columnDefinitions[].name と一致させる。
    pub const fn as_str(&self) -> &'static str {
        match self {
            NodeColumn::Name => "Name",
            NodeColumn::Status => "Status",
            NodeColumn::Roles => "Roles",
            NodeColumn::Age => "Age",
            NodeColumn::Version => "Version",
            NodeColumn::InternalIP => "Internal-IP",
            NodeColumn::ExternalIP => "External-IP",
            NodeColumn::OSImage => "OS-Image",
            NodeColumn::KernelVersion => "Kernel-Version",
            NodeColumn::ContainerRuntime => "Container-Runtime",
        }
    }

    pub const fn display(&self) -> &'static str {
        match self {
            NodeColumn::Name => "NAME",
            NodeColumn::Status => "STATUS",
            NodeColumn::Roles => "ROLES",
            NodeColumn::Age => "AGE",
            NodeColumn::Version => "VERSION",
            NodeColumn::InternalIP => "INTERNAL-IP",
            NodeColumn::ExternalIP => "EXTERNAL-IP",
            NodeColumn::OSImage => "OS-IMAGE",
            NodeColumn::KernelVersion => "KERNEL-VERSION",
            NodeColumn::ContainerRuntime => "CONTAINER-RUNTIME",
        }
    }

    pub fn normalize_column(column: &str) -> String {
        column.to_lowercase().replace([' ', '_', '-'], "")
    }
}

#[derive(Debug)]
pub struct NodeColumnParseError;

impl std::fmt::Display for NodeColumnParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid NodeColumn string representation")
    }
}

impl std::error::Error for NodeColumnParseError {}

impl std::str::FromStr for NodeColumn {
    type Err = NodeColumnParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match Self::normalize_column(s).as_str() {
            "name" => Ok(NodeColumn::Name),
            "status" => Ok(NodeColumn::Status),
            "roles" => Ok(NodeColumn::Roles),
            "age" => Ok(NodeColumn::Age),
            "version" => Ok(NodeColumn::Version),
            "internalip" => Ok(NodeColumn::InternalIP),
            "externalip" => Ok(NodeColumn::ExternalIP),
            "osimage" => Ok(NodeColumn::OSImage),
            "kernelversion" => Ok(NodeColumn::KernelVersion),
            "containerruntime" => Ok(NodeColumn::ContainerRuntime),
            _ => Err(NodeColumnParseError),
        }
    }
}
```

- [ ] **Step 4: モジュールを暫定登録してテスト実行**

`src/features.rs` に `pub mod node;` を追加し、`src/features/node.rs` を作成:

```rust
mod node_columns;

pub use node_columns::*;
```

Run: `cargo test features::node::node_columns`
Expected: PASS（4 テスト）。

- [ ] **Step 5: Commit**

```bash
git add src/features.rs src/features/node.rs src/features/node/node_columns.rs
git commit -m "feat(node): add NodeColumn/NodeColumns"
```

---

## Task 2: NodeMessage と Kube::Node

**Files:**
- Create: `src/features/node/message.rs`
- Modify: `src/features/node.rs`, `src/workers/kube/message.rs`

- [ ] **Step 1: message.rs を作成**

`src/features/node/message.rs`（`pod/message.rs` を踏襲）:

```rust
use anyhow::Result;

use crate::{kube::table::KubeTable, message::Message, workers::kube::message::Kube};

#[derive(Debug)]
pub enum NodeMessage {
    Poll(Result<KubeTable>),
}

impl From<NodeMessage> for Message {
    fn from(m: NodeMessage) -> Message {
        Message::Kube(Kube::Node(m))
    }
}
```

- [ ] **Step 2: node.rs に message を登録**

`src/features/node.rs` を更新:

```rust
pub mod message;
mod node_columns;

pub use node_columns::*;
```

- [ ] **Step 3: Kube enum に Node を追加**

`src/workers/kube/message.rs` の import に追加:

```rust
        node::message::NodeMessage,
```

（`pod::message::{LogMessage, PodMessage},` の直後に、`crate::features::{ ... }` ブロック内へ）

`Kube` enum に variant を追加（`Pod(PodMessage),` の直後）:

```rust
    Node(NodeMessage),
```

- [ ] **Step 4: ビルド確認**

Run: `cargo build`
Expected: 成功。`action.rs` の `_ => unreachable!()` により未処理でもコンパイルは通る（Node メッセージはまだ送出しない）。

- [ ] **Step 5: Commit**

```bash
git add src/features/node.rs src/features/node/message.rs src/workers/kube/message.rs
git commit -m "feat(node): add NodeMessage and Kube::Node variant"
```

---

## Task 3: NodeConfig と NodePoller

**Files:**
- Create: `src/features/node/kube/node.rs`, `src/features/node/kube.rs`
- Modify: `src/features/node.rs`
- Test: 同ファイル内 `#[cfg(test)]`

`NodePoller` はクラスタスコープ。`/api/v1/nodes` の Table を取得し、設定列のセルを抜き出して `KubeTable` を作る。

- [ ] **Step 1: 失敗するテストを書く**

`src/features/node/kube/node.rs` に追加:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::kube::apis::v1_table::{Table, TableColumnDefinition, TableRow, Value};
    use crate::mock_expect;
    use mockall::predicate::eq;
    use pretty_assertions::assert_eq;
    use serde_json::Value as JsonValue;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    fn coldef(name: &str) -> TableColumnDefinition {
        TableColumnDefinition { name: name.to_string(), ..Default::default() }
    }

    fn row(cells: &[&str]) -> TableRow {
        TableRow {
            cells: cells.iter().map(|c| Value(JsonValue::String(c.to_string()))).collect(),
            ..Default::default()
        }
    }

    fn node_table_fixture() -> Table {
        Table {
            column_definitions: vec![
                coldef("Name"), coldef("Status"), coldef("Roles"),
                coldef("Age"), coldef("Version"),
            ],
            rows: vec![
                row(&["node-a", "Ready", "worker", "10d", "v1.29.0"]),
                row(&["node-b", "NotReady", "control-plane", "11d", "v1.29.0"]),
            ],
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn builds_kube_table_from_default_columns() {
        let mut client = crate::kube::mock::MockTestKubeClient::new();
        mock_expect!(
            client,
            request_table,
            Table,
            eq("/api/v1/nodes"),
            Ok(node_table_fixture())
        );

        let shared = Arc::new(RwLock::new(NodeColumns::default()));
        let table = get_node_table(&client, &shared).await.unwrap();

        assert_eq!(table.header, vec!["NAME", "STATUS", "ROLES", "AGE", "VERSION"]);
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.rows[0].name, "node-a");
        assert_eq!(table.rows[0].row, vec!["node-a", "Ready", "worker", "10d", "v1.29.0"]);
    }
}
```

- [ ] **Step 2: テストが失敗することを確認**

Run: `cargo test features::node::kube::node`
Expected: コンパイルエラー（`get_node_table` 未定義）。

- [ ] **Step 3: 実装を書く**

`src/features/node/kube/node.rs` の先頭:

```rust
use std::{collections::BTreeMap, sync::Arc};

use anyhow::Result;
use async_trait::async_trait;
use crossbeam::channel::Sender;
use k8s_openapi::{api::core::v1::Node, Resource as _};
use kube::Resource;
use tokio::sync::RwLock;

use crate::{
    features::node::{message::NodeMessage, NodeColumn, NodeColumns},
    kube::{
        apis::v1_table::Table,
        table::{KubeTable, KubeTableRow},
        KubeClient, KubeClientRequest,
    },
    logger,
    message::Message,
    workers::kube::InfiniteWorker,
};

pub type SharedNodeColumns = Arc<RwLock<NodeColumns>>;

#[derive(Debug, Clone, Default)]
pub struct NodeConfig {
    pub default_columns: Option<NodeColumns>,
}

#[derive(Clone)]
pub struct NodePoller {
    tx: Sender<Message>,
    shared_node_columns: SharedNodeColumns,
    kube_client: KubeClient,
}

impl NodePoller {
    pub fn new(
        tx: Sender<Message>,
        shared_node_columns: SharedNodeColumns,
        kube_client: KubeClient,
    ) -> Self {
        Self {
            tx,
            shared_node_columns,
            kube_client,
        }
    }
}

#[async_trait]
impl InfiniteWorker for NodePoller {
    async fn run(&self) {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));

        let Self { tx, .. } = self;

        loop {
            interval.tick().await;

            let node_info = get_node_table(&self.kube_client, &self.shared_node_columns).await;

            if let Err(e) = tx.send(NodeMessage::Poll(node_info).into()) {
                logger!(error, "Failed to send NodeMessage::Poll: {}", e);
                return;
            }
        }
    }
}

async fn get_node_table<C: KubeClientRequest>(
    client: &C,
    shared_node_columns: &SharedNodeColumns,
) -> Result<KubeTable> {
    let node_columns = shared_node_columns.read().await;

    let targets: Vec<&str> = node_columns.columns().iter().map(|c| c.as_str()).collect();

    let path = Node::url_path(&(), None);
    let table: Table = client.request_table(&path).await?;

    let indexes = table.find_indexes(&targets)?;

    let name_index = node_columns
        .columns()
        .iter()
        .position(|c| *c == NodeColumn::Name)
        .expect("Name column must be present in node columns");

    let rows: Vec<KubeTableRow> = table
        .rows
        .iter()
        .map(|row| {
            let cells: Vec<String> = indexes.iter().map(|i| row.cells[*i].to_string()).collect();
            let name = cells[name_index].clone();
            KubeTableRow {
                namespace: String::new(),
                name,
                metadata: Some(BTreeMap::from([("kind".to_string(), Node::KIND.to_string())])),
                row: cells,
            }
        })
        .collect();

    let header: Vec<String> = node_columns
        .columns()
        .iter()
        .map(|c| c.display().to_string())
        .collect();

    let mut kube_table = KubeTable {
        header,
        ..Default::default()
    };
    kube_table.update_rows(rows);

    Ok(kube_table)
}
```

注: `get_node_table` は `KubeClientRequest` ジェネリックにし、`run()` からは `&self.kube_client` で呼ぶ。テストの `MockTestKubeClient` の import パスが異なる場合は、既存ワーカーテストの使用箇所（`grep -rn MockTestKubeClient src`）に合わせること。

- [ ] **Step 4: kube 集約とモジュール登録**

`src/features/node/kube.rs` を作成:

```rust
mod node;

pub use node::*;
```

`src/features/node.rs` を更新:

```rust
pub mod kube;
pub mod message;
mod node_columns;

pub use node_columns::*;
```

- [ ] **Step 5: テスト実行**

Run: `cargo test features::node::kube::node`
Expected: PASS（`builds_kube_table_from_default_columns`）。

- [ ] **Step 6: 列定義名の実機確認（重要）**

`as_str()` が実際の Table 列名と一致するか確認:

Run: `kubectl get nodes -o json -v9 2>&1 | grep -i "as=Table" -A2` もしくは
`kubectl get --raw '/api/v1/nodes' -H 'Accept: application/json;as=Table;v=v1;g=meta.k8s.io' | jq '.columnDefinitions[].name'`
Expected: `Name, Status, Roles, Age, Version, Internal-IP, External-IP, OS-Image, Kernel-Version, Container-Runtime` 等。差異があれば `NodeColumn::as_str()` と Task 1 のテストを修正。

- [ ] **Step 7: Commit**

```bash
git add src/features/node.rs src/features/node/kube.rs src/features/node/kube/node.rs
git commit -m "feat(node): add NodePoller and NodeConfig"
```

---

## Task 4: NodeThemeConfig（設定: 列プリセット）

**Files:**
- Create: `src/config/theme/node.rs`
- Modify: `src/config/theme.rs`

`PodThemeConfig` から `highlights` を除いた形。`NodeColumnConfig` は文字列 → `NodeColumn`。

- [ ] **Step 1: 失敗するテストを書く**

`src/config/theme/node.rs` に追加:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::node::NodeColumn;
    use pretty_assertions::assert_eq;

    #[test]
    fn deserializes_column_presets() {
        let json = r#"{
            "default_preset": "default",
            "column_presets": { "default": ["name", "status", "roles", "age", "version"] }
        }"#;
        let cfg: NodeThemeConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.default_preset.as_deref(), Some("default"));
        let preset = cfg.column_presets.as_ref().unwrap().get("default").unwrap();
        let cols: Vec<NodeColumn> = preset.iter().map(|c| c.0).collect();
        assert_eq!(
            cols,
            vec![
                NodeColumn::Name,
                NodeColumn::Status,
                NodeColumn::Roles,
                NodeColumn::Age,
                NodeColumn::Version
            ]
        );
    }
}
```

- [ ] **Step 2: テストが失敗することを確認**

Run: `cargo test config::theme::node`
Expected: コンパイルエラー（`NodeThemeConfig` 未定義）。

- [ ] **Step 3: 実装を書く**

`src/config/theme/node.rs` の先頭（`config/theme/pod.rs` の列部分を踏襲）:

```rust
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::features::node::NodeColumn;

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct NodeThemeConfig {
    pub default_preset: Option<String>,

    pub column_presets: Option<HashMap<String, Vec<NodeColumnConfig>>>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct NodeColumnConfig(#[serde(with = "serde_node_column")] pub NodeColumn);

mod serde_node_column {
    use std::str::FromStr as _;

    use serde::{de, Deserialize, Deserializer, Serializer};

    use crate::features::node::NodeColumn;

    pub fn serialize<S>(column: &NodeColumn, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // 正規化した文字列で出力（例: internalip）。
        serializer.serialize_str(&NodeColumn::normalize_column(column.as_str()))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<NodeColumn, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        NodeColumn::from_str(&s).map_err(de::Error::custom)
    }
}
```

- [ ] **Step 4: theme.rs に登録**

`src/config/theme.rs` に追加:
- モジュール宣言群（`mod pod;` 付近）に `mod node;`
- `pub use` 群（`pub use pod::*;` 付近）に `pub use node::*;`
- `ThemeConfig` の `pub pod: PodThemeConfig,` の直後に:

```rust
    #[serde(default)]
    pub node: NodeThemeConfig,
```

- [ ] **Step 5: テスト実行**

Run: `cargo test config::theme::node`
Expected: PASS。

- [ ] **Step 6: Commit**

```bash
git add src/config/theme/node.rs src/config/theme.rs
git commit -m "feat(node): add NodeThemeConfig (column presets)"
```

---

## Task 5: KubeWorkerConfig と controller への配線

**Files:**
- Modify: `src/workers/kube/config.rs`, `src/workers/kube/controller.rs`

- [ ] **Step 1: KubeWorkerConfig に node_config を追加**

`src/workers/kube/config.rs`:
- import に `use crate::features::node::kube::NodeConfig;`（既存の `PodConfig` の import 付近）
- `KubeWorkerConfig` の `pub pod_config: PodConfig,` の直後に:

```rust
    pub node_config: NodeConfig,
```

- [ ] **Step 2: controller に node_config を保持**

`src/workers/kube/controller.rs`:
- import に `use crate::features::node::kube::{NodePoller, NodeConfig};`（既存 poller import 群へ）
- `KubeController` struct の `pod_config: PodConfig,` 直後に `node_config: NodeConfig,`
- `KubeWorkerConfig { ... pod_config, ... }` の分解（`new` 内）に `node_config,` を追加し、`Ok(Self { ... pod_config, node_config, ... })` にも追加
- `run()` の `let Self { ... pod_config, ... } = self;` の分解に `node_config,` を追加

- [ ] **Step 3: shared_node_columns 生成と NodePoller spawn**

`run()` 内、`let shared_pod_columns = Arc::new(RwLock::new(...));` の直後に追加:

```rust
            let shared_node_columns = Arc::new(RwLock::new(
                node_config.default_columns.clone().unwrap_or_default(),
            ));
```

`let pod_handle = PodPoller::new(...).spawn();` の直後に追加:

```rust
            let node_handle = NodePoller::new(
                tx.clone(),
                shared_node_columns.clone(),
                client.clone(),
            )
            .spawn();
```

`let poller_handles = vec![ pod_handle, ... ];` に `node_handle,` を追加。

- [ ] **Step 4: ビルド確認**

Run: `cargo build`
Expected: 成功（`NodePoller` が spawn される）。この時点で Node メッセージが送出され始めるが、`action.rs` 未対応だと `unreachable!()` で panic するため、Task 6 まで実行はしないこと（ビルドのみ）。

- [ ] **Step 5: Commit**

```bash
git add src/workers/kube/config.rs src/workers/kube/controller.rs
git commit -m "feat(node): spawn NodePoller in KubeController"
```

---

## Task 6: action.rs で Poll を処理

**Files:**
- Modify: `src/workers/render/action.rs`

- [ ] **Step 1: import 追加**

`use crate::features::{ ... pod::message::{LogMessage, PodMessage}, ... }` のブロックに:

```rust
        node::message::NodeMessage,
```

`component_id` の import 群に `NODE_WIDGET_ID,` を追加（Task 7 で定義。順序により未定義なら Task 7 を先に実施可）。

- [ ] **Step 2: match 分岐を追加**

`update_contents()` の `Kube::Pod(PodMessage::Poll(pods_table)) => { ... }` ブロックの直後に追加:

```rust
        Kube::Node(NodeMessage::Poll(nodes_table)) => {
            update_widget_item_for_table(window, NODE_WIDGET_ID, nodes_table);
        }
```

- [ ] **Step 3: ビルド確認**

Run: `cargo build`
Expected: `NODE_WIDGET_ID` 未定義エラーが出る場合は Task 7 を先に完了させる。両方完了後に成功。

- [ ] **Step 4: Commit**

```bash
git add src/workers/render/action.rs
git commit -m "feat(node): handle NodeMessage::Poll in update_contents"
```

---

## Task 7: component_id と view（一覧 Table・NodeTab）

**Files:**
- Modify: `src/features/component_id.rs`
- Create: `src/features/node/view.rs`, `src/features/node/view/tab.rs`, `src/features/node/view/widgets.rs`, `src/features/node/view/widgets/node.rs`
- Modify: `src/features/node.rs`

- [ ] **Step 1: component_id に追加**

`src/features/component_id.rs` の `component_id!( ... )` 内、tabs に `node_tab,`、widgets に `node_widget,` を追加。

- [ ] **Step 2: 一覧 Table ウィジェット**

`src/features/node/view/widgets/node.rs`（`pod.rs` の table 部分を簡略化。フィルタ・on_select・列ダイアログは後続プラン）:

```rust
use crate::{
    config::theme::WidgetThemeConfig,
    features::component_id::NODE_WIDGET_ID,
    ui::widget::{
        FilterForm, FilterFormTheme, Table, TableTheme, Widget, WidgetBase, WidgetTheme,
    },
};

pub fn node_widget(theme: WidgetThemeConfig) -> Widget<'static> {
    let widget_theme = WidgetTheme::from(theme.clone());
    let table_theme = TableTheme::from(theme.clone());

    let widget_base = WidgetBase::builder()
        .title("Node")
        .theme(widget_theme)
        .build();

    let filter_form_theme = FilterFormTheme::from(theme.clone());
    let filter_form = FilterForm::builder().theme(filter_form_theme).build();

    Table::builder()
        .id(NODE_WIDGET_ID)
        .widget_base(widget_base)
        .filter_form(filter_form)
        .theme(table_theme)
        .filtered_key("NAME")
        .build()
        .into()
}
```

- [ ] **Step 3: widgets 集約**

`src/features/node/view/widgets.rs`:

```rust
mod node;

pub use node::*;
```

- [ ] **Step 4: NodeTab（単一ペイン）**

`src/features/node/view/tab.rs`:

```rust
use crate::{
    config::theme::WidgetThemeConfig,
    features::component_id::NODE_TAB_ID,
    ui::{
        tab::{LayoutElement, NestedLayoutElement, NestedWidgetLayout, TabLayout},
        Tab,
    },
};

use super::widgets::node_widget;
use ratatui::layout::{Constraint, Direction};

pub struct NodeTab {
    pub tab: Tab<'static>,
}

impl NodeTab {
    pub fn new(title: &'static str, theme: WidgetThemeConfig) -> Self {
        let node_widget = node_widget(theme.clone());

        let tab = Tab::new(
            NODE_TAB_ID,
            title,
            [node_widget],
            TabLayout::new(layout, Direction::Vertical),
        );

        Self { tab }
    }
}

fn layout(_split_direction: Direction) -> NestedWidgetLayout {
    NestedWidgetLayout::default().nested_widget_layout([NestedLayoutElement(
        Constraint::Percentage(100),
        LayoutElement::WidgetIndex(0),
    )])
}
```

注: `Tab::new` の第4引数は `TabLayout`。`layout` は `fn layout(split_direction: Direction) -> NestedWidgetLayout`（引数は使わないが Pod の `tab.rs` とシグネチャを揃える）。

- [ ] **Step 5: view 集約とモジュール登録**

`src/features/node/view.rs`:

```rust
mod tab;
mod widgets;

pub use tab::*;
```

`src/features/node.rs`:

```rust
pub mod kube;
pub mod message;
mod node_columns;
pub mod view;

pub use node_columns::*;
```

- [ ] **Step 6: ビルド確認**

Run: `cargo build`
Expected: 成功（`NodeTab` / `NODE_WIDGET_ID` が解決）。Task 6 と相互依存のため、両方完了後に通る。

- [ ] **Step 7: Commit**

```bash
git add src/features/component_id.rs src/features/node.rs src/features/node/view.rs src/features/node/view/tab.rs src/features/node/view/widgets.rs src/features/node/view/widgets/node.rs
git commit -m "feat(node): add Node list tab view"
```

---

## Task 8: window.rs にタブを配置

**Files:**
- Modify: `src/workers/render/window.rs`

- [ ] **Step 1: import 追加**

`features::{ ... pod::{view::PodTab, PodColumns}, ... }` のブロックに:

```rust
        node::view::NodeTab,
```

- [ ] **Step 2: tabs_dialogs() で NodeTab を生成**

`EventTab { tab: event_tab } = EventTab::new(...);` の直後に追加:

```rust
        let NodeTab { tab: node_tab } =
            NodeTab::new("Node", self.theme.component.clone());
```

- [ ] **Step 3: tabs ベクタに挿入（Event の右隣）**

`let tabs = vec![ pod_tab, config_tab, network_tab, event_tab, api_tab, yaml_tab ];` を以下に変更:

```rust
        let tabs = vec![
            pod_tab,
            config_tab,
            network_tab,
            event_tab,
            node_tab,
            api_tab,
            yaml_tab,
        ];
```

- [ ] **Step 4: ビルド・実行確認**

Run: `cargo build`
Expected: 成功。

Run: `cargo run`（クラスタ接続が必要）。
Expected: 5 キーで「Node」タブが開き、ノード一覧（NAME/STATUS/ROLES/AGE/VERSION）が 1 秒ごとに更新される。`/` でテーブル標準のフィルタ（NAME 部分一致）が使える。

- [ ] **Step 5: Commit**

```bash
git add src/workers/render/window.rs
git commit -m "feat(node): place Node tab to the right of Event"
```

---

## Task 9: 設定→ランタイムの配線（app.rs）

**Files:**
- Modify: `src/app.rs`

設定の `theme.node.column_presets` / `default_preset` を `NodeColumns` に解決して `node_config.default_columns` に渡す。`build_pod_columns` を踏襲。

- [ ] **Step 1: 失敗するテストを書く**

`src/app.rs` の `#[cfg(test)]` に追加（既存の pod 用テストがあれば近傍に）:

```rust
#[cfg(test)]
mod node_columns_tests {
    use super::*;
    use crate::config::theme::NodeColumnConfig;
    use crate::features::node::NodeColumn;
    use std::collections::HashMap;

    #[test]
    fn resolves_default_preset() {
        let presets = HashMap::from([(
            "default".to_string(),
            vec![
                NodeColumnConfig(NodeColumn::Name),
                NodeColumnConfig(NodeColumn::Status),
            ],
        )]);
        let actual = build_node_columns(&Some("default".to_string()), &Some(presets)).unwrap();
        let cols: Vec<NodeColumn> = actual.unwrap().columns().to_vec();
        assert_eq!(cols, vec![NodeColumn::Name, NodeColumn::Status]);
    }

    #[test]
    fn none_when_no_preset() {
        let actual = build_node_columns(&None, &None).unwrap();
        assert!(actual.is_none());
    }

    #[test]
    fn errors_when_default_preset_missing_from_presets() {
        let actual = build_node_columns(&Some("gpu".to_string()), &Some(HashMap::new()));
        assert!(actual.is_err());
    }
}
```

- [ ] **Step 2: テストが失敗することを確認**

Run: `cargo test --bin kubetui node_columns_tests`（バイナリ名が異なる場合は `cargo test build_node_columns`）
Expected: コンパイルエラー（`build_node_columns` 未定義）。

- [ ] **Step 3: 実装を書く**

`src/app.rs` に関数を追加（`build_pod_columns` 付近）:

```rust
fn build_node_columns(
    default_preset: &Option<String>,
    column_presets: &Option<std::collections::HashMap<String, Vec<crate::config::theme::NodeColumnConfig>>>,
) -> anyhow::Result<Option<crate::features::node::NodeColumns>> {
    use crate::features::node::NodeColumns;

    let Some(default_preset) = default_preset else {
        return Ok(None);
    };

    let Some(presets) = column_presets else {
        anyhow::bail!("No node column presets defined in config file, but 'default_preset' is set");
    };

    let Some(columns) = presets.get(default_preset) else {
        anyhow::bail!(
            "Default node columns preset '{}' is set in config file but not defined in column_presets",
            default_preset
        );
    };

    let node_columns = NodeColumns::new(columns.iter().map(|c| c.0))
        .ensure_name_column()
        .dedup_columns();

    Ok(Some(node_columns))
}
```

`run()` 内、`kube_worker_config.pod_config.default_columns = build_pod_columns(...)?;` の直後に追加:

```rust
    kube_worker_config.node_config.default_columns = build_node_columns(
        &config.theme.node.default_preset,
        &config.theme.node.column_presets,
    )?;
```

- [ ] **Step 4: テスト実行**

Run: `cargo test build_node_columns`
Expected: PASS（3 テスト）。

- [ ] **Step 5: ビルド確認**

Run: `cargo build`
Expected: 成功。

- [ ] **Step 6: 動作確認（設定あり）**

`~/.config/kubetui/config.yaml` に以下を追加して `cargo run`:

```yaml
theme:
  node:
    column_presets:
      default: [name, status, roles, age, version, internalip, osimage]
    default_preset: default
```

Expected: Node タブに INTERNAL-IP / OS-IMAGE 列が追加表示される。

- [ ] **Step 7: Commit**

```bash
git add src/app.rs
git commit -m "feat(node): wire node column presets from config"
```

---

## Task 10: 仕上げ（fmt / clippy / 全テスト）

- [ ] **Step 1: フォーマット**

注意: ローカルの nightly rustfmt がリポジトリの確定スタイルと異なり、`cargo +nightly fmt` を実行すると**無関係な既存ファイル全体（約110ファイル）が再フォーマット**される。CI は `stable` ツールチェーンで fmt チェックも無いため、**全体 fmt はコミットしない**。新規ファイルは既存スタイル（pod 等）に倣って手書きで揃える方針とする。必要なら個別ファイルのみ確認する。

- [ ] **Step 2: Lint**

Run: `cargo clippy --all-targets`
Expected: 警告なし（あれば修正）。

- [ ] **Step 3: 全テスト**

Run: `cargo test --all`
Expected: PASS。

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "chore(node): fmt and clippy for Node list tab"
```

---

## 後続プラン（このプランの完了後）

- **Plan 2**: ランタイム列ダイアログ（`t`）＋ `NodeMessage::Request(NodeColumns)`。`EventController` のメッセージループで `shared_node_columns` を更新する箇所（Pod の `PodMessage::Request` 処理）を調査して踏襲。`--node-columns` / `--node-columns-preset` CLI も検討。
- **Plan 3**: ラベル列（`label_columns` レジストリ＋ `NodeColumn::Label{key,name}`、`includeObject=Metadata` で `TableRow.object` → `metadata.labels[key]` を抽出、設定衝突/未定義参照の読込時バリデーション）。
- **Plan 4**: 詳細ペイン（2 ペイン化、`NodeDetailWorker`：Node YAML〔managedFields 除去〕＋関連 Pod〔`fieldSelector=spec.nodeName`、全 namespace〕、3 秒更新、`on_select`）。
- **Plan 5**: フィルタ（`node:`/`!node:`/`label:`、nom パーサ、`shared_node_filter`、`labelSelector`、フィルタ入力ウィジェット＋ヘルプダイアログ）。
