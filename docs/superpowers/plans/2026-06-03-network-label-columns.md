# Network label_columns + Column dialog Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Network タブに `label_columns` (config 由来) と column dialog を追加し、Config #1002 / Pod #993 と同等の UX を実現する。

**Architecture:** Config #1002 (`feat/config-label-columns`) と同型の構造を Network に mirror。`NetworkColumnSpec::{Builtin, Label}` + `NetworkColumns` + `NetworkLabelColumn` + registry-aware filter parser + spec-driven poller + metadata-roundtrip column dialog。CLI と presets は無し。KIND と NAME が dialog で OFF 不可の必須列。**Network 固有**: poller は 6+ sub-resource (Service/Ingress/Pod/NetworkPolicy/Gateway V1/V1Beta1/HTTPRoute V1/V1Beta1) を扱う点が Config と違うが、各 sub-resource の `fetch_table` 経由パターンは共通。

**Tech Stack:** Rust 2021, `tokio` async, `crossbeam` channel, `ratatui`, `serde`, `strum` EnumIter, `percent-encoding`。

**Spec:** `docs/superpowers/specs/2026-06-03-config-network-label-columns-design.md` (Config / Network 統合)

**Reference plan:** `docs/superpowers/plans/2026-06-03-config-label-columns.md` (Config 実装プラン、PR #1002 でマージ済み)

---

## File Structure

### New files

- `src/features/network/columns.rs` — `NetworkColumn` enum、`NetworkColumnSpec`、`NetworkLabelColumn`、`NetworkColumns` 型
- `src/config/theme/network.rs` — `NetworkThemeConfig` (config schema)
- `src/features/network/view/widgets/network_columns_dialog.rs` — column dialog widget

### Modified files

- `src/features/network.rs` — `mod columns;` + re-export
- `src/config/theme.rs` — `mod network;` + re-export + `ThemeConfig.network` フィールド
- `src/features/network/message.rs` — `NetworkMessage::ColumnsRequest(NetworkColumns)` variant 追加
- `src/workers/kube/controller.rs` — `SharedNetworkColumns` 型追加、`EventControllerArgs`/`EventController` フィールド追加、destructure 更新、message handler 追加、`NetworkPoller::new` 呼び出し更新
- `src/features/network/kube/network.rs` — `NetworkPoller` が `SharedNetworkColumns` を受領、poller を spec 駆動化、`build_network_row_cells` helper を抽出、`NetworkTableRow` を spec-driven cells に再構成、`NetworkTable` は header 生成を polling に移動して削減
- `src/app.rs` — `build_network_label_registry`/`build_default_network_columns` 追加、起動時 wiring、`Render::new` に `default_network_columns`/`network_label_columns` を渡す
- `src/workers/kube/config.rs` — `KubeWorkerConfig.default_network_columns: NetworkColumns` フィールド追加
- `src/workers/render.rs` — `Render` 構造体に `default_network_columns`/`network_label_columns` フィールド追加、`WindowInit` に同様
- `src/workers/render/window.rs` — `WindowInit` 構造体に追加、`NetworkTab::new` に渡す、`NetworkTab` から `network_columns_dialog` を destructure し global dialog list に追加
- `src/features/network/view/tab.rs` — `NetworkTab` に `network_columns_dialog` フィールド追加、`new` シグネチャ拡張
- `src/features/network/view/widgets/network.rs` — `network_widget` に `label_registry` 引数追加、action `'t'` 追加、`network_filter_applicator(label_registry, tx)` で呼ぶ
- `src/features/network/view/widgets.rs` — `mod network_columns_dialog;` + re-export
- `src/features/network/filter/parser.rs` — `parse_network_filter(input, &[NetworkLabelColumn])` に変更
- `src/features/network/filter.rs` — `network_filter_applicator(label_registry, tx)` に変更
- `src/features/component_id.rs` — `network_columns_dialog` 追加

---

## Pre-flight

- [ ] `main` に Config #1002 がマージされていること、ローカルが最新であること

```bash
git checkout main
git pull
git log --oneline -3 | rg "feat\(config\): label_columns"  # PR #1002 のマージコミットがあるはず
git checkout -b feat/network-label-columns
```

---

## Task 1: 型 (NetworkColumn / NetworkColumnSpec / NetworkLabelColumn / NetworkColumns)

**Files:**
- Create: `src/features/network/columns.rs`
- Modify: `src/features/network.rs`

- [ ] **Step 1: Write `src/features/network/columns.rs`**

```rust
use strum::EnumIter;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NetworkColumnSpec {
    Builtin(NetworkColumn),
    Label { key: String, header: String },
}

impl NetworkColumnSpec {
    pub fn header(&self) -> String {
        match self {
            NetworkColumnSpec::Builtin(c) => c.display().to_string(),
            NetworkColumnSpec::Label { header, .. } => header.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkLabelColumn {
    pub name: String,
    pub key: String,
    pub header: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkColumns {
    columns: Vec<NetworkColumnSpec>,
}

impl Default for NetworkColumns {
    fn default() -> Self {
        NetworkColumns::from_builtins(DEFAULT_NETWORK_COLUMNS.iter().copied())
    }
}

impl NetworkColumns {
    pub fn new(columns: impl IntoIterator<Item = NetworkColumnSpec>) -> Self {
        NetworkColumns {
            columns: columns.into_iter().collect(),
        }
    }

    pub fn from_builtins(columns: impl IntoIterator<Item = NetworkColumn>) -> Self {
        NetworkColumns {
            columns: columns.into_iter().map(NetworkColumnSpec::Builtin).collect(),
        }
    }

    pub fn specs(&self) -> &[NetworkColumnSpec] {
        &self.columns
    }

    /// KIND と NAME が存在しない場合のみ挿入する (KIND を index 0、NAME を
    /// その直後)。既存の列順は保持し、reorder はしない。
    pub fn ensure_required(mut self) -> Self {
        let has_kind = self
            .columns
            .iter()
            .any(|s| matches!(s, NetworkColumnSpec::Builtin(NetworkColumn::Kind)));
        if !has_kind {
            self.columns
                .insert(0, NetworkColumnSpec::Builtin(NetworkColumn::Kind));
        }

        let kind_pos = self
            .columns
            .iter()
            .position(|s| matches!(s, NetworkColumnSpec::Builtin(NetworkColumn::Kind)))
            .expect("Kind just ensured");
        let has_name = self
            .columns
            .iter()
            .any(|s| matches!(s, NetworkColumnSpec::Builtin(NetworkColumn::Name)));
        if !has_name {
            self.columns
                .insert(kind_pos + 1, NetworkColumnSpec::Builtin(NetworkColumn::Name));
        }

        self
    }

    pub fn dedup_columns(self) -> Self {
        let mut unique: Vec<NetworkColumnSpec> = Vec::new();
        for spec in self.columns {
            if !unique.contains(&spec) {
                unique.push(spec);
            }
        }
        NetworkColumns { columns: unique }
    }
}

pub const DEFAULT_NETWORK_COLUMNS: &[NetworkColumn] = &[
    NetworkColumn::Kind,
    NetworkColumn::Name,
    NetworkColumn::Age,
];

#[derive(EnumIter, PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy, Hash)]
pub enum NetworkColumn {
    Kind,
    Name,
    Age,
}

impl NetworkColumn {
    pub const fn as_str(&self) -> &'static str {
        match self {
            NetworkColumn::Kind => "Kind",
            NetworkColumn::Name => "Name",
            NetworkColumn::Age => "Age",
        }
    }

    pub const fn display(&self) -> &'static str {
        match self {
            NetworkColumn::Kind => "KIND",
            NetworkColumn::Name => "NAME",
            NetworkColumn::Age => "AGE",
        }
    }

    pub fn normalize_column(column: &str) -> String {
        column.to_lowercase().replace([' ', '_', '-'], "")
    }
}

#[derive(Debug)]
pub struct NetworkColumnParseError;

impl std::fmt::Display for NetworkColumnParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid NetworkColumn string representation")
    }
}

impl std::error::Error for NetworkColumnParseError {}

impl std::str::FromStr for NetworkColumn {
    type Err = NetworkColumnParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match Self::normalize_column(s).as_str() {
            "kind" => Ok(NetworkColumn::Kind),
            "name" => Ok(NetworkColumn::Name),
            "age" => Ok(NetworkColumn::Age),
            _ => Err(NetworkColumnParseError),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn builtins(cols: &[NetworkColumn]) -> Vec<NetworkColumnSpec> {
        cols.iter().copied().map(NetworkColumnSpec::Builtin).collect()
    }

    #[test]
    fn default_has_kind_name_age_in_order() {
        let cols = NetworkColumns::default();
        assert_eq!(
            cols.specs(),
            builtins(&[NetworkColumn::Kind, NetworkColumn::Name, NetworkColumn::Age]).as_slice()
        );
    }

    #[test]
    fn ensure_required_inserts_both_when_absent() {
        let cols = NetworkColumns::from_builtins([NetworkColumn::Age]).ensure_required();
        assert_eq!(
            cols.specs(),
            builtins(&[NetworkColumn::Kind, NetworkColumn::Name, NetworkColumn::Age]).as_slice()
        );
    }

    #[test]
    fn ensure_required_inserts_name_after_existing_kind() {
        let cols =
            NetworkColumns::from_builtins([NetworkColumn::Kind, NetworkColumn::Age])
                .ensure_required();
        assert_eq!(
            cols.specs(),
            builtins(&[NetworkColumn::Kind, NetworkColumn::Name, NetworkColumn::Age]).as_slice()
        );
    }

    #[test]
    fn ensure_required_inserts_kind_when_only_name_present() {
        let cols =
            NetworkColumns::from_builtins([NetworkColumn::Name, NetworkColumn::Age])
                .ensure_required();
        assert_eq!(
            cols.specs(),
            builtins(&[NetworkColumn::Kind, NetworkColumn::Name, NetworkColumn::Age]).as_slice()
        );
    }

    #[test]
    fn ensure_required_preserves_order_when_both_present() {
        let cols =
            NetworkColumns::from_builtins([NetworkColumn::Name, NetworkColumn::Kind, NetworkColumn::Age])
                .ensure_required();
        assert_eq!(
            cols.specs(),
            builtins(&[NetworkColumn::Name, NetworkColumn::Kind, NetworkColumn::Age]).as_slice()
        );
    }

    #[test]
    fn ensure_required_inserts_both_into_empty() {
        let cols = NetworkColumns::new([]).ensure_required();
        assert_eq!(
            cols.specs(),
            builtins(&[NetworkColumn::Kind, NetworkColumn::Name]).as_slice()
        );
    }

    #[test]
    fn ensure_required_prepends_to_label_only_input() {
        let label = NetworkColumnSpec::Label {
            key: "app.kubernetes.io/name".to_string(),
            header: "APP".to_string(),
        };
        let cols = NetworkColumns::new([label.clone()]).ensure_required();
        assert_eq!(
            cols.specs(),
            &[
                NetworkColumnSpec::Builtin(NetworkColumn::Kind),
                NetworkColumnSpec::Builtin(NetworkColumn::Name),
                label,
            ]
        );
    }

    #[test]
    fn dedup_columns_removes_duplicates_preserving_first() {
        let cols = NetworkColumns::new([
            NetworkColumnSpec::Builtin(NetworkColumn::Kind),
            NetworkColumnSpec::Builtin(NetworkColumn::Name),
            NetworkColumnSpec::Builtin(NetworkColumn::Kind),
            NetworkColumnSpec::Builtin(NetworkColumn::Age),
        ])
        .dedup_columns();
        assert_eq!(
            cols.specs(),
            builtins(&[NetworkColumn::Kind, NetworkColumn::Name, NetworkColumn::Age]).as_slice()
        );
    }

    #[test]
    fn builtin_spec_header_is_uppercase_display() {
        assert_eq!(
            NetworkColumnSpec::Builtin(NetworkColumn::Kind).header(),
            "KIND"
        );
    }

    #[test]
    fn label_spec_header_is_as_given() {
        let s = NetworkColumnSpec::Label {
            key: "app.kubernetes.io/name".to_string(),
            header: "APP".to_string(),
        };
        assert_eq!(s.header(), "APP");
    }

    #[test]
    fn normalize_column_strips_space_underscore_hyphen_and_lowercases() {
        assert_eq!(NetworkColumn::normalize_column("KIND"), "kind");
        assert_eq!(NetworkColumn::normalize_column("network-policy"), "networkpolicy");
        assert_eq!(NetworkColumn::normalize_column("Age_Group"), "agegroup");
    }

    #[test]
    fn from_str_accepts_normalized_forms() {
        use std::str::FromStr;
        assert!(matches!(
            NetworkColumn::from_str("KIND"),
            Ok(NetworkColumn::Kind)
        ));
        assert!(matches!(
            NetworkColumn::from_str("age"),
            Ok(NetworkColumn::Age)
        ));
        assert!(NetworkColumn::from_str("bogus").is_err());
    }
}
```

- [ ] **Step 2: Add module declaration in `src/features/network.rs`**

Current content:
```rust
mod filter;
pub mod kube;
pub mod message;
pub mod view;

pub use filter::network_filter_applicator;
```

Change to:
```rust
mod columns;
mod filter;
pub mod kube;
pub mod message;
pub mod view;

pub use columns::{NetworkColumn, NetworkColumnSpec, NetworkColumns, NetworkLabelColumn};
pub use filter::network_filter_applicator;
```

- [ ] **Step 3: Run tests**

```bash
cargo build 2>&1 | rg "error|warning: " | rg -v "kubeconfig|never used|never constructed|associated items|unused" | head -5
cargo test --all 2>&1 | rg "test result:" | tail -3
```

Expected: 0 errors, 12 new tests in `network::columns::tests` pass. Dead-code warnings on `NetworkColumns`/methods are expected (consumed in later tasks).

- [ ] **Step 4: Commit**

```bash
git add src/features/network/columns.rs src/features/network.rs
git commit -m "feat(network): introduce NetworkColumn/NetworkColumnSpec/NetworkColumns types"
```

---

## Task 2: Config schema (NetworkThemeConfig)

**Files:**
- Create: `src/config/theme/network.rs`
- Modify: `src/config/theme.rs`

- [ ] **Step 1: Create `src/config/theme/network.rs`**

```rust
use serde::{Deserialize, Serialize};

use super::LabelColumnConfig;

/// Theme/config-level settings for the Network tab.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct NetworkThemeConfig {
    /// Registry of label columns. All entries are appended to the default
    /// builtin columns at startup (user can toggle them off via the column
    /// dialog).
    pub label_columns: Option<Vec<LabelColumnConfig>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn deserializes_label_columns() {
        let json = r#"{
            "label_columns": [
                { "name": "app", "label": "app.kubernetes.io/name" }
            ]
        }"#;
        let cfg: NetworkThemeConfig = serde_json::from_str(json).unwrap();
        let labels = cfg.label_columns.as_ref().unwrap();
        assert_eq!(labels.len(), 1);
        assert_eq!(labels[0].name, "app");
        assert_eq!(labels[0].label, "app.kubernetes.io/name");
    }

    #[test]
    fn default_has_none_label_columns() {
        let cfg = NetworkThemeConfig::default();
        assert!(cfg.label_columns.is_none());
    }
}
```

- [ ] **Step 2: Wire module + ThemeConfig field**

Modify `src/config/theme.rs`:

There is an existing `mod network;` already in the file (network theme already existed before this PR). Actually wait — re-check. Looking at PR #1002 Config additions, `mod config;` was added there. The Network theme `mod network;` may or may not exist depending on prior PRs.

Run first to check:

```bash
rg -n "^mod network|^pub use network" src/config/theme.rs
```

If `mod network;` already exists in `src/config/theme.rs` (for some other reason), this step is just adding the `pub use network::NetworkThemeConfig;` and the `pub network: NetworkThemeConfig` field. Otherwise, add all three:

```rust
mod network;
```

(alphabetical order in existing mod block — between `mod list;` and `mod node;` if alphabetized, or wherever fits the existing convention)

```rust
pub use network::NetworkThemeConfig;
```

(in the `pub use` re-export block)

Add `pub network: NetworkThemeConfig,` to `ThemeConfig` struct. Place it after `pub event: EventThemeConfig,` (since current order in the struct is Pod → Config → Node → Event → API → YAML → Help; Network should go after Event but before API per the spec convention — actually verify by reading the file). Use `#[serde(default)]`:

```rust
pub struct ThemeConfig {
    // ... existing ...
    #[serde(default)]
    pub event: EventThemeConfig,

    #[serde(default)]
    pub network: NetworkThemeConfig,  // NEW

    // ... rest unchanged
}
```

Note: read the actual file to verify the existing field order; insert `network` in the position that matches the tab order convention.

- [ ] **Step 3: Verify**

```bash
cargo build 2>&1 | rg "error|warning: " | rg -v "kubeconfig" | head -5
cargo test --all 2>&1 | rg "test result:" | tail -3
```

Expected: clean build, +2 new tests.

- [ ] **Step 4: Commit**

```bash
git add src/config/theme.rs src/config/theme/network.rs
git commit -m "feat(network-theme): add NetworkThemeConfig with label_columns"
```

---

## Task 3: NetworkMessage::ColumnsRequest + SharedNetworkColumns + Controller routing

**Files:**
- Modify: `src/features/network/message.rs`
- Modify: `src/workers/kube/controller.rs`

- [ ] **Step 1: Add `NetworkMessage::ColumnsRequest` variant**

Modify `src/features/network/message.rs`:

Add import at the top (extend existing `use crate::...`):

```rust
use crate::features::network::NetworkColumns;
```

Modify the `NetworkMessage` enum (add new variant after `Filter`):

```rust
#[derive(Debug)]
pub enum NetworkMessage {
    Request(NetworkRequest),
    Response(NetworkResponse),
    /// Replace the active labelSelector value. `None` clears it.
    Filter(Option<String>),
    /// Replace the active column composition (sent from the column dialog).
    /// The poller will use the new columns on the next poll.
    ColumnsRequest(NetworkColumns),
}
```

- [ ] **Step 2: Add `SharedNetworkColumns` type in controller**

Modify `src/workers/kube/controller.rs`:

Find the existing `pub type SharedNetworkFilter = ...;` and add after it:

```rust
pub type SharedNetworkColumns = Arc<RwLock<NetworkColumns>>;
```

Add `NetworkColumns` import. Extend existing `use crate::features::network::...` import block (if it exists; otherwise add):

```rust
use crate::features::network::NetworkColumns;
```

- [ ] **Step 3: Add field to `EventControllerArgs` and `EventController`**

In `EventControllerArgs` struct, add `shared_network_columns: SharedNetworkColumns,` after `shared_network_filter`:

```rust
struct EventControllerArgs {
    // ... existing ...
    shared_network_filter: SharedNetworkFilter,
    shared_network_columns: SharedNetworkColumns,  // NEW
    // ... rest unchanged
}
```

Apply the same to `EventController` struct and `EventController::new` body (`shared_network_columns: args.shared_network_columns,`).

- [ ] **Step 4: Construct shared_network_columns and pass via args**

Find the section where `shared_network_filter` is constructed:

```rust
let shared_network_filter: SharedNetworkFilter = Arc::new(RwLock::new(None));
```

Add immediately after:

```rust
let shared_network_columns: SharedNetworkColumns =
    Arc::new(RwLock::new(NetworkColumns::default()));
```

Find the `EventControllerArgs { ... }` literal and add:

```rust
shared_network_filter: shared_network_filter.clone(),
shared_network_columns: shared_network_columns.clone(),  // NEW
```

- [ ] **Step 5: Add field to destructure in `run()`**

Find the `let Self { ... } = self;` destructure and add `shared_network_columns,` immediately after `shared_network_filter,`.

- [ ] **Step 6: Add message handler for `ColumnsRequest`**

Find the existing `Kube::Network(NetworkMessage::Filter(sel))` handler. Add an arm immediately after:

```rust
Kube::Network(NetworkMessage::Filter(sel)) => {
    *shared_network_filter.write().await = sel;
}

Kube::Network(NetworkMessage::ColumnsRequest(columns)) => {
    *shared_network_columns.write().await = columns;
}
```

- [ ] **Step 7: Build and verify**

```bash
cargo build 2>&1 | rg "error" | head -5
cargo test --all 2>&1 | rg "test result:" | tail -1
```

Expected: 0 errors. `NetworkPoller::new` will still take the existing args — it has not yet been modified to consume `shared_network_columns`. That's Task 4. Dead-code warning on `ColumnsRequest` expected.

- [ ] **Step 8: Commit**

```bash
git add src/features/network/message.rs src/workers/kube/controller.rs
git commit -m "feat(network-msg): ColumnsRequest variant + SharedNetworkColumns plumbing"
```

---

## Task 4: Poller spec-driven + label value rendering

**Files:**
- Modify: `src/features/network/kube/network.rs`
- Modify: `src/workers/kube/controller.rs` (`NetworkPoller::new` call site)

The current poller uses an intermediate `NetworkTableRow { namespace, kind, version, name, age }` populated from API table cells (`Name`, `Age` indices), then `to_kube_table_row(is_insert_ns)` produces the final `KubeTableRow` with fixed `[KIND, NAME, AGE]` ordering. After this task:

- `NetworkTableRow` carries `cells: Vec<String>` (built spec-driven via `build_network_row_cells`) instead of the hard-coded `age` field
- `NetworkTable` wrapper is eliminated; header and row construction happen in `polling()` directly (like Config does)
- The pure helper `build_network_row_cells` is unit-testable
- `target_columns` is dynamic per spec (skip KIND and Label) so toggling AGE off only fetches what's displayed

- [ ] **Step 1: Update imports + add `shared_network_columns` to `NetworkPoller`**

Modify `src/features/network/kube/network.rs`:

Extend the top `use crate::...` block:

```rust
use crate::{
    features::{
        api_resources::kube::{ApiResource, ApiResources, SharedApiResources},
        network::{
            message::{GatewayVersion, HTTPRouteVersion, NetworkResponse},
            NetworkColumn,
            NetworkColumnSpec,
            NetworkColumns,
        },
    },
    kube::{
        apis::{
            networking::gateway::{v1, v1beta1},
            v1_table::Table,
        },
        table::{insert_ns, KubeTable, KubeTableRow},
        KubeClient,
        KubeClientRequest,
    },
    logger,
    message::Message,
    workers::kube::{
        InfiniteWorker,
        SharedNetworkColumns,
        SharedNetworkFilter,
        SharedTargetNamespaces,
    },
};
```

(Other top-level imports — `anyhow`, `async_trait`, `crossbeam`, `futures`, `k8s_openapi`, `kube`, `percent_encoding` — remain unchanged.)

Modify `NetworkPoller` struct + `new`:

```rust
#[derive(Clone)]
pub struct NetworkPoller {
    tx: Sender<Message>,
    shared_target_namespaces: SharedTargetNamespaces,
    shared_network_columns: SharedNetworkColumns,
    shared_network_filter: SharedNetworkFilter,
    kube_client: KubeClient,
    api_resources: SharedApiResources,
}

impl NetworkPoller {
    pub fn new(
        tx: Sender<Message>,
        shared_target_namespaces: SharedTargetNamespaces,
        shared_network_columns: SharedNetworkColumns,
        shared_network_filter: SharedNetworkFilter,
        kube_client: KubeClient,
        api_resources: SharedApiResources,
    ) -> Self {
        Self {
            tx,
            shared_target_namespaces,
            shared_network_columns,
            shared_network_filter,
            kube_client,
            api_resources,
        }
    }
}
```

- [ ] **Step 2: Add `build_network_row_cells` helper**

Insert at file scope (e.g., right before `fn target_resources(...)` or near `fetch_resource_per_namespace`):

```rust
/// Build the per-row cell vector from a spec list, the resource's kind name,
/// and a k8s API `TableRow`.
///
/// `builtin_indexes` are the positional indexes into `row.cells` for the
/// non-KIND builtin columns (NAME / AGE, in the order specified by the
/// fetch's `target_columns`).
pub(crate) fn build_network_row_cells(
    specs: &[NetworkColumnSpec],
    kind: &str,
    row: &crate::kube::apis::v1_table::TableRow,
    builtin_indexes: &[usize],
) -> Vec<String> {
    let mut builtin_iter = builtin_indexes.iter();
    specs
        .iter()
        .map(|s| match s {
            NetworkColumnSpec::Builtin(NetworkColumn::Kind) => kind.to_string(),
            NetworkColumnSpec::Builtin(_) => {
                let i = builtin_iter.next().expect("builtin index available");
                row.cells[*i].to_string()
            }
            NetworkColumnSpec::Label { key, .. } => row
                .object
                .as_ref()
                .and_then(|o| o.0.get("metadata"))
                .and_then(|m| m.get("labels"))
                .and_then(|l| l.get(key))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        })
        .collect()
}
```

- [ ] **Step 3: Refactor `NetworkTableRow` to carry spec-driven cells**

Replace the existing `NetworkTableRow` definition + `to_kube_table_row` impl + the entire `NetworkTable` struct/impl with a leaner version:

```rust
#[derive(Debug, Default, Clone)]
struct NetworkTableRow {
    namespace: String,
    kind: String,
    version: String,
    name: String,
    cells: Vec<String>,
}

impl NetworkTableRow {
    fn to_kube_table_row(&self, is_insert_ns: bool) -> KubeTableRow {
        let mut row = self.cells.clone();
        if is_insert_ns {
            row.insert(0, self.namespace.clone());
        }
        KubeTableRow {
            namespace: self.namespace.clone(),
            name: self.name.clone(),
            metadata: Some(BTreeMap::from([
                ("kind".to_string(), self.kind.clone()),
                ("version".to_string(), self.version.clone()),
            ])),
            row,
        }
    }
}
```

(Note: drop the `pub` from `NetworkTableRow` since it's no longer needed at the module-public level — only the `kube/network.rs` module uses it now. If `pub(super)` is needed for tests in a different module, adjust accordingly.)

The entire `NetworkTable` struct and its impl block are now deleted — header and row construction move into `polling()`.

- [ ] **Step 4: Refactor `run` to read columns and thread to `polling`**

Replace the existing `impl InfiniteWorker for NetworkPoller`:

```rust
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

            let columns = self.shared_network_columns.read().await.clone();
            let label_selector = self.shared_network_filter.read().await.clone();

            let table = self
                .polling(&target_resources, &columns, label_selector.as_deref())
                .await;

            if let Err(e) = tx.send(NetworkResponse::List(table).into()) {
                logger!(error, "Failed to send NetworkResponse::List: {}", e);
                return;
            }
        }
    }
}
```

- [ ] **Step 5: Rewrite `polling` to build KubeTable inline from specs**

Replace the existing `impl NetworkPoller { async fn polling ... }` block:

```rust
impl NetworkPoller {
    async fn polling(
        &self,
        target_resources: &[TargetResource],
        columns: &NetworkColumns,
        label_selector: Option<&str>,
    ) -> Result<KubeTable> {
        let target_namespaces = self.shared_target_namespaces.read().await;
        let specs = columns.specs();

        // Build target_columns dynamically from specs (skip KIND — supplied by
        // each resource's TargetResource::as_str() — and Label, which is built
        // from row.object.metadata.labels).
        let target_columns: Vec<&str> = specs
            .iter()
            .filter_map(|s| match s {
                NetworkColumnSpec::Builtin(NetworkColumn::Kind) => None,
                NetworkColumnSpec::Builtin(c) => Some(c.as_str()),
                NetworkColumnSpec::Label { .. } => None,
            })
            .collect();

        let rows: Vec<_> = join_all(target_resources.iter().map(|kind| {
            self.fetch_resource(kind, &target_namespaces, specs, &target_columns, label_selector)
        }))
        .await
        .into_iter()
        .inspect(|res| {
            if let Err(e) = res {
                logger!(error, "Failed to fetch resource: {:?}", e);
            }
        })
        .filter_map(|res| res.ok())
        .collect();

        let is_insert_ns = insert_ns(&target_namespaces);

        let mut header: Vec<String> = specs.iter().map(|s| s.header()).collect();
        if is_insert_ns {
            header.insert(0, "NAMESPACE".to_string());
        }

        let kube_rows: Vec<KubeTableRow> = rows
            .into_iter()
            .flatten()
            .map(|r| r.to_kube_table_row(is_insert_ns))
            .collect();

        Ok(KubeTable {
            header,
            rows: kube_rows,
        })
    }

    async fn fetch_resource(
        &self,
        kind: &TargetResource,
        namespaces: &[String],
        specs: &[NetworkColumnSpec],
        target_columns: &[&str],
        label_selector: Option<&str>,
    ) -> Result<Vec<NetworkTableRow>> {
        let client = &self.kube_client;

        let jobs = try_join_all(namespaces.iter().map(|ns| {
            fetch_resource_per_namespace(client, kind, ns, specs, target_columns, label_selector)
        }))
        .await?;

        Ok(jobs.into_iter().flatten().collect())
    }
}
```

- [ ] **Step 6: Rewrite `fetch_resource_per_namespace` to use the helper**

Replace the existing standalone `fetch_resource_per_namespace`:

```rust
async fn fetch_resource_per_namespace(
    client: &KubeClient,
    kind: &TargetResource,
    ns: &str,
    specs: &[NetworkColumnSpec],
    target_columns: &[&str],
    label_selector: Option<&str>,
) -> Result<Vec<NetworkTableRow>> {
    let table = kind.fetch_table(client, ns, label_selector).await?;

    let indexes = table.find_indexes(target_columns)?;
    let name_pos_in_specs = specs
        .iter()
        .position(|s| matches!(s, NetworkColumnSpec::Builtin(NetworkColumn::Name)))
        .expect("Name column must be present in network columns");

    let rows = table
        .rows
        .iter()
        .map(|row| {
            let cells = build_network_row_cells(specs, &kind.to_string(), row, &indexes);
            let name = cells[name_pos_in_specs].clone();
            NetworkTableRow {
                namespace: ns.to_string(),
                kind: kind.to_string(),
                version: kind.version().to_string(),
                name,
                cells,
            }
        })
        .collect();

    Ok(rows)
}
```

- [ ] **Step 7: Add unit tests for `build_network_row_cells`**

Append at the bottom of `src/features/network/kube/network.rs` (alongside the existing `mod tests`):

```rust
#[cfg(test)]
mod build_network_row_cells_tests {
    use super::*;
    use crate::kube::apis::v1_table::{TableRow, Value};
    use k8s_openapi::apimachinery::pkg::runtime::RawExtension;
    use pretty_assertions::assert_eq;
    use serde_json::Value as JsonValue;

    fn make_row(cells: &[&str]) -> TableRow {
        TableRow {
            cells: cells
                .iter()
                .map(|c| Value(JsonValue::String(c.to_string())))
                .collect(),
            ..Default::default()
        }
    }

    fn make_row_with_labels(cells: &[&str], labels: &[(&str, &str)]) -> TableRow {
        let labels_json: serde_json::Map<String, JsonValue> = labels
            .iter()
            .map(|(k, v)| (k.to_string(), JsonValue::String(v.to_string())))
            .collect();
        let object = serde_json::json!({ "metadata": { "labels": labels_json } });
        let mut row = make_row(cells);
        row.object = Some(RawExtension(object));
        row
    }

    #[test]
    fn builtin_only_cells_in_spec_order_with_kind_from_argument() {
        let specs = vec![
            NetworkColumnSpec::Builtin(NetworkColumn::Kind),
            NetworkColumnSpec::Builtin(NetworkColumn::Name),
            NetworkColumnSpec::Builtin(NetworkColumn::Age),
        ];
        let row = make_row(&["my-svc", "3h"]);
        let cells = build_network_row_cells(&specs, "Service", &row, &[0, 1]);
        assert_eq!(cells, vec!["Service", "my-svc", "3h"]);
    }

    #[test]
    fn label_arm_returns_value_when_label_present() {
        let specs = vec![
            NetworkColumnSpec::Builtin(NetworkColumn::Name),
            NetworkColumnSpec::Label {
                key: "app".to_string(),
                header: "APP".to_string(),
            },
        ];
        let row = make_row_with_labels(&["my-svc"], &[("app", "nginx")]);
        let cells = build_network_row_cells(&specs, "Service", &row, &[0]);
        assert_eq!(cells, vec!["my-svc", "nginx"]);
    }

    #[test]
    fn label_arm_returns_empty_when_label_absent() {
        let specs = vec![
            NetworkColumnSpec::Builtin(NetworkColumn::Name),
            NetworkColumnSpec::Label {
                key: "app".to_string(),
                header: "APP".to_string(),
            },
        ];
        let row = make_row_with_labels(&["my-svc"], &[("other", "x")]);
        let cells = build_network_row_cells(&specs, "Service", &row, &[0]);
        assert_eq!(cells, vec!["my-svc", ""]);
    }

    #[test]
    fn label_arm_returns_empty_when_no_object() {
        let specs = vec![
            NetworkColumnSpec::Builtin(NetworkColumn::Name),
            NetworkColumnSpec::Label {
                key: "app".to_string(),
                header: "APP".to_string(),
            },
        ];
        let row = make_row(&["my-svc"]);
        let cells = build_network_row_cells(&specs, "Service", &row, &[0]);
        assert_eq!(cells, vec!["my-svc", ""]);
    }

    #[test]
    fn mixed_builtin_and_label_in_spec_order() {
        let specs = vec![
            NetworkColumnSpec::Builtin(NetworkColumn::Kind),
            NetworkColumnSpec::Label {
                key: "env".to_string(),
                header: "ENV".to_string(),
            },
            NetworkColumnSpec::Builtin(NetworkColumn::Name),
            NetworkColumnSpec::Builtin(NetworkColumn::Age),
        ];
        // builtin order from target_columns derived from spec: Name (0), Age (1).
        let row = make_row_with_labels(&["my-svc", "3h"], &[("env", "prod")]);
        let cells = build_network_row_cells(&specs, "Ingress", &row, &[0, 1]);
        assert_eq!(cells, vec!["Ingress", "prod", "my-svc", "3h"]);
    }
}
```

- [ ] **Step 8: Update controller call site to pass `shared_network_columns`**

Modify `src/workers/kube/controller.rs`. Find the existing `NetworkPoller::new(...)` call:

```rust
let network_handle = NetworkPoller::new(
    tx.clone(),
    shared_target_namespaces.clone(),
    shared_network_filter.clone(),
    client.clone(),
    shared_api_resources.clone(),
)
.spawn();
```

Replace with (inserts `shared_network_columns.clone()` as the 3rd positional arg):

```rust
let network_handle = NetworkPoller::new(
    tx.clone(),
    shared_target_namespaces.clone(),
    shared_network_columns.clone(),
    shared_network_filter.clone(),
    client.clone(),
    shared_api_resources.clone(),
)
.spawn();
```

- [ ] **Step 9: Build, test, fmt**

```bash
cargo build 2>&1 | rg "error" | head -5
cargo test --all 2>&1 | rg "test result:" | tail -3
cargo +nightly fmt
```

Expected: 0 errors. Test count goes up by 5 (the new `build_network_row_cells_tests` module). Existing `mod tests` for `find_api_resource` continues to pass.

- [ ] **Step 10: Commit**

```bash
git add src/features/network/kube/network.rs src/workers/kube/controller.rs
git commit -m "feat(network-poller): spec-driven rows with label value rendering"
```

---

## Task 5: app.rs registry + default + Render/Window/Tab wiring

**Files:**
- Modify: `src/app.rs`
- Modify: `src/features/network.rs` (re-export `DEFAULT_NETWORK_COLUMNS`)
- Modify: `src/workers/kube/config.rs` (`KubeWorkerConfig.default_network_columns` field)
- Modify: `src/workers/kube/controller.rs` (use `default_network_columns` from config for `shared_network_columns` initial value)
- Modify: `src/workers/render.rs`
- Modify: `src/workers/render/window.rs`
- Modify: `src/features/network/view/tab.rs`
- Modify: `src/features/network/view/widgets/network.rs`

- [ ] **Step 1: Re-export `DEFAULT_NETWORK_COLUMNS`**

Modify `src/features/network.rs`:

```rust
pub use columns::{
    NetworkColumn,
    NetworkColumnSpec,
    NetworkColumns,
    NetworkLabelColumn,
    DEFAULT_NETWORK_COLUMNS,
};
```

- [ ] **Step 2: Add `build_network_label_registry` + `build_default_network_columns` in app.rs**

Modify `src/app.rs`. Find existing `build_config_label_registry` (from PR #1002). Add the Network equivalent right after:

```rust
/// Build the label-column registry for Network from config, erroring on
/// builtin name collisions or duplicate label headers.
fn build_network_label_registry(
    label_columns: &Option<Vec<LabelColumnConfig>>,
) -> Result<Vec<NetworkLabelColumn>> {
    let mut out: Vec<NetworkLabelColumn> = Vec::new();
    if let Some(defs) = label_columns {
        for def in defs {
            let norm = NetworkColumn::normalize_column(&def.name);
            if NetworkColumn::from_str(&norm).is_ok() {
                anyhow::bail!(
                    "label_columns name '{}' collides with a builtin column name",
                    def.name
                );
            }
            if let Some(existing) = out
                .iter()
                .find(|lc| NetworkColumn::normalize_column(&lc.name) == norm)
            {
                anyhow::bail!(
                    "label_columns name '{}' has the same header as previously defined '{}'",
                    def.name,
                    existing.name
                );
            }
            out.push(NetworkLabelColumn {
                name: def.name.clone(),
                key: def.label.clone(),
                header: def.name.to_uppercase(),
            });
        }
    }
    Ok(out)
}

/// Build the default Network columns for startup: all builtin defaults
/// followed by every label column registered in `registry`. `ensure_required`
/// and `dedup_columns` guarantee KIND/NAME are present and there are no
/// duplicates.
fn build_default_network_columns(registry: &[NetworkLabelColumn]) -> NetworkColumns {
    let mut specs: Vec<NetworkColumnSpec> = DEFAULT_NETWORK_COLUMNS
        .iter()
        .copied()
        .map(NetworkColumnSpec::Builtin)
        .collect();
    for lc in registry {
        specs.push(NetworkColumnSpec::Label {
            key: lc.key.clone(),
            header: lc.header.clone(),
        });
    }
    NetworkColumns::new(specs).ensure_required().dedup_columns()
}
```

Extend the existing `crate::features::network::...` import (or add a new line):

```rust
use crate::features::network::{
    NetworkColumn,
    NetworkColumnSpec,
    NetworkColumns,
    NetworkLabelColumn,
    DEFAULT_NETWORK_COLUMNS,
};
```

- [ ] **Step 3: Build the registry and default columns at startup**

In `src/app.rs`, find where the Config registry is built (from PR #1002):

```rust
let config_label_registry =
    build_config_label_registry(&config.theme.config.label_columns)?;
let default_config_columns = build_default_config_columns(&config_label_registry);
```

Add immediately after:

```rust
let network_label_registry =
    build_network_label_registry(&config.theme.network.label_columns)?;
let default_network_columns = build_default_network_columns(&network_label_registry);
```

- [ ] **Step 4: Add `default_network_columns` to `KubeWorkerConfig`**

Modify `src/workers/kube/config.rs`. Add a new field next to `default_config_columns`:

```rust
pub struct KubeWorkerConfig {
    // ... existing fields ...
    pub default_config_columns: ConfigColumns,
    pub default_network_columns: NetworkColumns,  // NEW
}
```

Add `NetworkColumns` import at the top.

- [ ] **Step 5: Assign default_network_columns in app.rs**

In `src/app.rs`, after building `default_network_columns`, assign it (mirror the existing Config assignment from PR #1002):

```rust
kube_worker_config.default_config_columns = default_config_columns.clone();
kube_worker_config.default_network_columns = default_network_columns.clone();  // NEW
```

- [ ] **Step 6: Update controller to use the configured default**

In `src/workers/kube/controller.rs`, find:

```rust
let shared_network_columns: SharedNetworkColumns =
    Arc::new(RwLock::new(NetworkColumns::default()));
```

Replace with:

```rust
let shared_network_columns: SharedNetworkColumns =
    Arc::new(RwLock::new(kube_worker_config.default_network_columns.clone()));
```

(`kube_worker_config` should be accessible at the construction site — verify the surrounding code; the same access pattern was established by PR #1002 for `default_config_columns`.)

- [ ] **Step 7: Thread through `Render`**

Modify `src/workers/render.rs`:

Add fields to `Render` struct (after `default_config_columns` / `config_label_columns` from PR #1002):

```rust
pub struct Render {
    // ... existing ...
    default_config_columns: ConfigColumns,
    config_label_columns: Vec<ConfigLabelColumn>,
    default_network_columns: NetworkColumns,         // NEW
    network_label_columns: Vec<NetworkLabelColumn>,  // NEW
    // ... rest ...
}
```

Add corresponding parameters to `Render::new` (insert in same group):

```rust
pub fn new(
    // ... existing args ...
    default_config_columns: ConfigColumns,
    config_label_registry: Vec<ConfigLabelColumn>,
    default_network_columns: NetworkColumns,            // NEW
    network_label_registry: Vec<NetworkLabelColumn>,    // NEW
    // ... rest ...
) -> Self {
    Self {
        // ... existing ...
        default_config_columns,
        config_label_columns: config_label_registry,
        default_network_columns,
        network_label_columns: network_label_registry,
        // ...
    }
}
```

Add imports:

```rust
use crate::features::network::{NetworkColumns, NetworkLabelColumn};
```

In `src/app.rs`, find the existing `Render::new(...)` call and add the new args at the right position.

- [ ] **Step 8: Forward through `WindowInit`**

Modify `src/workers/render/window.rs`:

Add two fields to `WindowInit`:

```rust
pub struct WindowInit {
    // ... existing ...
    pub default_config_columns: ConfigColumns,
    pub config_label_columns: Vec<ConfigLabelColumn>,
    pub default_network_columns: NetworkColumns,         // NEW
    pub network_label_columns: Vec<NetworkLabelColumn>,  // NEW
    // ... rest ...
}
```

Where `WindowInit { .. }` is constructed in `render.rs`, populate the new fields with `self.default_network_columns.clone()` and `self.network_label_columns.clone()`.

In `WindowInit::run` (or wherever `NetworkTab::new(...)` is called), update the call:

```rust
let NetworkTab {
    tab: network_tab,
    network_filter_help_dialog,
    // network_columns_dialog added in Task 6
} = NetworkTab::new(
    "Network",
    &self.tx,
    &clipboard,
    self.split_mode,
    self.default_network_columns.clone(),
    self.network_label_columns.clone(),
    self.theme.component.clone(),
);
```

Add imports:

```rust
use crate::features::network::{NetworkColumns, NetworkLabelColumn};
```

- [ ] **Step 9: Update `NetworkTab::new` signature**

Modify `src/features/network/view/tab.rs`:

```rust
use crate::{
    clipboard::Clipboard,
    config::theme::WidgetThemeConfig,
    features::{
        component_id::NETWORK_TAB_ID,
        network::{
            view::widgets::{description_widget, network_filter_help_widget, network_widget},
            NetworkColumns,
            NetworkLabelColumn,
        },
    },
    message::Message,
    ui::{
        tab::{LayoutElement, NestedLayoutElement, NestedWidgetLayout, TabLayout},
        widget::Widget,
        Tab,
    },
};

pub struct NetworkTab {
    pub tab: Tab<'static>,
    pub network_filter_help_dialog: Widget<'static>,
    // network_columns_dialog added in Task 6
}

impl NetworkTab {
    pub fn new(
        title: &'static str,
        tx: &Sender<Message>,
        clipboard: &Option<Rc<RefCell<Clipboard>>>,
        split_direction: Direction,
        default_columns: NetworkColumns,
        label_registry: Vec<NetworkLabelColumn>,
        theme: WidgetThemeConfig,
    ) -> Self {
        let error_theme = theme.error.clone().into();

        // `default_columns` will be consumed by the column dialog in Task 6.
        let _ = &default_columns;

        let network_widget = network_widget(tx, label_registry.clone(), theme.clone());
        let description_widget = description_widget(clipboard, theme.clone());
        let network_filter_help_dialog = network_filter_help_widget(theme);

        let layout = TabLayout::new(layout, split_direction);

        NetworkTab {
            tab: Tab::new(
                NETWORK_TAB_ID,
                title,
                [network_widget, description_widget],
                layout,
            )
            .error_theme(error_theme),
            network_filter_help_dialog,
        }
    }
}
```

(Preserve any existing `fn layout` and other items in the file.)

- [ ] **Step 10: Update `network_widget`**

Modify `src/features/network/view/widgets/network.rs`:

Add `label_registry: Vec<NetworkLabelColumn>` arg. Suppress with `let _ = &label_registry;` because the filter applicator update is Task 7.

```rust
use crate::{
    config::theme::WidgetThemeConfig,
    features::{
        component_id::{NETWORK_DESCRIPTION_WIDGET_ID, NETWORK_WIDGET_ID},
        network::{
            message::{NetworkRequest, NetworkRequestTargetParams},
            network_filter_applicator,
            NetworkLabelColumn,
        },
    },
    kube::apis::networking::gateway::v1::{Gateway, HTTPRoute},
    message::Message,
    ui::{
        event::EventResult,
        widget::{
            FilterForm,
            FilterFormTheme,
            Table,
            TableItem,
            TableTheme,
            Widget,
            WidgetBase,
            WidgetTheme,
            WidgetTrait as _,
        },
        Window,
        WindowAction,
    },
};

pub fn network_widget(
    tx: &Sender<Message>,
    label_registry: Vec<NetworkLabelColumn>,
    theme: WidgetThemeConfig,
) -> Widget<'static> {
    // Consumed in Task 7 by network_filter_applicator
    let _ = &label_registry;

    let tx = tx.clone();
    // ... rest unchanged including `.filter_applicator(network_filter_applicator(tx.clone()))`
}
```

(Do not change the `filter_applicator(...)` call — Task 7 will.)

- [ ] **Step 11: Build, test, fmt**

```bash
cargo build 2>&1 | rg "error" | head -5
cargo test --all 2>&1 | rg "test result:" | tail -3
cargo +nightly fmt
```

Expected: 0 errors. Tests unchanged (no new tests in this task).

- [ ] **Step 12: Commit**

```bash
git add src/app.rs src/features/network.rs src/workers/kube/config.rs src/workers/kube/controller.rs src/workers/render.rs src/workers/render/window.rs src/features/network/view/tab.rs src/features/network/view/widgets/network.rs
git commit -m "feat(network): registry + default columns wired from app.rs to widget"
```

---

## Task 6: Column dialog widget + action 't'

**Files:**
- Modify: `src/features/component_id.rs`
- Create: `src/features/network/view/widgets/network_columns_dialog.rs`
- Modify: `src/features/network/view/widgets.rs`
- Modify: `src/features/network/view/widgets/network.rs`
- Modify: `src/features/network/view/tab.rs`
- Modify: `src/workers/render/window.rs`

- [ ] **Step 1: Add component ID**

Modify `src/features/component_id.rs`. Find the existing `network_filter_help_dialog,` (from PR #999) and add `network_columns_dialog,` immediately after it.

- [ ] **Step 2: Create `src/features/network/view/widgets/network_columns_dialog.rs`**

```rust
use std::{collections::BTreeMap, str::FromStr as _};

use crossbeam::channel::Sender;
use strum::IntoEnumIterator;

use crate::{
    config::theme::WidgetThemeConfig,
    features::{
        component_id::NETWORK_COLUMNS_DIALOG_ID,
        network::{
            message::NetworkMessage,
            NetworkColumn,
            NetworkColumnSpec,
            NetworkColumns,
            NetworkLabelColumn,
        },
    },
    message::Message,
    ui::{
        event::EventResult,
        widget::{CheckList, CheckListItem, CheckListTheme, Widget, WidgetBase, WidgetTheme},
        Window,
    },
};

pub fn network_columns_dialog(
    tx: &Sender<Message>,
    default_columns: NetworkColumns,
    label_registry: Vec<NetworkLabelColumn>,
    theme: WidgetThemeConfig,
) -> Widget<'static> {
    let check_list_theme = CheckListTheme::from(theme.clone());
    let widget_theme = WidgetTheme::from(theme.clone());
    let widget_base = WidgetBase::builder()
        .title("Network Columns")
        .theme(widget_theme)
        .build();

    let items = build_check_list_items(default_columns, &label_registry);

    CheckList::builder()
        .id(NETWORK_COLUMNS_DIALOG_ID)
        .widget_base(widget_base)
        .theme(check_list_theme)
        .items(items)
        .on_change(on_change(tx.clone()))
        .build()
        .into()
}

fn candidate_specs(label_registry: &[NetworkLabelColumn]) -> Vec<NetworkColumnSpec> {
    NetworkColumn::iter()
        .map(NetworkColumnSpec::Builtin)
        .chain(label_registry.iter().map(|lc| {
            NetworkColumnSpec::Label {
                key: lc.key.clone(),
                header: lc.header.clone(),
            }
        }))
        .collect()
}

fn build_check_list_items(
    default_columns: NetworkColumns,
    label_registry: &[NetworkLabelColumn],
) -> Vec<CheckListItem> {
    let candidates = candidate_specs(label_registry);
    let current = default_columns;

    current
        .specs()
        .iter()
        .map(|spec| make_item(spec, true))
        .chain(
            candidates
                .iter()
                .filter(|spec| !current.specs().contains(spec))
                .map(|spec| make_item(spec, false)),
        )
        .collect()
}

fn make_item(spec: &NetworkColumnSpec, checked: bool) -> CheckListItem {
    CheckListItem {
        label: spec.header(),
        checked,
        required: matches!(
            spec,
            NetworkColumnSpec::Builtin(NetworkColumn::Kind)
                | NetworkColumnSpec::Builtin(NetworkColumn::Name)
        ),
        metadata: Some(metadata_for(spec)),
    }
}

fn metadata_for(spec: &NetworkColumnSpec) -> BTreeMap<String, String> {
    match spec {
        NetworkColumnSpec::Builtin(c) => {
            BTreeMap::from([
                ("kind".to_string(), "builtin".to_string()),
                ("id".to_string(), c.as_str().to_string()),
            ])
        }
        NetworkColumnSpec::Label { key, header } => {
            BTreeMap::from([
                ("kind".to_string(), "label".to_string()),
                ("key".to_string(), key.clone()),
                ("header".to_string(), header.clone()),
            ])
        }
    }
}

fn spec_from_item(item: &CheckListItem) -> Option<NetworkColumnSpec> {
    let md = item.metadata.as_ref()?;
    match md.get("kind").map(String::as_str) {
        Some("builtin") => NetworkColumn::from_str(md.get("id")?)
            .ok()
            .map(NetworkColumnSpec::Builtin),
        Some("label") => Some(NetworkColumnSpec::Label {
            key: md.get("key")?.clone(),
            header: md.get("header")?.clone(),
        }),
        _ => None,
    }
}

fn collect_columns(items: &[CheckListItem]) -> NetworkColumns {
    let specs: Vec<NetworkColumnSpec> = items
        .iter()
        .filter(|item| item.required || item.checked)
        .filter_map(spec_from_item)
        .collect();

    NetworkColumns::new(specs).ensure_required()
}

fn on_change(tx: Sender<Message>) -> impl Fn(&mut Window, &CheckListItem) -> EventResult {
    move |w: &mut Window, _v| {
        let widget = w
            .find_widget_mut(NETWORK_COLUMNS_DIALOG_ID)
            .as_mut_check_list();
        let columns = collect_columns(widget.items());
        tx.send(NetworkMessage::ColumnsRequest(columns).into())
            .expect("Failed to send NetworkMessage::ColumnsRequest");
        EventResult::Nop
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn label_spec(key: &str, header: &str) -> NetworkColumnSpec {
        NetworkColumnSpec::Label {
            key: key.into(),
            header: header.into(),
        }
    }

    #[test]
    fn 選択列を先頭にその他候補を未チェックで並べる() {
        let registry = vec![NetworkLabelColumn {
            name: "app".into(),
            key: "app.kubernetes.io/name".into(),
            header: "APP".into(),
        }];
        let current = NetworkColumns::new([
            NetworkColumnSpec::Builtin(NetworkColumn::Kind),
            NetworkColumnSpec::Builtin(NetworkColumn::Name),
            label_spec("app.kubernetes.io/name", "APP"),
        ]);

        let items = build_check_list_items(current, &registry);

        assert_eq!(items[0].label, "KIND");
        assert!(items[0].checked);
        assert_eq!(items[1].label, "NAME");
        assert!(items[1].checked);
        assert_eq!(items[2].label, "APP");
        assert!(items[2].checked);
        assert!(items[3..].iter().all(|i| !i.checked));
    }

    #[test]
    fn collect_columns_は表示順を維持しensure_requiredが補う() {
        let items = vec![
            make_item(&label_spec("app.kubernetes.io/name", "APP"), true),
            make_item(&NetworkColumnSpec::Builtin(NetworkColumn::Kind), true),
            make_item(&NetworkColumnSpec::Builtin(NetworkColumn::Name), true),
            make_item(&NetworkColumnSpec::Builtin(NetworkColumn::Age), false),
        ];

        let columns = collect_columns(&items);

        assert_eq!(
            columns.specs(),
            &[
                label_spec("app.kubernetes.io/name", "APP"),
                NetworkColumnSpec::Builtin(NetworkColumn::Kind),
                NetworkColumnSpec::Builtin(NetworkColumn::Name),
            ]
        );
    }

    #[test]
    fn メタデータからspecを復元できる() {
        let builtin = make_item(&NetworkColumnSpec::Builtin(NetworkColumn::Age), true);
        let label = make_item(&label_spec("k", "APP"), true);

        assert_eq!(
            spec_from_item(&builtin),
            Some(NetworkColumnSpec::Builtin(NetworkColumn::Age))
        );
        assert_eq!(spec_from_item(&label), Some(label_spec("k", "APP")));
    }

    #[test]
    fn kind_と_name_は_required() {
        let kind = make_item(&NetworkColumnSpec::Builtin(NetworkColumn::Kind), true);
        let name = make_item(&NetworkColumnSpec::Builtin(NetworkColumn::Name), true);
        let age = make_item(&NetworkColumnSpec::Builtin(NetworkColumn::Age), true);
        let label = make_item(&label_spec("k", "APP"), true);

        assert!(kind.required);
        assert!(name.required);
        assert!(!age.required);
        assert!(!label.required);
    }
}
```

- [ ] **Step 3: Register dialog module**

Modify `src/features/network/view/widgets.rs`:

```rust
mod description;
mod network;
mod network_columns_dialog;
mod network_filter_help;

pub(super) use description::*;
pub(super) use network::*;
pub(super) use network_columns_dialog::*;
pub(super) use network_filter_help::*;
```

- [ ] **Step 4: Add action 't' to `network_widget`**

Modify `src/features/network/view/widgets/network.rs`. Add to the `Table::builder()` chain — insert `.action('t', open_network_columns_dialog())` immediately before `.block_injection(...)`:

```rust
Table::builder()
    .id(NETWORK_WIDGET_ID)
    .widget_base(widget_base)
    .filter_form(filter_form)
    .theme(table_theme)
    .filter_applicator(network_filter_applicator(tx.clone()))
    .action('t', open_network_columns_dialog())  // NEW
    .block_injection(block_injection())
    .on_select(on_select(tx))
    .build()
    .into()
```

Keep `let _ = &label_registry;` (Task 7 consumes it).

Add the helper at file scope (insert above `block_injection`):

```rust
fn open_network_columns_dialog() -> impl Fn(&mut Window) -> EventResult {
    use crate::features::component_id::NETWORK_COLUMNS_DIALOG_ID;
    |w: &mut Window| {
        w.open_dialog(NETWORK_COLUMNS_DIALOG_ID);
        EventResult::Nop
    }
}
```

- [ ] **Step 5: Build dialog in `NetworkTab::new` and expose field**

Modify `src/features/network/view/tab.rs`:

```rust
use super::widgets::{
    description_widget,
    network_columns_dialog,
    network_filter_help_widget,
    network_widget,
};

pub struct NetworkTab {
    pub tab: Tab<'static>,
    pub network_columns_dialog: Widget<'static>,        // NEW
    pub network_filter_help_dialog: Widget<'static>,
}

impl NetworkTab {
    pub fn new(
        title: &'static str,
        tx: &Sender<Message>,
        clipboard: &Option<Rc<RefCell<Clipboard>>>,
        split_direction: Direction,
        default_columns: NetworkColumns,
        label_registry: Vec<NetworkLabelColumn>,
        theme: WidgetThemeConfig,
    ) -> Self {
        let error_theme = theme.error.clone().into();

        let network_widget = network_widget(tx, label_registry.clone(), theme.clone());
        let description_widget = description_widget(clipboard, theme.clone());
        let network_columns_dialog =
            network_columns_dialog(tx, default_columns, label_registry, theme.clone());
        let network_filter_help_dialog = network_filter_help_widget(theme);

        let layout = TabLayout::new(layout, split_direction);

        NetworkTab {
            tab: Tab::new(
                NETWORK_TAB_ID,
                title,
                [network_widget, description_widget],
                layout,
            )
            .error_theme(error_theme),
            network_columns_dialog,
            network_filter_help_dialog,
        }
    }
}
```

(Drop the `let _ = &default_columns;` suppression from Task 5 — it's now consumed by the dialog constructor.)

- [ ] **Step 6: Register dialog in `window.rs`**

Modify `src/workers/render/window.rs`:

Update the `NetworkTab` destructure (find existing `let NetworkTab { tab: network_tab, network_filter_help_dialog } = ...`):

```rust
let NetworkTab {
    tab: network_tab,
    network_columns_dialog,         // NEW
    network_filter_help_dialog,
} = NetworkTab::new(
    // ... unchanged args ...
);
```

Add `network_columns_dialog` to the `dialog_widgets` vector immediately after `network_filter_help_dialog`:

```rust
let dialog_widgets = vec![
    // ... existing ...
    network_filter_help_dialog,
    network_columns_dialog,        // NEW
    // ... rest ...
];
```

- [ ] **Step 7: Build, test, fmt**

```bash
cargo build 2>&1 | rg "error" | head -5
cargo test --all 2>&1 | rg "test result:" | tail -3
cargo +nightly fmt
```

Expected: 0 errors. Test count: +4 new dialog tests.

- [ ] **Step 8: Commit**

```bash
git add src/features/component_id.rs src/features/network/view/widgets/network_columns_dialog.rs src/features/network/view/widgets.rs src/features/network/view/widgets/network.rs src/features/network/view/tab.rs src/workers/render/window.rs
git commit -m "feat(network-dialog): label-aware column dialog + 't' key binding"
```

---

## Task 7: Filter parser registry support

**Files:**
- Modify: `src/features/network/filter/parser.rs`
- Modify: `src/features/network/filter.rs`
- Modify: `src/features/network/view/widgets/network.rs`

- [ ] **Step 1: Update parser to accept registry**

Replace the contents of `src/features/network/filter/parser.rs` with:

```rust
//! Network filter parser.
//!
//! Delegates tokenization/quoting/predicate-building to the shared
//! `parse_table_filter`. The Network-specific part is the column validator:
//! `namespace:` returns a guidance message (namespace is a scope, not a
//! column-level filter — use the namespace selector); other unknown columns
//! return `unknown column '<x>'`; builtin `NetworkColumn`s and registered
//! label columns (whose header appears in `label_registry`) are accepted.

use std::collections::HashSet;

use strum::IntoEnumIterator;

use crate::{
    features::network::{NetworkColumn, NetworkLabelColumn},
    ui::widget::{normalize_column_name, parse_table_filter, TableFilterPredicate},
};

fn valid_columns(label_registry: &[NetworkLabelColumn]) -> HashSet<String> {
    let mut set: HashSet<String> = NetworkColumn::iter()
        .map(|c| normalize_column_name(c.display()))
        .collect();
    for lc in label_registry {
        set.insert(normalize_column_name(&lc.header));
    }
    set
}

pub fn parse_network_filter(
    input: &str,
    label_registry: &[NetworkLabelColumn],
) -> Result<TableFilterPredicate, String> {
    let valid = valid_columns(label_registry);
    parse_table_filter(input, |column| {
        let normalized = normalize_column_name(column);
        if normalized == "namespace" {
            return Err(
                "namespace is selected via the namespace selector, not the filter".to_string(),
            );
        }
        if valid.contains(&normalized) {
            Ok(())
        } else {
            Err(format!("unknown column '{}'", column))
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn no_label_cols() -> Vec<NetworkLabelColumn> {
        Vec::new()
    }

    fn registry_with(name: &str, header: &str) -> Vec<NetworkLabelColumn> {
        vec![NetworkLabelColumn {
            name: name.to_string(),
            key: "irrelevant.example.com/key".to_string(),
            header: header.to_string(),
        }]
    }

    #[test]
    fn empty_input_yields_empty_predicate() {
        let p = parse_network_filter("", &no_label_cols()).unwrap();
        assert!(p.column_includes.is_empty());
        assert!(p.column_excludes.is_empty());
        assert_eq!(p.label_selector, None);
    }

    #[test]
    fn bare_value_becomes_name_include() {
        let p = parse_network_filter("my-svc", &no_label_cols()).unwrap();
        let patterns = p.column_includes.get("name").expect("name column");
        assert!(patterns[0].is_match("my-svc-abc"));
    }

    #[test]
    fn builtin_columns_are_accepted() {
        let p = parse_network_filter("kind:Service !kind:Pod", &no_label_cols()).unwrap();
        assert!(p.column_includes.contains_key("kind"));
        assert!(p.column_excludes.contains_key("kind"));
    }

    #[test]
    fn age_column_is_accepted() {
        let p = parse_network_filter("age:1d", &no_label_cols()).unwrap();
        assert!(p.column_includes.contains_key("age"));
    }

    #[test]
    fn data_column_is_rejected_for_network() {
        // DATA belongs to Config, not Network.
        let err = parse_network_filter("data:0", &no_label_cols()).unwrap_err();
        assert!(err.contains("unknown column") && err.contains("data"));
    }

    #[test]
    fn label_selector_is_captured() {
        let p = parse_network_filter("label:app=nginx", &no_label_cols()).unwrap();
        assert_eq!(p.label_selector.as_deref(), Some("app=nginx"));
    }

    #[test]
    fn unknown_column_produces_parse_error() {
        let err = parse_network_filter("staus:Active", &no_label_cols()).unwrap_err();
        assert!(err.contains("unknown column") && err.contains("staus"));
    }

    #[test]
    fn namespace_returns_guidance_message() {
        let err = parse_network_filter("namespace:default", &no_label_cols()).unwrap_err();
        assert_eq!(
            err,
            "namespace is selected via the namespace selector, not the filter"
        );
    }

    #[test]
    fn registered_label_column_header_is_accepted() {
        let regs = registry_with("app", "APP");
        let p = parse_network_filter("app:nginx", &regs).unwrap();
        assert!(p.column_includes.contains_key("app"));
    }

    #[test]
    fn namespace_guidance_precedes_registry_even_on_collision() {
        let regs = registry_with("namespace", "NAMESPACE");
        let err = parse_network_filter("namespace:default", &regs).unwrap_err();
        assert_eq!(
            err,
            "namespace is selected via the namespace selector, not the filter"
        );
    }

    #[test]
    fn quoted_value_with_whitespace() {
        let p = parse_network_filter(r#"name:"my service""#, &no_label_cols()).unwrap();
        let patterns = p.column_includes.get("name").unwrap();
        assert!(patterns[0].is_match("my service"));
    }
}
```

- [ ] **Step 2: Update applicator to take registry**

Modify `src/features/network/filter.rs`:

```rust
mod parser;

use crossbeam::channel::Sender;

use crate::{
    features::{
        component_id::NETWORK_FILTER_HELP_DIALOG_ID,
        network::{message::NetworkMessage, NetworkLabelColumn},
    },
    message::Message,
    ui::{
        widget::{
            ApplyStrategy,
            OnFilterApply,
            OnFilterCancel,
            TableFilterApplicator,
            TableFilterParser,
        },
        Window,
    },
};

pub use parser::parse_network_filter;

pub fn network_filter_applicator(
    label_registry: Vec<NetworkLabelColumn>,
    tx: Sender<Message>,
) -> TableFilterApplicator {
    let parser: TableFilterParser =
        (move |input: &str| parse_network_filter(input, &label_registry)).into();

    let tx_apply = tx.clone();
    let tx_cancel = tx;

    let on_apply: OnFilterApply = (move |predicate: &crate::ui::widget::TableFilterPredicate,
                                         _window: &mut Window| {
        tx_apply
            .send(NetworkMessage::Filter(predicate.label_selector.clone()).into())
            .expect("Failed to send NetworkMessage::Filter");
    })
    .into();

    let on_cancel: OnFilterCancel = (move |_window: &mut Window| {
        tx_cancel
            .send(NetworkMessage::Filter(None).into())
            .expect("Failed to send NetworkMessage::Filter(None) on cancel");
    })
    .into();

    TableFilterApplicator::new(parser, ApplyStrategy::EnterToConfirm)
        .with_help_dialog(NETWORK_FILTER_HELP_DIALOG_ID)
        .with_on_apply(on_apply)
        .with_on_cancel(on_cancel)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applicator_constructs_without_panic() {
        let (tx, _rx) = crossbeam::channel::bounded(1);
        let _ = network_filter_applicator(Vec::new(), tx);
    }
}
```

- [ ] **Step 3: Update widget call site**

Modify `src/features/network/view/widgets/network.rs`. Drop the `let _ = &label_registry;` line. Update applicator call:

```rust
.filter_applicator(network_filter_applicator(label_registry, tx.clone()))
```

- [ ] **Step 4: Build, test, fmt**

```bash
cargo build 2>&1 | rg "error" | head -5
cargo test --all 2>&1 | rg "test result:" | tail -3
cargo +nightly fmt
```

Expected: 0 errors. Parser tests: 11 cases pass.

- [ ] **Step 5: Commit**

```bash
git add src/features/network/filter/parser.rs src/features/network/filter.rs src/features/network/view/widgets/network.rs
git commit -m "feat(network-filter): accept label_registry; registered headers are valid columns"
```

---

## Task 8: Final verification + PR

- [ ] **Step 1: Run all gates**

```bash
cargo build 2>&1 | rg "error|warning: " | rg -v "kubeconfig" | head -10
cargo test --all 2>&1 | rg "test result:" | tail -3
cargo clippy --all-targets 2>&1 | rg "src/features/network|src/app\.rs|src/config/theme/network|src/workers" | rg -v "^   --" | head -10
cargo +nightly fmt --check 2>&1 | head -5
```

Expected:
- build: clean (only pre-existing `try_from_kubeconfig` warning)
- test: ~756 passed (725 from main + ~31 new = ~756)
- clippy: no new warning categories (`too_many_arguments` on `Render::new`/`WindowInit::new` may widen further)
- fmt: clean

Apply fmt if needed:

```bash
cargo +nightly fmt
```

- [ ] **Step 2: Push and create PR**

```bash
git push -u origin feat/network-label-columns
gh pr create --title "feat(network): label_columns + column dialog (Config #1002 mirror)" --body "$(cat <<'EOF'
## Summary

Add label_columns + column dialog to the Network tab. Users can declare `theme.network.label_columns` and the values appear as table columns (toggleable via the new `t`-key dialog). KIND and NAME are required (cannot be unchecked). No CLI args, no presets.

Mirrors PR #1002 (Config label_columns) which mirrored PR #993 (Pod label_columns). Completes the label_columns rollout across all aggregated-view tabs.

## What changed

| Surface | Change |
|---|---|
| Types | New `NetworkColumn` / `NetworkColumnSpec::{Builtin, Label}` / `NetworkColumns` / `NetworkLabelColumn` |
| Schema | New `NetworkThemeConfig { label_columns: Option<Vec<LabelColumnConfig>> }` |
| Message | New `NetworkMessage::ColumnsRequest(NetworkColumns)` variant |
| Controller | `SharedNetworkColumns`, wired through `EventControllerArgs`/`EventController`, message handler |
| Poller | `NetworkPoller` takes `SharedNetworkColumns`; spec-driven row construction via new `build_network_row_cells` helper; label values from `row.object.metadata.labels[key]`; `target_columns` dynamic from specs. `NetworkTable` wrapper removed; header + rows built in `polling()`. Existing 6+ sub-resource fan-out (Service/Ingress/Pod/NetworkPolicy/Gateway/HTTPRoute) preserved. |
| app.rs | `build_network_label_registry` + `build_default_network_columns`; threaded into `KubeWorkerConfig` / `Render` / `WindowInit` / `NetworkTab` |
| Dialog | New `network_columns_dialog` widget; `t` key opens it; KIND and NAME marked `required: true` |
| Filter parser | `parse_network_filter(input, &[NetworkLabelColumn])` accepts registered label headers as valid columns; `namespace:` guidance preserved |

## Test plan

- [x] `cargo build`: clean
- [x] `cargo test --all`: ~756 passed / 0 failed
- [x] `cargo clippy --all-targets`: no new warning categories
- [x] `cargo +nightly fmt --check`: clean
- [ ] Manual GKE smoke:
  - [ ] With `theme.network.label_columns: [{name: app, label: app.kubernetes.io/name}]`, APP column appears at startup
  - [ ] Press `t` → "Network Columns" dialog opens; APP is toggleable; KIND/NAME cannot be unchecked
  - [ ] Toggle APP off → next poll the column disappears; back on → returns
  - [ ] Filter `app:nginx` works (registered header accepted)
  - [ ] Filter `kind:Service app:nginx` applies AND across builtin + label
  - [ ] `namespace:default` still returns guidance
  - [ ] `label:app=nginx` still applies server-side `?labelSelector=` to all 6+ sub-fetches (unchanged from PR #999)
  - [ ] Unknown column `foo:bar` produces `unknown column 'foo'`
  - [ ] `data:0` produces unknown column (DATA is Config-only)

## Related

- Spec: `docs/superpowers/specs/2026-06-03-config-network-label-columns-design.md` (shared spec for Config + Network)
- Plan: `docs/superpowers/plans/2026-06-03-network-label-columns.md`
- Mirrors PR #1002 (Config label_columns)
- Completes the label_columns rollout for all aggregated-view tabs
EOF
)"
```

- [ ] **Step 3: Manual GKE verification + final test plan update**

After PR creation, perform manual smoke testing (mirror the 10-step Config verification from PR #1002 review) using a real cluster. Update the PR description with checked-off boxes via `gh pr edit <pr> --body ...`.

---

## Notes

- The poller refactor in Task 4 is the largest single change because Network had a deeper intermediate (`NetworkTable` wrapper) that we collapse. The simplification matches Config's pattern.
- Tasks 5 and 6 together replace the suppressed `let _ = &default_columns;` and `let _ = &label_registry;` placeholders with real consumers.
- All 6+ sub-resource types (Service, Ingress, Pod, NetworkPolicy, Gateway V1/V1Beta1, HTTPRoute V1/V1Beta1) automatically benefit — the spec-driven row construction is per-resource via `build_network_row_cells`.
- This PR completes the label_columns rollout. Config/Network/Pod/Node all have the same UX shape after this lands.
