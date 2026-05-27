# Node タブ — Plan 3: ラベル列（(A) インターリーブ・ラベルもトグル/CLI 指定可）実装プラン

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Node 一覧で任意のラベルを列として表示できるようにする。ラベル列は `label_columns` レジストリで定義し、`column_presets` からビルトイン列と**任意順（インターリーブ）**で参照する。**ラベル列も `t` ダイアログでトグル可能**、**`--node-columns` でラベル名も指定可能**にする（採用案 = (A) 拡張版）。

**Architecture:** ランタイムの「列」を `NodeColumnSpec`（`Builtin(NodeColumn)` | `Label { key, header }`）の列とし、`NodeColumns = Vec<NodeColumnSpec>` に拡張。設定の `label_columns`（`{name,label}`）を解決した**ラベルレジストリ**（`Vec<NodeLabelColumn>`）を app.rs で構築し、CLI（名前の配列）とプリセット（名前の配列）を**同一の解決関数**で `NodeColumns` に変換する。レジストリはダイアログにも配線し、ビルトイン＋ラベルを全てトグル可能にする。poller は Plan 1 同様 `/api/v1/nodes` を取得し（Table API はデフォルトで `row.object`=PartialObjectMetadata＝`metadata.labels` を含む。実機確認済み・`includeObject` 指定不要）、`Builtin` 列はサーバ印字セル、`Label` 列は `row.object.metadata.labels[key]` から値を得る。

**Tech Stack:** Rust 2021, kube-rs/k8s-openapi（Table API の `row.object`=PartialObjectMetadata / `RawExtension`）, serde/figment, ratatui (CheckList), strum, mockall, rstest, pretty_assertions, serde_json。

**前提:** Plan 1（一覧）・Plan 2（列ダイアログ）完了。ブランチ `920-node-label-columns`（Plan 2 にスタック）。

**設計スペック:** `docs/superpowers/specs/2026-05-22-node-tab-design.md`（「列設定」節を本プランの内容へ更新。Task 6）。

---

## 確定した設計判断

1. **内部表現**: ビルトイン `NodeColumn`（Copy enum）は維持。新規 `NodeColumnSpec`:
   ```rust
   #[derive(Debug, Clone, PartialEq, Eq, Hash)]
   pub enum NodeColumnSpec {
       Builtin(NodeColumn),
       Label { key: String, header: String },
   }
   ```
   `NodeColumns = Vec<NodeColumnSpec>`。

2. **ラベルレジストリ（解決済み）**: 
   ```rust
   #[derive(Debug, Clone, PartialEq, Eq)]
   pub struct NodeLabelColumn { pub name: String, pub key: String, pub header: String }
   ```
   設定 `label_columns:[{name,label}]` から app.rs で構築（`header = name.to_uppercase()`、`name` は正規化して照合）。ダイアログにも配線する。

3. **設定**:
   - `label_columns: Vec<LabelColumnConfig>` — `{ name, label }`。
   - `column_presets: HashMap<String, Vec<String>>` — ビルトイン名/ラベル名を任意順で。

4. **解決＋バリデーション（app.rs、読込時）**: 名前列（CLI でもプリセットでも）を共通関数 `resolve_columns(names, &registry)` で `NodeColumns` に解決:
   - ビルトイン（`NodeColumn::from_str`）か、registry の name（正規化照合）か。
   - `"full"` 単独 → 全ビルトイン。
   - 衝突エラー（ラベル name = ビルトイン名）、未定義参照エラー（どちらでもない名前）。

5. **CLI**: `--node-columns` の値は**名前の配列**（`Option<Vec<String>>`）。`parse_node_columns` はカンマ分割のみ（解決しない）。app.rs で registry を使って解決（**ラベル名指定可**）。優先順位: CLI > `--node-columns-preset` > `default_preset`。

6. **ダイアログ**: チェックリスト = 全ビルトイン列 ＋ registry の全ラベル列。**両方トグル可能**。初期チェックは現 `NodeColumns`。`on_change` は `NodeColumns` を再構築（現順序を保持し、外したものを除去、新規チェックを末尾追加）。各項目の識別はチェックリスト構築時の `Vec<NodeColumnSpec>`（項目と同順）を closure に保持して index 対応させる。

