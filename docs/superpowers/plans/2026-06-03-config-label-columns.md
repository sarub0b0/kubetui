# Config label_columns + Column dialog Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Config タブに `label_columns` (config 由来) と column dialog を追加し、Pod #993 と同等の UX を実現する。

**Architecture:** Pod #993 (`feat/pod-label-columns`) と同型の構造を Config に mirror。`ConfigColumnSpec::{Builtin, Label}` + `ConfigColumns` + `ConfigLabelColumn` + registry-aware filter parser + spec-driven poller + metadata-roundtrip column dialog。CLI と presets は無し。KIND と NAME が dialog で OFF 不可の必須列。

**Tech Stack:** Rust 2021, `tokio` async, `crossbeam` channel, `ratatui`, `serde`, `strum` EnumIter, `percent-encoding`。

**Spec:** `docs/superpowers/specs/2026-06-03-config-network-label-columns-design.md`

---

## File Structure

### New files

- `src/features/config/columns.rs` — `ConfigColumn` enum、`ConfigColumnSpec`、`ConfigLabelColumn`、`ConfigColumns` 型
- `src/config/theme/config.rs` — `ConfigThemeConfig` (config schema)
- `src/features/config/view/widgets/config_columns_dialog.rs` — column dialog widget

### Modified files

- `src/features/config.rs` — `mod columns;` + re-export
- `src/config/theme.rs` — `mod config;` + re-export + `ThemeConfig.config` フィールド
- `src/features/config/message.rs` — `ConfigMessage::ColumnsRequest(ConfigColumns)` variant 追加
- `src/workers/kube/controller.rs` — `SharedConfigColumns` 型追加、`EventControllerArgs`/`EventController` フィールド追加、destructure 更新、message handler 追加、`ConfigPoller::new` 呼び出し更新
- `src/features/config/kube/config.rs` — `ConfigPoller` が `SharedConfigColumns` を受領、poller を spec 駆動化、`build_config_row_cells` helper を抽出
- `src/app.rs` — `build_config_label_registry`/`build_default_config_columns` 追加、起動時 wiring、`Render::new` に `default_config_columns`/`config_label_columns` を渡す
- `src/workers/render.rs` — `Render` 構造体に `default_config_columns`/`config_label_columns` フィールド追加、`WindowInit` に同様
- `src/workers/render/window.rs` — `WindowInit` 構造体に追加、`ConfigTab::new` に渡す、`ConfigTab` から `config_columns_dialog` を destructure し global dialog list に追加
- `src/features/config/view/tab.rs` — `ConfigTab` に `config_columns_dialog` フィールド追加、`new` シグネチャ拡張
- `src/features/config/view/widgets/config.rs` — `config_widget` に `label_registry` 引数追加、action `'t'` 追加、`config_filter_applicator(label_registry, tx)` で呼ぶ
- `src/features/config/view/widgets.rs` — `mod config_columns_dialog;` + re-export
- `src/features/config/filter/parser.rs` — `parse_config_filter(input, &[ConfigLabelColumn])` に変更
- `src/features/config/filter.rs` — `config_filter_applicator(label_registry, tx)` に変更
- `src/features/component_id.rs` — `config_columns_dialog` 追加

---

## Pre-flight

リポジトリの現状ブランチ確認:

- [ ] 作業ブランチ `feat/config-label-columns` に居ること、spec commit (`70e12dfb`) が HEAD に含まれること

```bash
git branch --show-current  # → feat/config-label-columns
git log --oneline -1       # → spec commit
```

---

## Task 1: 型 (ConfigColumn / ConfigColumnSpec / ConfigLabelColumn / ConfigColumns)

**Files:**
- Create: `src/features/config/columns.rs`
- Modify: `src/features/config.rs`

- [ ] **Step 1: Write `src/features/config/columns.rs`**

```rust
use strum::{EnumIter, IntoEnumIterator};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigColumnSpec {
    Builtin(ConfigColumn),
    Label { key: String, header: String },
}

impl ConfigColumnSpec {
    pub fn header(&self) -> String {
        match self {
            ConfigColumnSpec::Builtin(c) => c.display().to_string(),
            ConfigColumnSpec::Label { header, .. } => header.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigLabelColumn {
    pub name: String,
    pub key: String,
    pub header: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigColumns {
    columns: Vec<ConfigColumnSpec>,
}

impl Default for ConfigColumns {
    fn default() -> Self {
        ConfigColumns::from_builtins(DEFAULT_CONFIG_COLUMNS.iter().copied())
    }
}

impl ConfigColumns {
    pub fn new(columns: impl IntoIterator<Item = ConfigColumnSpec>) -> Self {
        ConfigColumns {
            columns: columns.into_iter().collect(),
        }
    }

    pub fn from_builtins(columns: impl IntoIterator<Item = ConfigColumn>) -> Self {
        ConfigColumns {
            columns: columns.into_iter().map(ConfigColumnSpec::Builtin).collect(),
        }
    }

    pub fn specs(&self) -> &[ConfigColumnSpec] {
        &self.columns
    }

    /// KIND と NAME を canonical 順 (KIND 先頭, NAME 2 番目) で強制配置。
    pub fn ensure_required(mut self) -> Self {
        let has_kind = self
            .columns
            .iter()
            .any(|s| matches!(s, ConfigColumnSpec::Builtin(ConfigColumn::Kind)));
        if !has_kind {
            self.columns
                .insert(0, ConfigColumnSpec::Builtin(ConfigColumn::Kind));
        }

        let kind_pos = self
            .columns
            .iter()
            .position(|s| matches!(s, ConfigColumnSpec::Builtin(ConfigColumn::Kind)))
            .expect("Kind just ensured");
        let has_name = self
            .columns
            .iter()
            .any(|s| matches!(s, ConfigColumnSpec::Builtin(ConfigColumn::Name)));
        if !has_name {
            self.columns.insert(
                kind_pos + 1,
                ConfigColumnSpec::Builtin(ConfigColumn::Name),
            );
        }

        self
    }

    /// 順序を保ちながら重複を排除。
    pub fn dedup_columns(self) -> Self {
        let mut unique: Vec<ConfigColumnSpec> = Vec::new();
        for spec in self.columns {
            if !unique.contains(&spec) {
                unique.push(spec);
            }
        }
        ConfigColumns { columns: unique }
    }
}

pub const DEFAULT_CONFIG_COLUMNS: &[ConfigColumn] = &[
    ConfigColumn::Kind,
    ConfigColumn::Name,
    ConfigColumn::Data,
    ConfigColumn::Age,
];

#[derive(EnumIter, PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy, Hash)]
pub enum ConfigColumn {
    Kind,
    Name,
    Data,
    Age,
}

impl ConfigColumn {
    pub const fn as_str(&self) -> &'static str {
        match self {
            ConfigColumn::Kind => "Kind",
            ConfigColumn::Name => "Name",
            ConfigColumn::Data => "Data",
            ConfigColumn::Age => "Age",
        }
    }

    pub const fn display(&self) -> &'static str {
        match self {
            ConfigColumn::Kind => "KIND",
            ConfigColumn::Name => "NAME",
            ConfigColumn::Data => "DATA",
            ConfigColumn::Age => "AGE",
        }
    }

    pub fn normalize_column(column: &str) -> String {
        column.to_lowercase().replace([' ', '_', '-'], "")
    }
}

#[derive(Debug)]
pub struct ConfigColumnParseError;

impl std::fmt::Display for ConfigColumnParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid ConfigColumn string representation")
    }
}

impl std::error::Error for ConfigColumnParseError {}

impl std::str::FromStr for ConfigColumn {
    type Err = ConfigColumnParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match Self::normalize_column(s).as_str() {
            "kind" => Ok(ConfigColumn::Kind),
            "name" => Ok(ConfigColumn::Name),
            "data" => Ok(ConfigColumn::Data),
            "age" => Ok(ConfigColumn::Age),
            _ => Err(ConfigColumnParseError),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn builtins(cols: &[ConfigColumn]) -> Vec<ConfigColumnSpec> {
        cols.iter().copied().map(ConfigColumnSpec::Builtin).collect()
    }

    #[test]
    fn default_has_kind_name_data_age_in_order() {
        let cols = ConfigColumns::default();
        assert_eq!(
            cols.specs(),
            builtins(&[
                ConfigColumn::Kind,
                ConfigColumn::Name,
                ConfigColumn::Data,
                ConfigColumn::Age,
            ])
            .as_slice()
        );
    }

    #[test]
    fn ensure_required_inserts_both_when_absent() {
        let cols = ConfigColumns::from_builtins([ConfigColumn::Data, ConfigColumn::Age])
            .ensure_required();
        assert_eq!(
            cols.specs(),
            builtins(&[
                ConfigColumn::Kind,
                ConfigColumn::Name,
                ConfigColumn::Data,
                ConfigColumn::Age,
            ])
            .as_slice()
        );
    }

    #[test]
    fn ensure_required_inserts_name_after_existing_kind() {
        let cols = ConfigColumns::from_builtins([ConfigColumn::Kind, ConfigColumn::Age])
            .ensure_required();
        assert_eq!(
            cols.specs(),
            builtins(&[
                ConfigColumn::Kind,
                ConfigColumn::Name,
                ConfigColumn::Age,
            ])
            .as_slice()
        );
    }

    #[test]
    fn ensure_required_inserts_kind_when_only_name_present() {
        let cols = ConfigColumns::from_builtins([ConfigColumn::Name, ConfigColumn::Age])
            .ensure_required();
        assert_eq!(
            cols.specs(),
            builtins(&[
                ConfigColumn::Kind,
                ConfigColumn::Name,
                ConfigColumn::Age,
            ])
            .as_slice()
        );
    }

    #[test]
    fn ensure_required_preserves_order_when_both_present() {
        let cols = ConfigColumns::from_builtins([
            ConfigColumn::Name,
            ConfigColumn::Kind,
            ConfigColumn::Age,
        ])
        .ensure_required();
        // Order preserved — not reordered to canonical.
        assert_eq!(
            cols.specs(),
            builtins(&[
                ConfigColumn::Name,
                ConfigColumn::Kind,
                ConfigColumn::Age,
            ])
            .as_slice()
        );
    }

    #[test]
    fn dedup_columns_removes_duplicates_preserving_first() {
        let cols = ConfigColumns::new([
            ConfigColumnSpec::Builtin(ConfigColumn::Kind),
            ConfigColumnSpec::Builtin(ConfigColumn::Name),
            ConfigColumnSpec::Builtin(ConfigColumn::Kind),
            ConfigColumnSpec::Builtin(ConfigColumn::Age),
        ])
        .dedup_columns();
        assert_eq!(
            cols.specs(),
            builtins(&[
                ConfigColumn::Kind,
                ConfigColumn::Name,
                ConfigColumn::Age,
            ])
            .as_slice()
        );
    }

    #[test]
    fn builtin_spec_header_is_uppercase_display() {
        assert_eq!(
            ConfigColumnSpec::Builtin(ConfigColumn::Kind).header(),
            "KIND"
        );
    }

    #[test]
    fn label_spec_header_is_as_given() {
        let s = ConfigColumnSpec::Label {
            key: "app.kubernetes.io/version".to_string(),
            header: "VERSION".to_string(),
        };
        assert_eq!(s.header(), "VERSION");
    }

    #[test]
    fn normalize_column_strips_space_underscore_hyphen_and_lowercases() {
        assert_eq!(ConfigColumn::normalize_column("KIND"), "kind");
        assert_eq!(ConfigColumn::normalize_column("config-map"), "configmap");
        assert_eq!(ConfigColumn::normalize_column("data_count"), "datacount");
    }

    #[test]
    fn from_str_accepts_normalized_forms() {
        use std::str::FromStr;
        assert!(matches!(
            ConfigColumn::from_str("KIND"),
            Ok(ConfigColumn::Kind)
        ));
        assert!(matches!(
            ConfigColumn::from_str("name"),
            Ok(ConfigColumn::Name)
        ));
        assert!(ConfigColumn::from_str("bogus").is_err());
    }
}
```