7. **poller**: URL は Plan 1 のまま `/api/v1/nodes`（**`includeObject` 指定は不要**。Table API はデフォルトで `row.object` に `metadata.labels` を含む。kind v1.34.0 で実機確認済み）。`Builtin` 列は `columnDefinitions` から名前で、`Label` 列は `row.object`（`RawExtension`）→ `metadata.labels[key]`。無ければ空セル。

---

## ファイル構成

変更:
- `src/features/node/node_columns.rs` — `NodeColumnSpec`・`NodeLabelColumn` 追加、`NodeColumns = Vec<NodeColumnSpec>`、`header()`/`from_builtins`/`specs()` 等。
- `src/config/theme/node.rs` — `label_columns: Vec<LabelColumnConfig>`、`column_presets: HashMap<String, Vec<String>>`（`NodeColumnConfig` 廃止）。
- `src/cmd/args/node_columns.rs` — `parse_node_columns` を `Vec<String>` 生成に。
- `src/cmd/command.rs` — `pub node_columns: Option<Vec<String>>` に変更。
- `src/app.rs` — `resolve_columns` ＋ `build_node_columns`（CLI 名前列・プリセットを registry で解決＋検証）＋ registry 構築。`Render::new` に registry を渡す。
- `src/workers/render.rs` / `src/workers/render/window.rs` — `node_label_columns: Vec<NodeLabelColumn>` を配線。
- `src/features/node/view/tab.rs` — `NodeTab::new` が registry を受け取りダイアログへ。
- `src/features/node/view/widgets/node_columns_dialog.rs` — ビルトイン＋ラベルのトグル対応。
- `src/features/node/kube/node.rs` — `get_node_table` を `NodeColumnSpec` 対応＋ラベル抽出（`row.object.metadata.labels`）。URL は据え置き（`/api/v1/nodes`）。
- `example/config.yaml` / 設計スペック — (A) 形式へ。

---

## Task 1: NodeColumnSpec / NodeLabelColumn 導入と NodeColumns 刷新（挙動不変リファクタ）

**Files:** `src/features/node/node_columns.rs`（＋呼び出し側を型追随）

`NodeColumns` を `Vec<NodeColumnSpec>` に変更。ビルトインのみで従来挙動を維持。

- [ ] **Step 1: 失敗するテストを書く**

```rust
#[cfg(test)]
mod spec_tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn builtin_spec_header_is_uppercase_display() {
        assert_eq!(NodeColumnSpec::Builtin(NodeColumn::Status).header(), "STATUS");
    }

    #[test]
    fn label_spec_header_is_as_given() {
        let s = NodeColumnSpec::Label { key: "x".into(), header: "MIG".into() };
        assert_eq!(s.header(), "MIG");
    }

    #[test]
    fn default_columns_are_builtin_specs() {
        assert_eq!(
            NodeColumns::default().specs(),
            &[
                NodeColumnSpec::Builtin(NodeColumn::Name),
                NodeColumnSpec::Builtin(NodeColumn::Status),
                NodeColumnSpec::Builtin(NodeColumn::Roles),
                NodeColumnSpec::Builtin(NodeColumn::Age),
                NodeColumnSpec::Builtin(NodeColumn::Version),
            ]
        );
    }
}
```

- [ ] **Step 2: 失敗確認** — `cargo test features::node::node_columns`（コンパイルエラー）。

- [ ] **Step 3: 実装**（`NodeColumn` Copy enum と `DEFAULT_NODE_COLUMNS` は維持）:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NodeColumnSpec {
    Builtin(NodeColumn),
    Label { key: String, header: String },
}

impl NodeColumnSpec {
    pub fn header(&self) -> String {
        match self {
            NodeColumnSpec::Builtin(c) => c.display().to_string(),
            NodeColumnSpec::Label { header, .. } => header.clone(),
        }
    }
}