- [ ] **Step 2: Add module declaration in `src/features/config.rs`**

Modify `src/features/config.rs`:

```rust
mod columns;
mod filter;
pub mod kube;
pub mod message;
pub mod view;

pub use columns::{ConfigColumn, ConfigColumnSpec, ConfigColumns, ConfigLabelColumn};
pub use filter::config_filter_applicator;
```

(`pub use columns::{...}` を `pub use filter::...` の上に追加)

- [ ] **Step 3: Run tests**

```bash
cargo test --all columns 2>&1 | rg "test result:"
```

Expected: New tests pass. `cargo build` succeeds.

- [ ] **Step 4: Commit**

```bash
git add src/features/config/columns.rs src/features/config.rs
git commit -m "feat(config): introduce ConfigColumn/ConfigColumnSpec/ConfigColumns types"
```

---

## Task 2: Config schema (ConfigThemeConfig)

**Files:**
- Create: `src/config/theme/config.rs`
- Modify: `src/config/theme.rs`

- [ ] **Step 1: Create `src/config/theme/config.rs`**

```rust
use serde::{Deserialize, Serialize};

use super::LabelColumnConfig;

/// Theme/config-level settings for the Config tab.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct ConfigThemeConfig {
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
                { "name": "app", "label": "app.kubernetes.io/name" },
                { "name": "version", "label": "app.kubernetes.io/version" }
            ]
        }"#;
        let cfg: ConfigThemeConfig = serde_json::from_str(json).unwrap();
        let labels = cfg.label_columns.as_ref().unwrap();
        assert_eq!(labels.len(), 2);
        assert_eq!(labels[0].name, "app");
        assert_eq!(labels[0].label, "app.kubernetes.io/name");
        assert_eq!(labels[1].name, "version");
        assert_eq!(labels[1].label, "app.kubernetes.io/version");
    }

    #[test]
    fn default_has_none_label_columns() {
        let cfg = ConfigThemeConfig::default();
        assert!(cfg.label_columns.is_none());
    }
}
```

- [ ] **Step 2: Wire module + ThemeConfig field**

Modify `src/config/theme.rs`:

Add `mod config;` near other mod declarations (alphabetical order in existing file — insert after `mod check_list;`):

```rust
mod check_list;
mod config;
mod dialog;
```

Add `pub use config::ConfigThemeConfig;` in the re-export section (alphabetical):

```rust
pub use check_list::*;
pub use config::ConfigThemeConfig;
pub use dialog::*;
```

Add `pub config: ConfigThemeConfig,` to `ThemeConfig` struct (between existing `pub pod` and `pub node` per the order they appear in `ThemeConfig`):

```rust
pub struct ThemeConfig {
    // ... existing fields up to and including pod ...

    #[serde(default)]
    pub pod: PodThemeConfig,

    #[serde(default)]
    pub config: ConfigThemeConfig,  // NEW

    #[serde(default)]
    pub node: NodeThemeConfig,

    // ... rest unchanged
}
```

(The exact existing field order should be preserved; insert `config` after `pod` because that mirrors the tab order in the UI.)

- [ ] **Step 3: Verify with build and tests**

```bash
cargo build 2>&1 | rg "error|warning: " | head -5
cargo test --all 2>&1 | rg "test result:" | tail -1
```

Expected: builds clean (only pre-existing `try_from_kubeconfig` warning), tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/config/theme.rs src/config/theme/config.rs
git commit -m "feat(config-theme): add ConfigThemeConfig with label_columns"
```

---

## Task 3: ConfigMessage::ColumnsRequest + SharedConfigColumns + Controller routing

**Files:**
- Modify: `src/features/config/message.rs`
- Modify: `src/workers/kube/controller.rs`

- [ ] **Step 1: Add `ConfigMessage::ColumnsRequest` variant**

Modify `src/features/config/message.rs`:

```rust
use crate::features::config::ConfigColumns;
```

(Add `ConfigColumns` import — top of file, with existing `use crate::...` imports.)

Modify the `ConfigMessage` enum:

```rust
#[derive(Debug)]
pub enum ConfigMessage {
    Request(ConfigRequest),
    Response(ConfigResponse),
    /// Replace the active labelSelector value. `None` clears it (the poller
    /// stops sending `?labelSelector=` in its sub-fetch URLs).
    Filter(Option<String>),
    /// Replace the active column composition (sent from the column dialog).
    /// The poller will use the new columns on the next poll.
    ColumnsRequest(ConfigColumns),
}
```

- [ ] **Step 2: Add `SharedConfigColumns` type in controller**

Modify `src/workers/kube/controller.rs`:

Find the line `pub type SharedConfigFilter = Arc<RwLock<Option<String>>>;` and add after it:

```rust
pub type SharedConfigColumns = Arc<RwLock<ConfigColumns>>;
```

Add the `ConfigColumns` import. Find the existing `use crate::features::config::...` line or add a new one in the top imports:

```rust
use crate::features::config::ConfigColumns;
```

(If a `use crate::features::config::...` already exists, extend it; otherwise add the line.)

- [ ] **Step 3: Add field to `EventControllerArgs` and `EventController`**

Find the `EventControllerArgs` struct and add `shared_config_columns: SharedConfigColumns,` after `shared_config_filter`:

```rust
struct EventControllerArgs {
    // ... existing fields ...
    shared_config_filter: SharedConfigFilter,
    shared_config_columns: SharedConfigColumns,  // NEW
    shared_network_filter: SharedNetworkFilter,
    // ... rest unchanged
}
```

Apply the **same insertion** to `EventController` struct and to `EventController::new` constructor body (`shared_config_columns: args.shared_config_columns,` after `shared_config_filter: args.shared_config_filter,`).

- [ ] **Step 4: Construct `shared_config_columns` and pass via args**

Find the section where `shared_config_filter` is constructed (around line 313):

```rust
let shared_config_filter: SharedConfigFilter = Arc::new(RwLock::new(None));
```

Add immediately after:

```rust
let shared_config_columns: SharedConfigColumns =
    Arc::new(RwLock::new(ConfigColumns::default()));
```

Find the `EventControllerArgs { ... }` literal and add the field:

```rust
shared_config_filter: shared_config_filter.clone(),
shared_config_columns: shared_config_columns.clone(),  // NEW
shared_network_filter: shared_network_filter.clone(),
```

- [ ] **Step 5: Add field to destructure in `run()`**

Find the `let Self { ... } = self;` destructure and add `shared_config_columns,` after `shared_config_filter,`:

```rust
shared_config_filter,
shared_config_columns,
shared_network_filter,
```

- [ ] **Step 6: Add message handler for `ColumnsRequest`**

Find the existing `Kube::Config(ConfigMessage::Filter(sel))` handler. Add another arm immediately after it:

```rust
Kube::Config(ConfigMessage::Filter(sel)) => {
    *shared_config_filter.write().await = sel;
}

Kube::Config(ConfigMessage::ColumnsRequest(columns)) => {
    *shared_config_columns.write().await = columns;
}
```

- [ ] **Step 7: Build and verify**

```bash
cargo build 2>&1 | rg "error" | head -5
```

Expected: 0 errors. ConfigPoller::new will still take only the existing args — it has not yet been modified to consume `shared_config_columns`. That's Task 5.

- [ ] **Step 8: Commit**

```bash
git add src/features/config/message.rs src/workers/kube/controller.rs
git commit -m "feat(config-msg): ColumnsRequest variant + SharedConfigColumns plumbing"
```

---

## Task 4: Poller spec-driven + label value rendering

**Files:**
- Modify: `src/features/config/kube/config.rs`

The poller currently builds rows by indexing fixed cell positions (`row.cells[indexes[0..3]]`). After this task, it iterates `ConfigColumnSpec`s from the shared columns and renders each cell accordingly (KIND from `ty.resource()`, builtin from API `row.cells[...]`, label from `row.object.metadata.labels[key]`).

The pure cell-building logic is extracted to `build_config_row_cells` so it can be unit-tested without the async HTTP layer.

- [ ] **Step 1: Add imports and `SharedConfigColumns` to `ConfigPoller`**

Modify `src/features/config/kube/config.rs`:

Update the `use crate::...` block:

```rust
use crate::{
    features::config::{
        message::ConfigResponse,
        ConfigColumn,
        ConfigColumnSpec,
        ConfigColumns,
    },
    kube::{
        apis::v1_table::TableRow,
        table::{get_resource_per_namespace, insert_ns, KubeTable, KubeTableRow},
        KubeClient,
    },
    logger,
    message::Message,
    workers::kube::{
        InfiniteWorker,
        SharedConfigColumns,
        SharedConfigFilter,
        SharedTargetNamespaces,
    },
};
```

Modify the `ConfigPoller` struct and `new` constructor:

```rust
#[derive(Clone)]
pub struct ConfigPoller {
    tx: Sender<Message>,
    shared_target_namespaces: SharedTargetNamespaces,
    shared_config_columns: SharedConfigColumns,
    shared_config_filter: SharedConfigFilter,
    kube_client: KubeClient,
}

impl ConfigPoller {
    pub fn new(
        tx: Sender<Message>,
        shared_target_namespaces: SharedTargetNamespaces,
        shared_config_columns: SharedConfigColumns,
        shared_config_filter: SharedConfigFilter,
        kube_client: KubeClient,
    ) -> Self {
        Self {
            tx,
            shared_target_namespaces,
            shared_config_columns,
            shared_config_filter,
            kube_client,
        }
    }
}
```

- [ ] **Step 2: Add `build_config_row_cells` helper at file scope**

Add the following standalone function near the existing top-level helpers (above `async fn fetch_configs_per_namespace`):

```rust
/// Build the per-row cell vector from a spec list, the resource's kind name,
/// and a k8s API `TableRow`.
///
/// `builtin_indexes` are the positional indexes into `row.cells` for the
/// non-KIND builtin columns (NAME / DATA / AGE), in the order specified by
/// the fetch's `target_columns`.
pub(crate) fn build_config_row_cells(
    specs: &[ConfigColumnSpec],
    kind: &str,
    row: &TableRow,
    builtin_indexes: &[usize],
) -> Vec<String> {
    let mut builtin_iter = builtin_indexes.iter();
    specs
        .iter()
        .map(|s| {
            match s {
                ConfigColumnSpec::Builtin(ConfigColumn::Kind) => kind.to_string(),
                ConfigColumnSpec::Builtin(_) => {
                    let i = builtin_iter.next().expect("builtin index available");
                    row.cells[*i].to_string()
                }
                ConfigColumnSpec::Label { key, .. } => {
                    row.object
                        .as_ref()
                        .and_then(|o| o.0.get("metadata"))
                        .and_then(|m| m.get("labels"))
                        .and_then(|l| l.get(key))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string()
                }
            }
        })
        .collect()
}
```

- [ ] **Step 3: Refactor `run` to read columns and pass to `fetch_configs`**

Replace the existing `impl InfiniteWorker for ConfigPoller` body:

```rust
#[async_trait]
impl InfiniteWorker for ConfigPoller {
    async fn run(&self) {
        let mut interval = tokio::time::interval(time::Duration::from_secs(1));

        let Self {
            tx,
            shared_target_namespaces,
            shared_config_columns,
            shared_config_filter,
            kube_client,
        } = self;

        loop {
            interval.tick().await;

            let target_namespaces = shared_target_namespaces.read().await;
            let columns = shared_config_columns.read().await.clone();
            let label_selector = shared_config_filter.read().await.clone();

            let table = fetch_configs(
                kube_client,
                &target_namespaces,
                &columns,
                label_selector.as_deref(),
            )
            .await;

            if let Err(e) = tx.send(ConfigResponse::Table(table).into()) {
                logger!(error, "Failed to send ConfigResponse::Table: {}", e);
                return;
            }
        }
    }
}
```

- [ ] **Step 4: Replace `fetch_configs` + `fetch_configs_per_namespace`**

Replace those two functions entirely with the spec-driven version:

```rust
async fn fetch_configs(
    client: &KubeClient,
    namespaces: &[String],
    columns: &ConfigColumns,
    label_selector: Option<&str>,
) -> Result<KubeTable> {
    let specs = columns.specs();

    let mut header: Vec<String> = specs.iter().map(|s| s.header()).collect();
    if namespaces.len() != 1 {
        header.insert(0, "NAMESPACE".to_string());
    }

    let jobs = try_join_all([
        fetch_configs_per_namespace(client, namespaces, Configs::ConfigMap, specs, label_selector),
        fetch_configs_per_namespace(client, namespaces, Configs::Secret, specs, label_selector),
    ])
    .await?;

    let mut table = KubeTable {
        header,
        ..Default::default()
    };
    table.update_rows(jobs.into_iter().flatten().collect());

    Ok(table)
}