/// 解決済みラベル列定義（registry の要素）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeLabelColumn {
    pub name: String,
    pub key: String,
    pub header: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeColumns {
    columns: Vec<NodeColumnSpec>,
}

impl Default for NodeColumns {
    fn default() -> Self {
        NodeColumns {
            columns: DEFAULT_NODE_COLUMNS.iter().map(|c| NodeColumnSpec::Builtin(*c)).collect(),
        }
    }
}

impl NodeColumns {
    pub fn new(columns: impl IntoIterator<Item = NodeColumnSpec>) -> Self {
        NodeColumns { columns: columns.into_iter().collect() }
    }

    pub fn from_builtins(columns: impl IntoIterator<Item = NodeColumn>) -> Self {
        NodeColumns { columns: columns.into_iter().map(NodeColumnSpec::Builtin).collect() }
    }

    pub fn specs(&self) -> &[NodeColumnSpec] {
        &self.columns
    }

    pub fn ensure_name_column(mut self) -> Self {
        let has_name = self.columns.iter()
            .any(|s| matches!(s, NodeColumnSpec::Builtin(NodeColumn::Name)));
        if !has_name {
            self.columns.insert(0, NodeColumnSpec::Builtin(NodeColumn::Name));
        }
        self
    }

    pub fn dedup_columns(self) -> Self {
        let mut unique: Vec<NodeColumnSpec> = Vec::new();
        for c in self.columns {
            if !unique.contains(&c) {
                unique.push(c);
            }
        }
        NodeColumns { columns: unique }
    }
}
```

- [ ] **Step 4: 呼び出し側を型追随（ビルトインのみ、挙動不変）**
  - `kube/node.rs`（`get_node_table`）: ヘッダは `spec.header()`。当面 `Builtin` のみ処理（`Label` は Task 3 で実装。暫定で `Label` セルは空文字）。
  - `cmd/args/node_columns.rs`: 当面 `NodeColumns::from_builtins(...)` を返す（CLI 変更は Task 2）。
  - `node_columns_dialog.rs`: `specs()` ベースに最小追随（ビルトインのみ。本実装は Task 4）。
  - `config/theme/node.rs` / `app.rs`: 当面そのまま（Task 2 で刷新）。`NodeColumnConfig` 経由の `from_builtins` 変換でビルド可。

- [ ] **Step 5: テスト・ビルド** — `cargo test features::node` / `cargo build`（挙動不変）。

- [ ] **Step 6: Commit**
```bash
git add -A
git commit -m "refactor(node): NodeColumnSpec/NodeLabelColumn and Vec<NodeColumnSpec>"
```

---

## Task 2: 設定刷新＋共通解決（CLI 名前列・プリセットを registry で解決）

**Files:** `src/config/theme/node.rs`, `src/cmd/args/node_columns.rs`, `src/cmd/command.rs`, `src/app.rs`

- [ ] **Step 1: 設定型**（`config/theme/node.rs`）— `NodeColumnConfig` 廃止:
```rust
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct NodeThemeConfig {
    pub default_preset: Option<String>,
    pub column_presets: Option<HashMap<String, Vec<String>>>,
    pub label_columns: Option<Vec<LabelColumnConfig>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct LabelColumnConfig {
    pub name: String,
    pub label: String,
}
```

- [ ] **Step 2: CLI を名前列に**:
  - `cmd/args/node_columns.rs`: `parse_node_columns(input: &str) -> Result<Vec<String>>`（カンマ分割・trim・空要素除去・空ならエラー）。解決はしない。
  - `cmd/command.rs`: `pub node_columns: Option<Vec<String>>`（`value_parser = parse_node_columns`）。

- [ ] **Step 3: 解決＋検証のテスト（app.rs）**:
```rust
#[test]
fn resolves_preset_with_builtin_and_label_interleaved() {
    let presets = HashMap::from([("gpu".to_string(),
        vec!["name".to_string(), "mig".to_string(), "status".to_string()])]);
    let labels = vec![LabelColumnConfig { name: "mig".into(), label: "nvidia.com/mig.config.state".into() }];
    let cols = build_node_columns(None, None, &Some("gpu".into()), &Some(presets), &Some(labels))
        .unwrap().unwrap();
    assert_eq!(cols.specs(), &[
        NodeColumnSpec::Builtin(NodeColumn::Name),
        NodeColumnSpec::Label { key: "nvidia.com/mig.config.state".into(), header: "MIG".into() },
        NodeColumnSpec::Builtin(NodeColumn::Status),
    ]);
}

#[test]
fn cli_names_resolve_labels_and_take_precedence() {
    let labels = vec![LabelColumnConfig { name: "mig".into(), label: "k".into() }];
    let cols = build_node_columns(
        Some(vec!["name".to_string(), "mig".to_string()]), None, &None, &None, &Some(labels))
        .unwrap().unwrap();
    assert_eq!(cols.specs(), &[
        NodeColumnSpec::Builtin(NodeColumn::Name),
        NodeColumnSpec::Label { key: "k".into(), header: "MIG".into() },
    ]);
}

#[test]
fn error_on_label_name_colliding_with_builtin() {
    let labels = vec![LabelColumnConfig { name: "status".into(), label: "x".into() }];
    let presets = HashMap::from([("p".to_string(), vec!["name".to_string()])]);
    assert!(build_node_columns(None, None, &Some("p".into()), &Some(presets), &Some(labels)).is_err());
}

#[test]
fn error_on_unknown_name() {
    let presets = HashMap::from([("p".to_string(), vec!["name".to_string(), "bogus".to_string()])]);
    assert!(build_node_columns(None, None, &Some("p".into()), &Some(presets), &None).is_err());
}
```

- [ ] **Step 4: 実装（app.rs）**:
```rust
use crate::features::node::{NodeColumn, NodeColumnSpec, NodeColumns, NodeLabelColumn};
use crate::config::theme::LabelColumnConfig;

/// label_columns を解決し、ビルトイン名衝突を検証して registry を返す。
pub fn build_node_label_registry(
    label_columns: &Option<Vec<LabelColumnConfig>>,
) -> Result<Vec<NodeLabelColumn>> {
    let mut out = Vec::new();
    if let Some(defs) = label_columns {
        for def in defs {
            let norm = NodeColumn::normalize_column(&def.name);
            if NodeColumn::from_str(&norm).is_ok() {
                anyhow::bail!("label_columns name '{}' collides with a builtin column name", def.name);
            }
            out.push(NodeLabelColumn {
                name: def.name.clone(),
                key: def.label.clone(),
                header: def.name.to_uppercase(),
            });
        }
    }
    Ok(out)
}

/// 名前列を NodeColumns に解決（ビルトイン or registry ラベル、"full"=全ビルトイン）。
fn resolve_columns(names: &[String], registry: &[NodeLabelColumn]) -> Result<NodeColumns> {
    if names.len() == 1 && NodeColumn::normalize_column(&names[0]) == "full" {
        return Ok(NodeColumns::from_builtins(NodeColumn::iter()));
    }
    let mut specs = Vec::new();
    for name in names {
        let norm = NodeColumn::normalize_column(name);
        if let Ok(builtin) = NodeColumn::from_str(&norm) {
            specs.push(NodeColumnSpec::Builtin(builtin));
        } else if let Some(lc) = registry.iter().find(|lc| NodeColumn::normalize_column(&lc.name) == norm) {
            specs.push(NodeColumnSpec::Label { key: lc.key.clone(), header: lc.header.clone() });
        } else {
            anyhow::bail!("Column '{}' is neither a builtin column nor a defined label column", name);
        }
    }
    Ok(NodeColumns::new(specs).ensure_name_column().dedup_columns())
}

fn build_node_columns(
    cmd_node_columns: Option<Vec<String>>,
    cmd_node_columns_preset: Option<String>,
    default_preset: &Option<String>,
    column_presets: &Option<HashMap<String, Vec<String>>>,
    label_columns: &Option<Vec<LabelColumnConfig>>,
) -> Result<Option<NodeColumns>> {
    let registry = build_node_label_registry(label_columns)?;

    if let Some(names) = cmd_node_columns {
        return Ok(Some(resolve_columns(&names, &registry)?));
    }

    let Some(preset_name) = cmd_node_columns_preset.as_ref().or(default_preset.as_ref()) else {
        return Ok(None);
    };
    let Some(presets) = column_presets else {
        anyhow::bail!("No node column presets defined, but a preset was requested");
    };
    let Some(entries) = presets.get(preset_name) else {
        anyhow::bail!("Node column preset '{}' is not defined in column_presets", preset_name);
    };
    Ok(Some(resolve_columns(entries, &registry)?))
}
```

`run()` 内: `kube_worker_config.node_config.default_columns = build_node_columns(cmd.node_columns, cmd.node_columns_preset, &config.theme.node.default_preset, &config.theme.node.column_presets, &config.theme.node.label_columns)?;`。
また registry を別途構築して Render へ渡す（Task 4 で使用）: `let node_label_columns = build_node_label_registry(&config.theme.node.label_columns)?;`。

- [ ] **Step 5: テスト・ビルド** — `cargo test node_columns` / `cargo build`。

- [ ] **Step 6: Commit**
```bash
git add -A
git commit -m "feat(node): resolve builtin+label columns for CLI and presets with validation"
```

---

## Task 3: poller でラベル値取得（row.object のラベル）

**Files:** `src/features/node/kube/node.rs`

URL は Plan 1 のまま（`includeObject` 不要・実機確認済み）。`row.object.metadata.labels` からラベル列値を抽出する。

- [ ] **Step 1:** `row.object`（`RawExtension`）に `metadata.labels` を持つ Table fixture で、`Label` 列の値抽出を検証するテストを書く（パスは `eq("/api/v1/nodes")` のまま）。
- [ ] **Step 2:** 失敗確認。
- [ ] **Step 3:** `get_node_table` 実装:
  - パスは Plan 1 のまま `Node::url_path(&(), None)`（=`/api/v1/nodes`、クエリなし）。
  - `Builtin` 列のみ `find_indexes`。各行で `specs` を走査: `Builtin`→対応セル、`Label{key,..}`→`row.object.as_ref().map(|o| &o.0)` の `["metadata"]["labels"][key]` 文字列（無ければ空）。
  - ヘッダ `spec.header()`、name は `Builtin(Name)` セル。
- [ ] **Step 4:** テスト・`cargo build`・実機（ラベル列・空セル）。
- [ ] **Step 5:** Commit `feat(node): fetch label column values from Table row.object`。

---

## Task 4: registry をダイアログへ配線＋ビルトイン/ラベル両方トグル

**Files:** `src/app.rs`(Render 呼び出し), `src/workers/render.rs`, `src/workers/render/window.rs`, `src/features/node/view/tab.rs`, `src/features/node/view/widgets/node_columns_dialog.rs`

- [ ] **Step 1: registry を配線**
  - `app.rs`: `Render::new(..., default_node_columns, node_label_columns, ...)` に `node_label_columns: Vec<NodeLabelColumn>` を追加。
  - `render.rs`: `Render` に `node_label_columns: Vec<NodeLabelColumn>` フィールド／`new` 引数／`WindowInit::new` へ。
  - `window.rs`: `WindowInit` に同フィールド／`new` 引数／`NodeTab::new(.., default_node_columns, node_label_columns, theme)` へ。
  - `tab.rs`: `NodeTab::new` が `registry: Vec<NodeLabelColumn>` を受け取り `node_columns_dialog(tx, default_columns, registry, theme)` に渡す。

- [ ] **Step 2: ダイアログ再構築ロジックのテスト**
```rust
#[test]
fn rebuild_preserves_order_and_appends_new() {
    let current = NodeColumns::new([
        NodeColumnSpec::Builtin(NodeColumn::Name),
        NodeColumnSpec::Label { key: "k".into(), header: "MIG".into() },
        NodeColumnSpec::Builtin(NodeColumn::Status),
    ]);
    // 全候補（チェックリスト順）と各 checked 状態から再構築
    // 例: Name=チェック, MIG=チェック, Status=外す, Roles=新規チェック
    let rebuilt = rebuild_columns(&current, &[
        (NodeColumnSpec::Builtin(NodeColumn::Name), true),
        (NodeColumnSpec::Label { key: "k".into(), header: "MIG".into() }, true),
        (NodeColumnSpec::Builtin(NodeColumn::Status), false),
        (NodeColumnSpec::Builtin(NodeColumn::Roles), true),
    ]);
    assert_eq!(rebuilt.specs(), &[
        NodeColumnSpec::Builtin(NodeColumn::Name),
        NodeColumnSpec::Label { key: "k".into(), header: "MIG".into() },
        NodeColumnSpec::Builtin(NodeColumn::Roles),
    ]);
}
```
`rebuild_columns(current, &[(spec, checked)])`: 現順序を保ちつつ checked のみ残し、current に無い checked を末尾追加。

- [ ] **Step 3: ダイアログ実装**
  - 候補 = 全ビルトイン（`NodeColumn::iter()` → `NodeColumnSpec::Builtin`）＋ registry の各 `NodeLabelColumn`（→ `NodeColumnSpec::Label{key,header}`）。
  - `CheckListItem` の `label` は `spec.header()`、`checked` は現 `default_columns`(specs) に含まれるか、`required` は `Builtin(Name)` のみ。
  - 候補 `Vec<NodeColumnSpec>` を**項目と同順**で closure に保持。`on_change` で `widget.items()` の checked と保持 specs を index 対応させ、`rebuild_columns(&current, &pairs)` で `NodeColumns` を作り `NodeMessage::Request` 送信。
  - 「現順序保持」のため、`current`（=送信直前の状態）はダイアログ内に保持して更新するか、`shared_node_columns` の最新を都度反映する設計にする（実装時に最小で確定。最低限、候補順での再構築でも可だが、できれば現順序保持）。

- [ ] **Step 4:** テスト（`features::node::view`）・`cargo build`・実機（`t` でビルトイン/ラベル両方トグル、ラベル保持）。

- [ ] **Step 5:** Commit `feat(node): toggle builtin and label columns in dialog`。

---

## Task 5: example/config.yaml と スペック更新

**Files:** `example/config.yaml`, `docs/superpowers/specs/2026-05-22-node-tab-design.md`

- [ ] example/config.yaml の `theme.node` を (A) 形式へ:
```yaml
  node:
    label_columns:
      - name: mig
        label: nvidia.com/mig.config.state
      - name: zone
        label: failure-domain.beta.kubernetes.io/zone
    column_presets:
      default: [name, status, roles, age, version]
      gpu:     [name, mig, status, roles]
      topology: [name, status, zone]
    default_preset: default
```
- [ ] スペック「列設定」節を (A)（NodeColumnSpec／registry／インターリーブ／CLI 名前解決／ダイアログ全トグル）へ更新。
- [ ] Commit `docs(node): document label columns (interleave, toggle, CLI) in example and spec`。

---

## Task 6: 仕上げ（fmt / clippy / 全テスト ＋ 実機）

- [ ] `cargo +nightly fmt`（変更ファイルのみ）／`cargo clippy --all-targets`（新規警告なし）／`cargo test`（全 PASS）
- [ ] 実機 `cargo run -- --config-file example/config.yaml`：`gpu`/`topology` プリセットでインターリーブ・空セル、`t` でビルトイン/ラベルのトグル、`--node-columns=name,mig,status` 動作を確認
- [ ] Commit（fmt 差分があれば）

---

## 後続プラン

- **Plan 4**: 詳細ペイン（2 ペイン化、`NodeDetailWorker`：Node YAML〔managedFields 除去〕＋関連 Pod、3 秒更新、`on_select`）。
- **Plan 5**: フィルタ（`node:`/`!node:`/`label:`、nom パーサ、`shared_node_filter`、`labelSelector`、フィルタ入力ウィジェット＋ヘルプダイアログ）。