async fn fetch_configs_per_namespace(
    client: &KubeClient,
    namespaces: &[String],
    ty: Configs,
    specs: &[ConfigColumnSpec],
    label_selector: Option<&str>,
) -> Result<Vec<KubeTableRow>> {
    let insert_ns = insert_ns(namespaces);
    let label_selector = label_selector.map(|s| s.to_string());

    // Build target_columns dynamically from specs (skip KIND — supplied by
    // ty.resource()), so the API only fetches what the user currently wants.
    // This keeps `builtin_indexes` aligned with the non-KIND Builtin entries
    // in spec order, even if the user toggles DATA off.
    let target_columns: Vec<&str> = specs
        .iter()
        .filter_map(|s| match s {
            ConfigColumnSpec::Builtin(ConfigColumn::Kind) => None,
            ConfigColumnSpec::Builtin(c) => Some(c.as_str()),
            ConfigColumnSpec::Label { .. } => None,
        })
        .collect();

    // Captures specs for use inside the per-namespace closures.
    let specs_owned: Vec<ConfigColumnSpec> = specs.to_vec();

    let jobs = try_join_all(namespaces.iter().map(|ns| {
        let specs_for_ns = specs_owned.clone();
        let base_path = ty.url_path(ns);
        let path = match label_selector.as_deref().filter(|s| !s.is_empty()) {
            Some(sel) => format!(
                "{}?labelSelector={}",
                base_path,
                utf8_percent_encode(sel, NON_ALPHANUMERIC)
            ),
            None => base_path,
        };
        get_resource_per_namespace(
            client,
            path,
            &target_columns,
            move |row: &TableRow, indexes: &[usize]| {
                let mut row_cells =
                    build_config_row_cells(&specs_for_ns, ty.resource(), row, indexes);

                let name_pos = specs_for_ns
                    .iter()
                    .position(|s| matches!(s, ConfigColumnSpec::Builtin(ConfigColumn::Name)))
                    .expect("Name column must be present in config columns");
                let name = row_cells[name_pos].clone();

                if insert_ns {
                    row_cells.insert(0, ns.to_string());
                }

                KubeTableRow {
                    namespace: ns.to_string(),
                    name,
                    row: row_cells,
                    metadata: Some(BTreeMap::from([(
                        "kind".to_string(),
                        ty.resource().to_string(),
                    )])),
                }
            },
        )
    }))
    .await?;

    Ok(jobs.into_iter().flatten().collect())
}
```

Imports needed (add to the top of file if missing — `percent_encoding` should already be present from PR #997):

```rust
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
```

- [ ] **Step 5: Add unit tests for `build_config_row_cells`**

Append at the bottom of `src/features/config/kube/config.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::kube::apis::v1_table::Value;
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
            ConfigColumnSpec::Builtin(ConfigColumn::Kind),
            ConfigColumnSpec::Builtin(ConfigColumn::Name),
            ConfigColumnSpec::Builtin(ConfigColumn::Data),
            ConfigColumnSpec::Builtin(ConfigColumn::Age),
        ];
        let row = make_row(&["my-cm", "5", "3h"]);
        let cells = build_config_row_cells(&specs, "ConfigMap", &row, &[0, 1, 2]);
        assert_eq!(cells, vec!["ConfigMap", "my-cm", "5", "3h"]);
    }

    #[test]
    fn label_arm_returns_value_when_label_present() {
        let specs = vec![
            ConfigColumnSpec::Builtin(ConfigColumn::Name),
            ConfigColumnSpec::Label {
                key: "app".to_string(),
                header: "APP".to_string(),
            },
        ];
        let row = make_row_with_labels(&["my-cm"], &[("app", "datadog")]);
        let cells = build_config_row_cells(&specs, "ConfigMap", &row, &[0]);
        assert_eq!(cells, vec!["my-cm", "datadog"]);
    }

    #[test]
    fn label_arm_returns_empty_when_label_absent() {
        let specs = vec![
            ConfigColumnSpec::Builtin(ConfigColumn::Name),
            ConfigColumnSpec::Label {
                key: "app".to_string(),
                header: "APP".to_string(),
            },
        ];
        let row = make_row_with_labels(&["my-cm"], &[("other", "x")]);
        let cells = build_config_row_cells(&specs, "ConfigMap", &row, &[0]);
        assert_eq!(cells, vec!["my-cm", ""]);
    }

    #[test]
    fn label_arm_returns_empty_when_no_object() {
        let specs = vec![
            ConfigColumnSpec::Builtin(ConfigColumn::Name),
            ConfigColumnSpec::Label {
                key: "app".to_string(),
                header: "APP".to_string(),
            },
        ];
        let row = make_row(&["my-cm"]);
        let cells = build_config_row_cells(&specs, "ConfigMap", &row, &[0]);
        assert_eq!(cells, vec!["my-cm", ""]);
    }

    #[test]
    fn mixed_builtin_and_label_in_spec_order() {
        let specs = vec![
            ConfigColumnSpec::Builtin(ConfigColumn::Kind),
            ConfigColumnSpec::Label {
                key: "env".to_string(),
                header: "ENV".to_string(),
            },
            ConfigColumnSpec::Builtin(ConfigColumn::Name),
            ConfigColumnSpec::Label {
                key: "team".to_string(),
                header: "TEAM".to_string(),
            },
            ConfigColumnSpec::Builtin(ConfigColumn::Age),
        ];
        // builtin order: Name (0), Age (1) in row.cells (Data is skipped from spec)
        let row =
            make_row_with_labels(&["my-cm", "3h"], &[("env", "prod"), ("team", "platform")]);
        let cells = build_config_row_cells(&specs, "Secret", &row, &[0, 1]);
        assert_eq!(cells, vec!["Secret", "prod", "my-cm", "platform", "3h"]);
    }
}
```

- [ ] **Step 6: Update controller call site to pass `shared_config_columns`**

Modify `src/workers/kube/controller.rs`:

Find the `ConfigPoller::new(...)` call:

```rust
let config_handle = ConfigPoller::new(
    tx.clone(),
    shared_target_namespaces.clone(),
    shared_config_filter.clone(),
    client.clone(),
)
.spawn();
```

Replace with:

```rust
let config_handle = ConfigPoller::new(
    tx.clone(),
    shared_target_namespaces.clone(),
    shared_config_columns.clone(),
    shared_config_filter.clone(),
    client.clone(),
)
.spawn();
```

- [ ] **Step 7: Build, test, fmt**

```bash
cargo build 2>&1 | rg "error" | head -5
cargo test --all 2>&1 | rg "test result:" | tail -1
cargo +nightly fmt
```

Expected: 0 build errors, all tests pass.

- [ ] **Step 8: Commit**

```bash
git add src/features/config/kube/config.rs src/workers/kube/controller.rs
git commit -m "feat(config-poller): spec-driven rows with label value rendering"
```

---

## Task 5: app.rs registry + default + Render/Window/Tab wiring (dialog placeholder excluded)

**Files:**
- Modify: `src/app.rs`
- Modify: `src/workers/render.rs`
- Modify: `src/workers/render/window.rs`
- Modify: `src/features/config/view/tab.rs`
- Modify: `src/features/config/view/widgets/config.rs`

This task threads the registry from `app.rs` through `Render` → `WindowInit` → `ConfigTab::new` → `config_widget`. The column dialog itself is added in Task 6; here the widget gets `label_registry` but doesn't yet have action `'t'`.

- [ ] **Step 1: Add `build_config_label_registry` + `build_default_config_columns` in `src/app.rs`**

Find the existing `build_pod_label_registry` / `build_node_label_registry` (around lines 256/312). Add a Config equivalent right after `build_node_label_registry`:

```rust
/// Build the label-column registry for Config from config, erroring on
/// builtin name collisions or duplicate label headers (same canonical name
/// would render two identical-looking columns and break filter matching).
fn build_config_label_registry(
    label_columns: &Option<Vec<LabelColumnConfig>>,
) -> Result<Vec<ConfigLabelColumn>> {
    let mut out: Vec<ConfigLabelColumn> = Vec::new();
    if let Some(defs) = label_columns {
        for def in defs {
            let norm = ConfigColumn::normalize_column(&def.name);
            if ConfigColumn::from_str(&norm).is_ok() {
                anyhow::bail!(
                    "label_columns name '{}' collides with a builtin column name",
                    def.name
                );
            }
            if let Some(existing) = out
                .iter()
                .find(|lc| ConfigColumn::normalize_column(&lc.name) == norm)
            {
                anyhow::bail!(
                    "label_columns name '{}' has the same header as previously defined '{}'",
                    def.name,
                    existing.name
                );
            }
            out.push(ConfigLabelColumn {
                name: def.name.clone(),
                key: def.label.clone(),
                header: def.name.to_uppercase(),
            });
        }
    }
    Ok(out)
}

/// Build the default Config columns for startup: all builtin defaults followed
/// by every label column registered in `registry`. `ensure_required` and
/// `dedup_columns` guarantee KIND/NAME are present and there are no duplicates.
fn build_default_config_columns(registry: &[ConfigLabelColumn]) -> ConfigColumns {
    let mut specs: Vec<ConfigColumnSpec> = DEFAULT_CONFIG_COLUMNS
        .iter()
        .copied()
        .map(ConfigColumnSpec::Builtin)
        .collect();
    for lc in registry {
        specs.push(ConfigColumnSpec::Label {
            key: lc.key.clone(),
            header: lc.header.clone(),
        });
    }
    ConfigColumns::new(specs).ensure_required().dedup_columns()
}
```

Add to the top of `src/app.rs` (extend the existing `crate::features::config::...` import or add it):

```rust
use crate::features::config::{
    ConfigColumn,
    ConfigColumnSpec,
    ConfigColumns,
    ConfigLabelColumn,
};
use crate::features::config::columns::DEFAULT_CONFIG_COLUMNS;
```

Note: `DEFAULT_CONFIG_COLUMNS` is `pub const` in `columns.rs` but not re-exported through `features::config`. Re-export it: in `src/features/config.rs` change:

```rust
pub use columns::{ConfigColumn, ConfigColumnSpec, ConfigColumns, ConfigLabelColumn};
```

to:

```rust
pub use columns::{
    ConfigColumn,
    ConfigColumnSpec,
    ConfigColumns,
    ConfigLabelColumn,
    DEFAULT_CONFIG_COLUMNS,
};
```

(Then drop the `use crate::features::config::columns::DEFAULT_CONFIG_COLUMNS;` line in `app.rs` — just import everything from `crate::features::config::...`.)

Also ensure `LabelColumnConfig` is available; it is already imported in `app.rs` (used by Pod / Node).

`ConfigColumn::from_str` requires `std::str::FromStr` in scope. Add to the top of `app.rs` if not already present (Pod's path uses `FromStr` so it's likely already imported as `use std::{collections::HashMap, str::FromStr as _, thread, time};`).

- [ ] **Step 2: Build the registry and default columns at startup**

In `src/app.rs`, find the existing registry builds (around the `let pod_label_registry = ...` line):

```rust
let pod_label_registry = build_pod_label_registry(&config.theme.pod.label_columns)?;
let node_label_registry = build_node_label_registry(&config.theme.node.label_columns)?;
```

Add a Config line right after:

```rust
let config_label_registry =
    build_config_label_registry(&config.theme.config.label_columns)?;
let default_config_columns = build_default_config_columns(&config_label_registry);
```

- [ ] **Step 3: Pass `default_config_columns` to controller's `shared_config_columns`**

In `src/workers/kube/controller.rs`, the `shared_config_columns` is currently initialized with `ConfigColumns::default()`. It must instead come from `app.rs` (via `KubeWorkerConfig` or a constructor arg). To keep this PR's surface minimal, pass it through `KubeWorker::new`.

Find `KubeWorker::new` signature and `KubeWorker`'s field set (in `src/workers/kube/worker.rs` or `src/workers/kube.rs`):

```bash
rg -n "struct KubeWorker|impl KubeWorker|fn new" src/workers/kube/ --type rust | head -10
```

Locate `KubeWorker`'s configuration struct (likely `KubeWorkerConfig`) and add a field:

```rust
pub struct KubeWorkerConfig {
    // ... existing fields ...
    pub default_config_columns: ConfigColumns,  // NEW
}
```

Set the field at the call site in `app.rs`:

```rust
kube_worker_config.default_config_columns = default_config_columns.clone();
```

(Insert near the existing `kube_worker_config.<...>.default_columns = ...` assignments for Pod/Node.)

Then in `controller.rs`, replace:

```rust
let shared_config_columns: SharedConfigColumns =
    Arc::new(RwLock::new(ConfigColumns::default()));
```

with:

```rust
let shared_config_columns: SharedConfigColumns =
    Arc::new(RwLock::new(kube_worker_config.default_config_columns.clone()));
```

(`kube_worker_config` is destructured in the controller's `start`; ensure the field is reachable. If it's already destructured, add `default_config_columns` to the pattern.)

- [ ] **Step 4: Thread `default_config_columns` and `config_label_registry` into `Render`**

Modify `src/workers/render.rs`:

`Render` struct — add fields after `default_node_columns`:

```rust
pub struct Render {
    // ... existing ...
    default_pod_columns: Option<PodColumns>,
    default_node_columns: Option<NodeColumns>,
    default_config_columns: ConfigColumns,         // NEW
    pod_label_columns: Vec<PodLabelColumn>,
    node_label_columns: Vec<NodeLabelColumn>,
    config_label_columns: Vec<ConfigLabelColumn>,  // NEW
    // ... rest ...
}
```

Add corresponding parameters to `Render::new` and constructor body:

```rust
pub fn new(
    // ... existing positional args ...
    default_pod_columns: Option<PodColumns>,
    default_node_columns: Option<NodeColumns>,
    default_config_columns: ConfigColumns,            // NEW (insert in same group as other defaults)
    pod_label_registry: Vec<PodLabelColumn>,
    node_label_registry: Vec<NodeLabelColumn>,
    config_label_registry: Vec<ConfigLabelColumn>,    // NEW
    // ... rest ...
) -> Self {
    Self {
        // ... existing field assignments ...
        default_config_columns,
        config_label_columns: config_label_registry,
        // ...
    }
}
```

Add imports at the top of `render.rs`:

```rust
use crate::features::config::{ConfigColumns, ConfigLabelColumn};
```

In `app.rs`'s `Render::new(...)` call, pass:

```rust
default_pod_columns,
default_node_columns,
default_config_columns,           // NEW
pod_label_registry,
node_label_registry,
config_label_registry,            // NEW
```

- [ ] **Step 5: Forward through `WindowInit` → `ConfigTab::new`**

Modify `src/workers/render/window.rs`:

Find `WindowInit` (around line 88). Add the same two fields:

```rust
pub struct WindowInit {
    // ... existing fields ...
    pub default_config_columns: ConfigColumns,         // NEW
    pub config_label_columns: Vec<ConfigLabelColumn>,  // NEW
    // ... rest ...
}
```

In `Render::start` (or wherever `WindowInit { .. }` is constructed), populate:

```rust
WindowInit {
    // ... existing ...
    default_config_columns: self.default_config_columns.clone(),
    config_label_columns: self.config_label_columns.clone(),
    // ...
}
```

In `WindowInit::run` (where `ConfigTab::new(...)` is called) update the call to include the new args (Step 6 changes `ConfigTab::new` signature).

```rust
let ConfigTab {
    tab: config_tab,
    config_filter_help_dialog,
    // config_columns_dialog added in Task 6
} = ConfigTab::new(
    "Config",
    &self.tx,
    &clipboard,
    self.split_mode,
    self.default_config_columns.clone(),
    self.config_label_columns.clone(),
    self.theme.component.clone(),
);
```

(Adding clone calls; `ConfigTab::new` will be updated in next step.)

Imports at top of `window.rs`:

```rust
use crate::features::config::{ConfigColumns, ConfigLabelColumn};
```

- [ ] **Step 6: Update `ConfigTab::new` signature and pass into widget**

Modify `src/features/config/view/tab.rs`:

```rust
use crate::{
    clipboard::Clipboard,
    config::theme::WidgetThemeConfig,
    features::{
        component_id::CONFIG_TAB_ID,
        config::{ConfigColumns, ConfigLabelColumn},
    },
    message::Message,
    ui::{
        tab::{LayoutElement, NestedLayoutElement, NestedWidgetLayout, TabLayout},
        widget::Widget,
        Tab,
    },
};

use super::widgets::{config_filter_help_widget, config_widget, raw_data_widget};

pub struct ConfigTab {
    pub tab: Tab<'static>,
    pub config_filter_help_dialog: Widget<'static>,
    // config_columns_dialog: added in Task 6
}

impl ConfigTab {
    pub fn new(
        title: &'static str,
        tx: &Sender<Message>,
        clipboard: &Option<Rc<RefCell<Clipboard>>>,
        split_direction: Direction,
        default_columns: ConfigColumns,                  // NEW (suppressed until Task 6)
        label_registry: Vec<ConfigLabelColumn>,          // NEW
        theme: WidgetThemeConfig,
    ) -> Self {
        let error_theme = theme.error.clone().into();

        let _ = &default_columns;  // Task 6 consumes this in the dialog widget

        let config_widget = config_widget(tx, label_registry.clone(), theme.clone());
        let raw_data_widget = raw_data_widget(clipboard, theme.clone());
        let config_filter_help_dialog = config_filter_help_widget(theme);

        let layout = TabLayout::new(layout, split_direction);

        Self {
            tab: Tab::new(
                CONFIG_TAB_ID,
                title,
                [config_widget, raw_data_widget],
                layout,
            )
            .error_theme(error_theme),
            config_filter_help_dialog,
        }
    }
}
```

(The `_ = &default_columns;` line silences the unused-variable warning while Task 6 wires the dialog.)

- [ ] **Step 7: Update `config_widget` to take `label_registry`**

Modify `src/features/config/view/widgets/config.rs`:

```rust
use crate::{
    config::theme::WidgetThemeConfig,
    features::{
        component_id::{CONFIG_RAW_DATA_WIDGET_ID, CONFIG_WIDGET_ID},
        config::{
            config_filter_applicator,
            message::{ConfigRequest, RequestData},
            ConfigLabelColumn,
        },
    },
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

pub fn config_widget(
    tx: &Sender<Message>,
    label_registry: Vec<ConfigLabelColumn>,
    theme: WidgetThemeConfig,
) -> Widget<'static> {
    let tx = tx.clone();

    let widget_theme = WidgetTheme::from(theme.clone());
    let filter_theme = FilterFormTheme::from(theme.clone());
    let table_theme = TableTheme::from(theme.clone());

    let widget_base = WidgetBase::builder()
        .title("Config")
        .theme(widget_theme)
        .build();

    let filter_form = FilterForm::builder().theme(filter_theme).build();

    Table::builder()
        .id(CONFIG_WIDGET_ID)
        .widget_base(widget_base)
        .filter_form(filter_form)
        .theme(table_theme)
        .filter_applicator(config_filter_applicator(label_registry, tx.clone()))
        .block_injection(block_injection())
        .on_select(on_select(tx))
        .build()
        .into()
}
```

`config_filter_applicator` will be updated in Task 7 to take `Vec<ConfigLabelColumn>`. For now in this task, the call site has the wrong arity — fix this by also updating the filter applicator signature here (anticipating Task 7) OR keep the registry threading but call the old signature `config_filter_applicator(tx.clone())`.

For incremental commits, use the latter — keep `config_filter_applicator(tx.clone())` and add `let _ = &label_registry;` to silence the warning. Task 7 will update the call.

So Step 7 becomes:

```rust
pub fn config_widget(
    tx: &Sender<Message>,
    label_registry: Vec<ConfigLabelColumn>,
    theme: WidgetThemeConfig,
) -> Widget<'static> {
    let _ = &label_registry;  // consumed in Task 7
    let tx = tx.clone();
    // ... rest unchanged, including filter_applicator(config_filter_applicator(tx.clone()))
}
```

- [ ] **Step 8: Build, test, fmt**

```bash
cargo build 2>&1 | rg "error" | head -5
cargo test --all 2>&1 | rg "test result:" | tail -1
cargo +nightly fmt
```

Expected: 0 errors. Existing tests pass. New behavior: config tab uses `default_config_columns` constructed from registry, label values appear in table.

- [ ] **Step 9: Commit**

```bash
git add src/app.rs src/features/config.rs src/workers/render.rs src/workers/render/window.rs src/features/config/view/tab.rs src/features/config/view/widgets/config.rs src/workers/kube/controller.rs src/workers/kube/config.rs
git commit -m "feat(config): registry + default columns wired from app.rs to widget"
```

(Adjust `git add` to the actual files you touched — `src/workers/kube/config.rs` is the worker config file, not the poller.)

---

## Task 6: Column dialog widget + action 't' + Window/Tab integration

**Files:**
- Create: `src/features/config/view/widgets/config_columns_dialog.rs`
- Modify: `src/features/config/view/widgets.rs`
- Modify: `src/features/component_id.rs`
- Modify: `src/features/config/view/tab.rs`
- Modify: `src/features/config/view/widgets/config.rs`
- Modify: `src/workers/render/window.rs`

- [ ] **Step 1: Add component ID**

Modify `src/features/component_id.rs`:

```rust
config_filter_help_dialog,
config_columns_dialog,           // NEW
network_filter_help_dialog,
```

- [ ] **Step 2: Create `src/features/config/view/widgets/config_columns_dialog.rs`**

```rust
use std::{collections::BTreeMap, str::FromStr as _};

use crossbeam::channel::Sender;
use strum::IntoEnumIterator;

use crate::{
    config::theme::WidgetThemeConfig,
    features::{
        component_id::CONFIG_COLUMNS_DIALOG_ID,
        config::{
            message::ConfigMessage,
            ConfigColumn,
            ConfigColumnSpec,
            ConfigColumns,
            ConfigLabelColumn,
        },
    },
    message::Message,
    ui::{
        event::EventResult,
        widget::{CheckList, CheckListItem, CheckListTheme, Widget, WidgetBase, WidgetTheme},
        Window,
    },
};

pub fn config_columns_dialog(
    tx: &Sender<Message>,
    default_columns: ConfigColumns,
    label_registry: Vec<ConfigLabelColumn>,
    theme: WidgetThemeConfig,
) -> Widget<'static> {
    let check_list_theme = CheckListTheme::from(theme.clone());
    let widget_theme = WidgetTheme::from(theme.clone());
    let widget_base = WidgetBase::builder()
        .title("Config Columns")
        .theme(widget_theme)
        .build();

    let items = build_check_list_items(default_columns, &label_registry);

    CheckList::builder()
        .id(CONFIG_COLUMNS_DIALOG_ID)
        .widget_base(widget_base)
        .theme(check_list_theme)
        .items(items)
        .on_change(on_change(tx.clone()))
        .build()
        .into()
}

/// All candidate columns: every builtin, then every defined label column.
fn candidate_specs(label_registry: &[ConfigLabelColumn]) -> Vec<ConfigColumnSpec> {
    ConfigColumn::iter()
        .map(ConfigColumnSpec::Builtin)
        .chain(label_registry.iter().map(|lc| {
            ConfigColumnSpec::Label {
                key: lc.key.clone(),
                header: lc.header.clone(),
            }
        }))
        .collect()
}

fn build_check_list_items(
    default_columns: ConfigColumns,
    label_registry: &[ConfigLabelColumn],
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

fn make_item(spec: &ConfigColumnSpec, checked: bool) -> CheckListItem {
    CheckListItem {
        label: spec.header(),
        checked,
        required: matches!(
            spec,
            ConfigColumnSpec::Builtin(ConfigColumn::Kind)
                | ConfigColumnSpec::Builtin(ConfigColumn::Name)
        ),
        metadata: Some(metadata_for(spec)),
    }
}

fn metadata_for(spec: &ConfigColumnSpec) -> BTreeMap<String, String> {
    match spec {
        ConfigColumnSpec::Builtin(c) => {
            BTreeMap::from([
                ("kind".to_string(), "builtin".to_string()),
                ("id".to_string(), c.as_str().to_string()),
            ])
        }
        ConfigColumnSpec::Label { key, header } => {
            BTreeMap::from([
                ("kind".to_string(), "label".to_string()),
                ("key".to_string(), key.clone()),
                ("header".to_string(), header.clone()),
            ])
        }
    }
}

fn spec_from_item(item: &CheckListItem) -> Option<ConfigColumnSpec> {
    let md = item.metadata.as_ref()?;
    match md.get("kind").map(String::as_str) {
        Some("builtin") => ConfigColumn::from_str(md.get("id")?)
            .ok()
            .map(ConfigColumnSpec::Builtin),
        Some("label") => Some(ConfigColumnSpec::Label {
            key: md.get("key")?.clone(),
            header: md.get("header")?.clone(),
        }),
        _ => None,
    }
}

fn collect_columns(items: &[CheckListItem]) -> ConfigColumns {
    let specs: Vec<ConfigColumnSpec> = items
        .iter()
        .filter(|item| item.required || item.checked)
        .filter_map(spec_from_item)
        .collect();

    ConfigColumns::new(specs).ensure_required()
}

fn on_change(tx: Sender<Message>) -> impl Fn(&mut Window, &CheckListItem) -> EventResult {
    move |w: &mut Window, _v| {
        let widget = w
            .find_widget_mut(CONFIG_COLUMNS_DIALOG_ID)
            .as_mut_check_list();
        let columns = collect_columns(widget.items());
        tx.send(ConfigMessage::ColumnsRequest(columns).into())
            .expect("Failed to send ConfigMessage::ColumnsRequest");
        EventResult::Nop
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn label_spec(key: &str, header: &str) -> ConfigColumnSpec {
        ConfigColumnSpec::Label {
            key: key.into(),
            header: header.into(),
        }
    }

    #[test]
    fn 選択列を先頭にその他候補を未チェックで並べる() {
        let registry = vec![ConfigLabelColumn {
            name: "app".into(),
            key: "app.kubernetes.io/name".into(),
            header: "APP".into(),
        }];
        let current = ConfigColumns::new([
            ConfigColumnSpec::Builtin(ConfigColumn::Kind),
            ConfigColumnSpec::Builtin(ConfigColumn::Name),
            label_spec("app.kubernetes.io/name", "APP"),
        ]);

        let items = build_check_list_items(current, &registry);

        assert_eq!(items[0].label, "KIND");
        assert!(items[0].checked);
        assert_eq!(items[1].label, "NAME");
        assert!(items[1].checked);
        assert_eq!(items[2].label, "APP");
        assert!(items[2].checked);
        // 残りは unchecked
        assert!(items[3..].iter().all(|i| !i.checked));
    }

    #[test]
    fn collect_columns_は表示順を維持しdedupする() {
        let items = vec![
            make_item(&label_spec("app.kubernetes.io/name", "APP"), true),
            make_item(&ConfigColumnSpec::Builtin(ConfigColumn::Kind), true),
            make_item(&ConfigColumnSpec::Builtin(ConfigColumn::Name), true),
            make_item(&ConfigColumnSpec::Builtin(ConfigColumn::Data), false),
        ];

        let columns = collect_columns(&items);

        // ensure_required は KIND/NAME を強制配置するが、すでに含まれているので
        // 並び替えはしない。APP が先頭、KIND, NAME が続く。
        assert_eq!(
            columns.specs(),
            &[
                label_spec("app.kubernetes.io/name", "APP"),
                ConfigColumnSpec::Builtin(ConfigColumn::Kind),
                ConfigColumnSpec::Builtin(ConfigColumn::Name),
            ]
        );
    }

    #[test]
    fn メタデータからspecを復元できる() {
        let builtin = make_item(&ConfigColumnSpec::Builtin(ConfigColumn::Data), true);
        let label = make_item(&label_spec("k", "MIG"), true);

        assert_eq!(
            spec_from_item(&builtin),
            Some(ConfigColumnSpec::Builtin(ConfigColumn::Data))
        );
        assert_eq!(spec_from_item(&label), Some(label_spec("k", "MIG")));
    }

    #[test]
    fn KINDとNAMEはrequired() {
        let kind = make_item(&ConfigColumnSpec::Builtin(ConfigColumn::Kind), true);
        let name = make_item(&ConfigColumnSpec::Builtin(ConfigColumn::Name), true);
        let data = make_item(&ConfigColumnSpec::Builtin(ConfigColumn::Data), true);
        let label = make_item(&label_spec("k", "APP"), true);

        assert!(kind.required);
        assert!(name.required);
        assert!(!data.required);
        assert!(!label.required);
    }
}
```

- [ ] **Step 3: Register dialog module**

Modify `src/features/config/view/widgets.rs`:

```rust
mod config;
mod config_columns_dialog;
mod config_filter_help;
mod raw_data;

pub(super) use config::*;
pub(super) use config_columns_dialog::*;
pub(super) use config_filter_help::*;
pub(super) use raw_data::*;
```

- [ ] **Step 4: Add action 't' on `config_widget`**

Modify `src/features/config/view/widgets/config.rs` — add to the `Table::builder()` chain:

```rust
Table::builder()
    .id(CONFIG_WIDGET_ID)
    .widget_base(widget_base)
    .filter_form(filter_form)
    .theme(table_theme)
    .filter_applicator(config_filter_applicator(tx.clone()))
    .action('t', open_config_columns_dialog())  // NEW
    .block_injection(block_injection())
    .on_select(on_select(tx))
    .build()
    .into()
```

Also remove the `let _ = &label_registry;` suppressor from Task 5 (registry is still not consumed in this file — Task 7 does that). Replace with a placeholder for action only:

```rust
let _ = &label_registry;  // consumed in Task 7 by the filter applicator
```

Add the helper function at file scope (above `block_injection`):

```rust
fn open_config_columns_dialog() -> impl Fn(&mut Window) -> EventResult {
    use crate::features::component_id::CONFIG_COLUMNS_DIALOG_ID;
    |w: &mut Window| {
        w.open_dialog(CONFIG_COLUMNS_DIALOG_ID);
        EventResult::Nop
    }
}
```

- [ ] **Step 5: Build dialog in `ConfigTab::new` and expose field**

Modify `src/features/config/view/tab.rs`:

```rust
use super::widgets::{
    config_columns_dialog,
    config_filter_help_widget,
    config_widget,
    raw_data_widget,
};

pub struct ConfigTab {
    pub tab: Tab<'static>,
    pub config_columns_dialog: Widget<'static>,        // NEW
    pub config_filter_help_dialog: Widget<'static>,
}

impl ConfigTab {
    pub fn new(
        title: &'static str,
        tx: &Sender<Message>,
        clipboard: &Option<Rc<RefCell<Clipboard>>>,
        split_direction: Direction,
        default_columns: ConfigColumns,
        label_registry: Vec<ConfigLabelColumn>,
        theme: WidgetThemeConfig,
    ) -> Self {
        let error_theme = theme.error.clone().into();

        let config_widget = config_widget(tx, label_registry.clone(), theme.clone());
        let raw_data_widget = raw_data_widget(clipboard, theme.clone());
        let config_columns_dialog =
            config_columns_dialog(tx, default_columns, label_registry, theme.clone());
        let config_filter_help_dialog = config_filter_help_widget(theme);

        let layout = TabLayout::new(layout, split_direction);

        Self {
            tab: Tab::new(
                CONFIG_TAB_ID,
                title,
                [config_widget, raw_data_widget],
                layout,
            )
            .error_theme(error_theme),
            config_columns_dialog,
            config_filter_help_dialog,
        }
    }
}
```

(Drop the `let _ = &default_columns;` line from Task 5 — it's now consumed.)

- [ ] **Step 6: Register dialog in `window.rs`**

Modify `src/workers/render/window.rs`:

Update the `ConfigTab` destructure:

```rust
let ConfigTab {
    tab: config_tab,
    config_columns_dialog,       // NEW
    config_filter_help_dialog,
} = ConfigTab::new(
    // ... unchanged ...
);
```

Add to `dialog_widgets` vector (alphabetical placement near `config_filter_help_dialog`):

```rust
let dialog_widgets = vec![
    // ... existing ...
    config_filter_help_dialog,
    config_columns_dialog,        // NEW
    // ...
];
```

- [ ] **Step 7: Build, test, fmt**

```bash
cargo build 2>&1 | rg "error" | head -5
cargo test --all 2>&1 | rg "test result:" | tail -1
cargo +nightly fmt
```

Expected: 0 errors. New dialog tests pass.

- [ ] **Step 8: Commit**

```bash
git add src/features/component_id.rs src/features/config/view/widgets/config_columns_dialog.rs src/features/config/view/widgets.rs src/features/config/view/widgets/config.rs src/features/config/view/tab.rs src/workers/render/window.rs
git commit -m "feat(config-dialog): label-aware column dialog + 't' key binding"
```

---

## Task 7: Filter parser registry support

**Files:**
- Modify: `src/features/config/filter/parser.rs`
- Modify: `src/features/config/filter.rs`
- Modify: `src/features/config/view/widgets/config.rs`

- [ ] **Step 1: Update `parse_config_filter` to accept registry**

Modify `src/features/config/filter/parser.rs`:

```rust
use std::collections::HashSet;

use strum::IntoEnumIterator;

use crate::{
    features::config::{ConfigColumn, ConfigLabelColumn},
    ui::widget::{normalize_column_name, parse_table_filter, TableFilterPredicate},
};

fn valid_columns(label_registry: &[ConfigLabelColumn]) -> HashSet<String> {
    let mut set: HashSet<String> = ConfigColumn::iter()
        .map(|c| normalize_column_name(c.display()))
        .collect();
    for lc in label_registry {
        set.insert(normalize_column_name(&lc.header));
    }
    set
}

/// Parse a Config-filter input string into a `TableFilterPredicate`.
///
/// `namespace:` is rejected with a guidance message that points users to the
/// namespace selector. This check fires *before* the builtin / registry lookup
/// so the guidance is preserved even when a label column with header
/// "NAMESPACE" is registered. Other columns are validated against the builtin
/// `ConfigColumn` set plus any defined label columns in `label_registry`.
pub fn parse_config_filter(
    input: &str,
    label_registry: &[ConfigLabelColumn],
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

    fn no_label_cols() -> Vec<ConfigLabelColumn> {
        Vec::new()
    }

    fn registry_with(name: &str, header: &str) -> Vec<ConfigLabelColumn> {
        vec![ConfigLabelColumn {
            name: name.to_string(),
            key: "irrelevant.example.com/key".to_string(),
            header: header.to_string(),
        }]
    }

    #[test]
    fn empty_input_yields_empty_predicate() {
        let p = parse_config_filter("", &no_label_cols()).unwrap();
        assert!(p.column_includes.is_empty());
        assert!(p.column_excludes.is_empty());
        assert_eq!(p.label_selector, None);
    }

    #[test]
    fn bare_value_becomes_name_include() {
        let p = parse_config_filter("my-cm", &no_label_cols()).unwrap();
        let patterns = p.column_includes.get("name").expect("name column");
        assert!(patterns[0].is_match("my-cm-abc"));
    }

    #[test]
    fn builtin_columns_are_accepted() {
        let p = parse_config_filter("kind:ConfigMap !kind:Secret", &no_label_cols()).unwrap();
        assert!(p.column_includes.contains_key("kind"));
        assert!(p.column_excludes.contains_key("kind"));
    }

    #[test]
    fn data_and_age_columns_are_accepted() {
        let p = parse_config_filter("data:0 age:1d", &no_label_cols()).unwrap();
        assert!(p.column_includes.contains_key("data"));
        assert!(p.column_includes.contains_key("age"));
    }

    #[test]
    fn label_selector_is_captured() {
        let p = parse_config_filter("label:app=nginx", &no_label_cols()).unwrap();
        assert_eq!(p.label_selector.as_deref(), Some("app=nginx"));
    }

    #[test]
    fn unknown_column_produces_parse_error() {
        let err = parse_config_filter("staus:Active", &no_label_cols()).unwrap_err();
        assert!(err.contains("unknown column") && err.contains("staus"));
    }

    #[test]
    fn namespace_returns_guidance_message() {
        let err = parse_config_filter("namespace:default", &no_label_cols()).unwrap_err();
        assert_eq!(
            err,
            "namespace is selected via the namespace selector, not the filter"
        );
    }

    #[test]
    fn registered_label_column_header_is_accepted() {
        let regs = registry_with("app", "APP");
        let p = parse_config_filter("app:nginx", &regs).unwrap();
        assert!(p.column_includes.contains_key("app"));
    }

    #[test]
    fn namespace_guidance_precedes_registry_even_on_collision() {
        let regs = registry_with("namespace", "NAMESPACE");
        let err = parse_config_filter("namespace:default", &regs).unwrap_err();
        assert_eq!(
            err,
            "namespace is selected via the namespace selector, not the filter"
        );
    }

    #[test]
    fn quoted_value_with_whitespace() {
        let p = parse_config_filter(r#"name:"my config""#, &no_label_cols()).unwrap();
        let patterns = p.column_includes.get("name").unwrap();
        assert!(patterns[0].is_match("my config"));
    }
}
```

(The Pod-style `BUILTIN_COLUMNS` const goes away because we now build `valid_columns` from `ConfigColumn::iter()` directly — simpler and stays in sync with the enum.)

- [ ] **Step 2: Update `config_filter_applicator` to take registry**

Modify `src/features/config/filter.rs`:

```rust
mod parser;

use crossbeam::channel::Sender;

use crate::{
    features::{
        component_id::CONFIG_FILTER_HELP_DIALOG_ID,
        config::{message::ConfigMessage, ConfigLabelColumn},
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

pub use parser::parse_config_filter;

pub fn config_filter_applicator(
    label_registry: Vec<ConfigLabelColumn>,
    tx: Sender<Message>,
) -> TableFilterApplicator {
    let parser: TableFilterParser =
        (move |input: &str| parse_config_filter(input, &label_registry)).into();

    let tx_apply = tx.clone();
    let tx_cancel = tx;

    let on_apply: OnFilterApply = (move |predicate: &crate::ui::widget::TableFilterPredicate,
                                         _window: &mut Window| {
        tx_apply
            .send(ConfigMessage::Filter(predicate.label_selector.clone()).into())
            .expect("Failed to send ConfigMessage::Filter");
    })
    .into();

    let on_cancel: OnFilterCancel = (move |_window: &mut Window| {
        tx_cancel
            .send(ConfigMessage::Filter(None).into())
            .expect("Failed to send ConfigMessage::Filter(None) on cancel");
    })
    .into();

    TableFilterApplicator::new(parser, ApplyStrategy::EnterToConfirm)
        .with_help_dialog(CONFIG_FILTER_HELP_DIALOG_ID)
        .with_on_apply(on_apply)
        .with_on_cancel(on_cancel)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applicator_constructs_without_panic() {
        let (tx, _rx) = crossbeam::channel::bounded(1);
        let _ = config_filter_applicator(Vec::new(), tx);
    }
}
```

- [ ] **Step 3: Update widget call site**

Modify `src/features/config/view/widgets/config.rs`:

Drop the `let _ = &label_registry;` line added in Task 5/6. Update the applicator call:

```rust
.filter_applicator(config_filter_applicator(label_registry, tx.clone()))
```

- [ ] **Step 4: Build, test, fmt**

```bash
cargo build 2>&1 | rg "error" | head -5
cargo test --all 2>&1 | rg "test result:" | tail -1
cargo +nightly fmt
```

Expected: 0 errors. Filter parser tests (10) pass.

- [ ] **Step 5: Commit**

```bash
git add src/features/config/filter/parser.rs src/features/config/filter.rs src/features/config/view/widgets/config.rs
git commit -m "feat(config-filter): accept label_registry; registered headers are valid columns"
```

---

## Task 8: Final verification

- [ ] **Step 1: Run all gates**

```bash
cargo build 2>&1 | tail -10
cargo test --all 2>&1 | tail -3
cargo clippy --all-targets 2>&1 | rg "src/features/config|src/config/theme/config|src/app.rs" | head -10
cargo +nightly fmt --check 2>&1 | head -10
```

Expected:
- build: clean (only pre-existing `try_from_kubeconfig` warning)
- test: all green, test count = baseline + Task 1 tests (10ish) + Task 2 tests (2) + Task 4 tests (5) + Task 6 tests (4) + Task 7 tests (10) − any old parser tests that became obsolete (none — they were updated, not deleted)
- clippy: no new warnings
- fmt: clean

Apply fmt if needed:

```bash
cargo +nightly fmt
```

- [ ] **Step 2: Update PR description test plan placeholder**

(Skip — this is done after PR creation.)

- [ ] **Step 3: Create PR**

```bash
git push -u origin feat/config-label-columns
gh pr create --title "feat(config): label_columns + column dialog (Pod #993 mirror)" --body "$(cat <<'EOF'
## Summary

Add Pod-#993-style label_columns + column dialog to the Config tab. Users can declare `theme.config.label_columns` and the values appear as table columns (toggleable via the new `t`-key dialog). KIND and NAME are required (cannot be unchecked). No CLI args, no presets — those are deferred per the spec.

## Test plan

- [x] `cargo build`: clean
- [x] `cargo test --all`: ≥701 + new (~31) = ≥732 passed
- [x] `cargo clippy --all-targets`: no new warnings
- [x] `cargo +nightly fmt --check`: clean
- [ ] Manual GKE smoke:
  - [ ] With `theme.config.label_columns: [{name: app, label: app.kubernetes.io/name}]`, APP column appears at startup
  - [ ] `t` opens Config Columns dialog; APP is toggleable
  - [ ] Filter `app:datadog` works (registered header is accepted)
  - [ ] KIND and NAME cannot be unchecked
  - [ ] `namespace:default` still returns guidance (Z model preserved)
  - [ ] Filter `label:app=datadog` still applies server-side

## Related

- Spec: `docs/superpowers/specs/2026-06-03-config-network-label-columns-design.md`
- Mirrors PR #993 (Pod label_columns)
- Next: same pattern for Network tab in a follow-up PR
EOF
)"
```

- [ ] **Step 4: Manual GKE verification**

Configure your kubetui config with a label_columns entry and start kubetui:

```yaml
theme:
  config:
    label_columns:
      - { name: app, label: app.kubernetes.io/name }
```

Verify in order:

1. Config tab opens; APP column visible (4 builtins + 1 label = 5 columns)
2. Press `t` → "Config Columns" dialog opens. APP, KIND, NAME, DATA, AGE listed. KIND and NAME marked required.
3. Toggle APP off → table re-renders without APP within 1 second
4. Toggle APP back on → table re-renders with APP
5. Try to uncheck KIND → not allowed (`required: true`)
6. Filter input: `app:datadog` → narrows to rows where APP matches
7. Filter input: `unknown_column:x` → "unknown column 'unknown_column'" error
8. Filter input: `namespace:default` → namespace guidance
9. Filter input: `label:app=datadog` → server-side labelSelector filters at API

- [ ] **Step 5: Update PR test plan checkboxes**

After manual verification passes, edit the PR body with `gh pr edit <pr> --body ...` and check the boxes for each manual smoke item.

---

## Notes

- Each task is committable on its own; build and tests pass after every task.
- Tasks 5 and 6 together replace the suppressed `let _ = &default_columns;` and `let _ = &label_registry;` placeholders with real consumers.
- Task 5's poller refactor (Step 4 in Task 5) and Task 6's dialog consume the same `ConfigColumns` from different ends; the SharedConfigColumns plumbing in Task 3 is what connects them.
- Filter parser changes (Task 7) come last so the parser doesn't need to know about the registry until the widget can supply it.
- Network is **out of scope** for this plan; a separate plan covers it (same shape, fewer differences once Config is merged).
