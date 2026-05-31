# Pod label columns 実装計画

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Pod テーブルに label columns（任意のラベル値を列として表示・フィルタ可）を Node と同型で追加する。

**Architecture:** Node の `NodeColumnSpec::{Builtin, Label}` / `NodeLabelColumn` を雛形に Pod 側に同型を導入。config の `theme.pod.label_columns` で定義した label 列を、`column_presets`（`Vec<String>` 化）／列ダイアログ／poller／フィルタが受け入れる。Poller は `row.object.metadata.labels[key]` を Table API レスポンスから読む（追加 API コール無し）。namespace の Z 案内は parser 内で最初に評価して維持。

**Tech Stack:** Rust 2021、tokio、kube/k8s-openapi、strum、serde、`#[cfg(test)]` インラインテスト、pretty_assertions。

参照 spec: `docs/superpowers/specs/2026-05-31-pod-label-columns-design.md`
注: kubetui は binary crate。テストは `cargo test <path>` / `cargo test --all`（`--lib` 不可）。
雛形: Node の同機能（PR #920 Plan 3 = `src/features/node/node_columns.rs` / `src/app.rs::build_node_label_registry,resolve_columns` / `src/features/node/view/widgets/node_columns_dialog.rs` / `src/features/node/kube/node.rs::get_node_table`）。

---

## ファイル構成

- Modify: `src/features/pod/pod_columns.rs` — `PodColumnSpec`/`PodLabelColumn` 追加、`PodColumns` 内部型変更、`columns()` → `specs()`。
- Modify: `src/features/pod.rs` — 新型を re-export。
- Modify: `src/config/theme/pod.rs` — `column_presets` 値型を `Vec<String>` に、`label_columns` 追加、`PodColumnConfig` と関連 impl 削除。
- Modify: `src/app.rs` — `build_pod_label_registry` / `resolve_pod_columns` 追加、`build_pod_columns` を `Vec<String>` ＋ registry 経路に、PodTab::new に registry を渡す。
- Modify: `src/features/pod/kube/pod.rs` — poller のセル構築を spec 駆動化、Label は `row.object.metadata.labels[key]` から取得。
- Modify: `src/features/pod/view/widgets/pod.rs` — `pod_widget(tx, label_registry, theme)` 化、applicator に registry を渡す。
- Modify: `src/features/pod/view/widgets/pod_columns_dialog.rs` — `pod_columns_dialog(tx, default_columns, label_registry, theme)`、`candidate_specs`、CheckListItem の metadata で kind/builtin/label を保持。
- Modify: `src/features/pod/view/tab.rs` — `PodTab::new(..., label_registry, ...)` に追加、widget/dialog に渡す。
- Modify: `src/features/pod/filter.rs` — `pod_filter_applicator(label_registry, tx)` 化。
- Modify: `src/features/pod/filter/parser.rs` — `parse_pod_filter(input, label_registry)` 化、既知列に registry header を含める（namespace 特例は順序的に最初）。

各タスクは依存順に並べてある。各タスク終了時に `cargo test --all` が緑になるよう中間状態を保つ。

---

## Task 1: 型 — `PodColumnSpec` / `PodLabelColumn` 導入、`PodColumns` 内部型変更

挙動不変。Pod は引き続き builtin 列のみ扱うが、内部表現が `Vec<PodColumnSpec>` になる。`columns()` を `specs()` に改名し、現在の全呼び出し元（poller, dialog, config の From impl, app.rs build_pod_columns, テスト）を spec ベースに更新する。Node の `src/features/node/node_columns.rs` が形状の正典。

**Files:**
- Modify: `src/features/pod/pod_columns.rs`
- Modify: `src/features/pod.rs`
- Modify: `src/features/pod/kube/pod.rs`（spec ベース呼び出しに）
- Modify: `src/features/pod/view/widgets/pod_columns_dialog.rs`（spec ベース呼び出しに）
- Modify: `src/config/theme/pod.rs`（`From<&[PodColumnConfig]> for PodColumns` を spec 経路に更新、PodColumnConfig 自体は Task 2 で削除）
- Modify: `src/app.rs`（必要なら `from_builtins` への置き換え）

- [ ] **Step 1: `PodColumnSpec` / `PodLabelColumn` を追加**

`src/features/pod/pod_columns.rs` の冒頭、既存 `PodColumns` 構造体の前に追加:

```rust
/// A runtime column in the pod table: a built-in column or a label column.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PodColumnSpec {
    Builtin(PodColumn),
    Label { key: String, header: String },
}

impl PodColumnSpec {
    /// Display header (uppercase). Builtin uses display(), Label uses its header.
    pub fn header(&self) -> String {
        match self {
            PodColumnSpec::Builtin(c) => c.display().to_string(),
            PodColumnSpec::Label { header, .. } => header.clone(),
        }
    }
}

/// A resolved label-column definition (an entry of the label registry).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PodLabelColumn {
    pub name: String,
    pub key: String,
    pub header: String,
}
```

- [ ] **Step 2: `PodColumns` 内部型を `Vec<PodColumnSpec>` に**

`src/features/pod/pod_columns.rs` の `PodColumns` 関連を次に置き換える（Node の `NodeColumns` を雛形に。`full()` の意味は維持＝全 builtin 展開）:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PodColumns {
    columns: Vec<PodColumnSpec>,
}

impl Default for PodColumns {
    fn default() -> Self {
        PodColumns {
            columns: DEFAULT_POD_COLUMNS
                .iter()
                .copied()
                .map(PodColumnSpec::Builtin)
                .collect(),
        }
    }
}

impl PodColumns {
    pub fn new(columns: impl IntoIterator<Item = PodColumnSpec>) -> Self {
        PodColumns {
            columns: columns.into_iter().collect(),
        }
    }

    pub fn from_builtins(columns: impl IntoIterator<Item = PodColumn>) -> Self {
        PodColumns {
            columns: columns.into_iter().map(PodColumnSpec::Builtin).collect(),
        }
    }

    pub fn full() -> Self {
        Self::from_builtins(PodColumn::iter())
    }

    pub fn specs(&self) -> &[PodColumnSpec] {
        &self.columns
    }

    pub fn ensure_name_column(mut self) -> Self {
        let has_name = self
            .columns
            .iter()
            .any(|s| matches!(s, PodColumnSpec::Builtin(PodColumn::Name)));
        if !has_name {
            self.columns
                .insert(0, PodColumnSpec::Builtin(PodColumn::Name));
        }
        self
    }

    pub fn dedup_columns(self) -> Self {
        let mut unique: Vec<PodColumnSpec> = Vec::new();
        for c in self.columns {
            if !unique.contains(&c) {
                unique.push(c);
            }
        }
        PodColumns { columns: unique }
    }
}
```

（既存 `pub fn columns(&self) -> &[PodColumn]` は削除。`new(IntoIterator<Item=PodColumn>)` も削除し `from_builtins` を用いる。`DEFAULT_POD_COLUMNS` の宣言と `PodColumn` enum・impl・`FromStr` はそのまま残す。）

- [ ] **Step 3: `pod.rs` モジュールの re-export を更新**

`src/features/pod.rs` を次に置き換える:

```rust
mod filter;
pub mod kube;
pub mod message;
mod pod_columns;
pub mod view;

pub use filter::pod_filter_applicator;
pub use pod_columns::{PodColumn, PodColumnSpec, PodColumns, PodLabelColumn};
```

- [ ] **Step 4: Poller の呼び出し更新（spec ベースだが builtin のみ通る経路）**

`src/features/pod/kube/pod.rs` の `get_pod_info` と `get_pods_per_namespace` で `pod_columns.columns()` を全て `pod_columns.specs()` に置換し、要素型が `&PodColumnSpec` に変わる箇所を以下のように調整する（label 値の読み出しは Task 4 で追加。本タスクでは Label が来ない前提 + 来た場合のフォールバックで空文字を返すよう書き、コンパイル可能にする）。

`display_columns` の構築:

```rust
        let mut display_columns: Vec<String> = pod_columns
            .specs()
            .iter()
            .map(|s| s.header())
            .collect();
```

`get_pods_per_namespace` 冒頭の各種インデックス・列名収集を spec 経由に:

```rust
        let name_index = pod_columns
            .specs()
            .iter()
            .position(|s| matches!(s, PodColumnSpec::Builtin(PodColumn::Name)))
            .expect("Name column must be present in pod columns");

        let status_index = pod_columns
            .specs()
            .iter()
            .position(|s| matches!(s, PodColumnSpec::Builtin(PodColumn::Status)));

        // builtin 列名のみ集める（label は Task 4 で別経路）
        let columns: Vec<&str> = pod_columns
            .specs()
            .iter()
            .filter_map(|s| {
                match s {
                    PodColumnSpec::Builtin(c) => Some(c.as_str()),
                    PodColumnSpec::Label { .. } => None,
                }
            })
            .collect();
```

行ごとのクロージャ内の `indexes.iter().map(|i| row.cells[*i].to_string())` も、本タスクでは「Label が来た場合は空文字」にして spec の数だけセルを並べる形に変える（Task 4 で `row.object` 経路を追加）:

```rust
                move |row: &TableRow, indexes: &[usize]| {
                    let mut builtin_iter = indexes.iter();
                    let mut row_cells: Vec<String> = pod_columns_specs
                        .iter()
                        .map(|s| {
                            match s {
                                PodColumnSpec::Builtin(_) => {
                                    let i = builtin_iter.next().expect("builtin index available");
                                    row.cells[*i].to_string()
                                }
                                PodColumnSpec::Label { .. } => String::new(),
                            }
                        })
                        .collect();
```

クロージャに渡すため、`get_pods_per_namespace` の冒頭で `let pod_columns_specs: Vec<PodColumnSpec> = pod_columns.specs().to_vec();` を取り、`move` 内で `pod_columns_specs` を再 clone or borrow できる形にする（クロージャ毎に独立して使えるよう `pod_columns_specs.clone()` を渡すか `Arc` 化。実装が簡単なら毎ループ `clone`）。row_cells の後の name/status/insert_ns/ANSI 着色ロジックは既存どおり。

- [ ] **Step 5: Dialog の呼び出し更新（spec ベースに）**

`src/features/pod/view/widgets/pod_columns_dialog.rs` の `default_columns.columns().iter()` 等の呼び出しを全て `specs()` に置換。`PodColumn::from_str(&i.label)` のような builtin 限定の逆変換は本タスクでは残してよい（label 対応は Task 5 で完全置換）。少なくとも `pub fn pod_columns_dialog(tx, default_columns, theme)` のシグネチャは本タスクでは不変。

- [ ] **Step 6: config の `From` impl を spec 経由に更新**

`src/config/theme/pod.rs` の `impl From<T: AsRef<[PodColumnConfig]>> for PodColumns` を次に変更（PodColumnConfig はまだ存在するが、`PodColumns::from_builtins` 経由に）:

```rust
impl<T: AsRef<[PodColumnConfig]>> From<T> for PodColumns {
    fn from(value: T) -> Self {
        PodColumns::from_builtins(value.as_ref().iter().map(|c| c.0))
    }
}
```

（PodColumnConfig 自体の削除は Task 2 で。本タスクはコンパイル通過を優先。）

- [ ] **Step 7: app.rs `build_pod_columns` の調整**

`src/app.rs` の `build_pod_columns` 内で `PodColumns::new(...)` を呼んでいる箇所があれば、要素型が `PodColumnSpec` を期待するようになっているので、`PodColumns::from_builtins(...)` に置き換える（既存 `PodColumns::new(impl IntoIterator<Item=PodColumn>)` の呼び出し）。具体には `PodColumns::new(entries.iter().map(|e| e.0))` 等の箇所を `PodColumns::from_builtins(entries.iter().map(|e| e.0))` に修正。

- [ ] **Step 8: テストの修正**

`src/features/pod/pod_columns.rs` の `mod tests` で `actual.columns()` を `actual.specs()` に置換。比較対象は `Vec<PodColumnSpec>` になるので、`vec![PodColumn::Name, ...]` のような期待値は `vec![PodColumnSpec::Builtin(PodColumn::Name), ...]` に置き換える（または `from_builtins(...)` で作って比較）。

`src/config/theme/pod.rs` のテストも同様に specs 経由比較に。

- [ ] **Step 9: ビルドと全テスト**

Run: `cargo test --all 2>&1 | rg "test result:" | tail -3`
Expected: 全テスト PASS（挙動不変＝既存の主要テストはそのまま通る、型関連テストは新シグネチャに更新済み）。

- [ ] **Step 10: コミット**

```bash
git add src/features/pod/pod_columns.rs src/features/pod.rs src/features/pod/kube/pod.rs src/features/pod/view/widgets/pod_columns_dialog.rs src/config/theme/pod.rs src/app.rs
git commit -m "refactor(pod): introduce PodColumnSpec; PodColumns holds Vec<PodColumnSpec>"
```

---

## Task 2: config — `column_presets: Vec<String>` 化、`label_columns` 追加、`PodColumnConfig` 削除

挙動不変（YAML 互換）。preset 解決は app.rs に移動し、本タスクでは「文字列リスト」までを受け取る形に。

**Files:**
- Modify: `src/config/theme/pod.rs`
- Modify: `src/app.rs`（preset の取り回しを `Vec<String>` 経路へ更新）

- [ ] **Step 1: `PodThemeConfig` スキーマを更新**

`src/config/theme/pod.rs` を次に変更:

- 冒頭 import から `use crate::features::pod::{PodColumn, PodColumns};` を削除（不要になる）。代わりに `use super::node::LabelColumnConfig;` を追加（Node 側の型を再利用）。
- `pub struct PodThemeConfig` を次に変更:

```rust
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PodThemeConfig {
    #[serde(default = "default_highlights")]
    pub highlights: Vec<PodHighlightConfig>,

    pub default_preset: Option<String>,

    pub column_presets: Option<HashMap<String, Vec<String>>>,

    pub label_columns: Option<Vec<LabelColumnConfig>>,
}

impl Default for PodThemeConfig {
    fn default() -> Self {
        Self {
            highlights: default_highlights(),
            default_preset: None,
            column_presets: None,
            label_columns: None,
        }
    }
}
```

`PodColumnConfig` struct、`serde_pod_column` モジュール、`impl<T: AsRef<[PodColumnConfig]>> From<T> for PodColumns` を**削除**（用途消滅）。

- [ ] **Step 2: app.rs の preset 経路を `Vec<String>` 化**

`src/app.rs` で `column_presets: &Option<HashMap<String, Vec<PodColumnConfig>>>` を引数に取っている箇所を `column_presets: &Option<HashMap<String, Vec<String>>>` に変更。`build_pod_columns` の本体で preset を引いて作る部分:

- Before: `let columns = PodColumns::from(entries.as_slice());`
- After（label registry は Task 3 で追加するので、本タスクでは builtin 限定の臨時実装）:

```rust
        let columns = PodColumns::from_builtins(
            entries.iter()
                .map(|s| PodColumn::from_str(s))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| anyhow::anyhow!(
                    "Pod column preset '{}' contains an unknown column: {}", preset, e
                ))?
        );
```

（Task 3 で `resolve_pod_columns(entries, &pod_label_registry)` に置き換わるため、本タスクは一時実装。`PodColumn::from_str` が unknown ならエラー、というのが従来 PodColumnConfig での挙動と等価。）

`default_preset` 経路（CLI 未指定でも config に preset があれば使う）も同形に修正。

`use crate::config::theme::PodColumnConfig;` の import は削除。

- [ ] **Step 3: theme/pod.rs テストの更新**

`#[cfg(test)] mod tests` がある場合（preset / 既定 / 不正な PodColumn 値などのテスト）、新スキーマに合わせて修正。`PodColumnConfig` 直接参照を `String`/`Vec<String>` に置換。新フィールド `label_columns` のシリアライズ／デシリアライズも1テスト追加:

```rust
    #[test]
    fn deserializes_label_columns_and_string_presets() {
        let json = r#"{
            "column_presets": { "wide": ["name", "status", "version"] },
            "label_columns": [{ "name": "version", "label": "app.kubernetes.io/version" }]
        }"#;
        let cfg: PodThemeConfig = serde_json::from_str(json).unwrap();
        assert_eq!(
            cfg.column_presets.as_ref().unwrap().get("wide").unwrap(),
            &vec!["name".to_string(), "status".to_string(), "version".to_string()]
        );
        let labels = cfg.label_columns.as_ref().unwrap();
        assert_eq!(labels.len(), 1);
        assert_eq!(labels[0].name, "version");
        assert_eq!(labels[0].label, "app.kubernetes.io/version");
    }
```

- [ ] **Step 4: ビルドとテスト**

Run: `cargo test --all 2>&1 | rg "test result:"`
Expected: 全テスト PASS。

- [ ] **Step 5: コミット**

```bash
git add src/config/theme/pod.rs src/app.rs
git commit -m "feat(pod-config): label_columns + column_presets: Vec<String>; drop PodColumnConfig"
```

---

## Task 3: app.rs — `build_pod_label_registry` / `resolve_pod_columns` + registry を PodTab に配線

label registry を構築し、preset 解決にも使う。registry を PodTab に渡して widget/dialog/filter まで届ける枠を作る（実際に label を消費するのは Task 4-6）。

**Files:**
- Modify: `src/app.rs`
- Modify: `src/features/pod/view/tab.rs`
- Modify: `src/features/pod/view/widgets/pod.rs`
- Modify: `src/features/pod/view/widgets/pod_columns_dialog.rs`

- [ ] **Step 1: `build_pod_label_registry` / `resolve_pod_columns` を追加**

`src/app.rs` に Node 同型関数を追加（Node の `build_node_label_registry`/`resolve_columns` をミラー、PodColumn 用に書き換え）:

```rust
fn build_pod_label_registry(
    label_columns: &Option<Vec<LabelColumnConfig>>,
) -> Result<Vec<PodLabelColumn>> {
    let mut out = Vec::new();
    if let Some(defs) = label_columns {
        for def in defs {
            let norm = PodColumn::normalize_column(&def.name);
            if PodColumn::from_str(&norm).is_ok() {
                anyhow::bail!(
                    "label_columns name '{}' collides with a builtin column name",
                    def.name
                );
            }
            out.push(PodLabelColumn {
                name: def.name.clone(),
                key: def.label.clone(),
                header: def.name.to_uppercase(),
            });
        }
    }
    Ok(out)
}

fn resolve_pod_columns(names: &[String], registry: &[PodLabelColumn]) -> Result<PodColumns> {
    if names.len() == 1 && PodColumn::normalize_column(&names[0]) == "full" {
        return Ok(PodColumns::full());
    }

    let mut specs = Vec::new();
    for name in names {
        let norm = PodColumn::normalize_column(name);
        if let Ok(builtin) = PodColumn::from_str(&norm) {
            specs.push(PodColumnSpec::Builtin(builtin));
        } else if let Some(lc) = registry
            .iter()
            .find(|lc| PodColumn::normalize_column(&lc.name) == norm)
        {
            specs.push(PodColumnSpec::Label {
                key: lc.key.clone(),
                header: lc.header.clone(),
            });
        } else {
            anyhow::bail!(
                "Pod column '{}' is neither a builtin column nor a defined label column",
                name
            );
        }
    }

    Ok(PodColumns::new(specs).ensure_name_column().dedup_columns())
}
```

import に `PodColumnSpec`, `PodLabelColumn` を追加。

- [ ] **Step 2: `build_pod_columns` を `resolve_pod_columns` 経由に**

Task 2 で書いた一時実装（`PodColumn::from_str` 直接）を、`resolve_pod_columns(entries, &pod_label_registry)` に置き換える。`build_pod_columns` の引数に `pod_label_registry: &[PodLabelColumn]` を追加。

- [ ] **Step 3: app.rs で `pod_label_registry` を構築**

`build_pod_columns` を呼ぶ前に `let pod_label_registry = build_pod_label_registry(&config.theme.pod.label_columns)?;` を作り、`build_pod_columns(..., &pod_label_registry)` に渡す。さらに `kube_worker_config.pod_label_registry = pod_label_registry.clone();` のように後続（PodTab::new）に届ける手段を用意する。

簡便な方法: `PodTab::new` 呼び出し時に直接 `pod_label_registry.clone()` を渡す（`KubeWorkerConfig` を経由しない経路）。app.rs の中で PodTab::new を呼ぶ箇所を変更（PodTab がここで構築されているはず — `default_pod_columns` を渡している関連を grep して見つける）。

- [ ] **Step 4: PodTab::new に `label_registry` を追加**

`src/features/pod/view/tab.rs` の `PodTab::new` シグネチャに `label_registry: Vec<PodLabelColumn>` を追加（既存の `default_columns: Option<PodColumns>` の隣など、自然な位置に）。本体で `pod_widget(tx, label_registry.clone(), theme.clone())` と `pod_columns_dialog(tx, default_columns, label_registry, theme.clone())` に渡す。`use` に `crate::features::pod::PodLabelColumn` 追加。

- [ ] **Step 5: `pod_widget` シグネチャ更新**

`src/features/pod/view/widgets/pod.rs` の `pod_widget(tx: &Sender<Message>, theme: WidgetThemeConfig)` を `pod_widget(tx: &Sender<Message>, label_registry: Vec<PodLabelColumn>, theme: WidgetThemeConfig)` に。`use crate::features::pod::{ ..., PodLabelColumn, ... }` を追加。本タスクでは `label_registry` をまだ使わない（filter applicator への配線は Task 6）。`#[allow(unused_variables)]` で受けるか、`let _ = label_registry;` で警告を回避してもよい（次タスクで使われるので一時的）。

- [ ] **Step 6: `pod_columns_dialog` シグネチャ更新**

`src/features/pod/view/widgets/pod_columns_dialog.rs` の `pod_columns_dialog(tx, default_columns, theme)` に `label_registry: Vec<PodLabelColumn>` を追加。本タスクでは未使用でよい（Task 5 で消費）。

- [ ] **Step 7: ビルドとテスト**

Run: `cargo test --all 2>&1 | rg "test result:"`
Expected: 全テスト PASS。registry は配線済みだが用途は次タスク以降。

app.rs の `node_columns_tests` モジュールに倣い、新規 `pod_columns_tests` を追加（`build_pod_label_registry` の衝突エラーケース、`resolve_pod_columns` の builtin / label / 混合 / "full" / 未定義名 / ensure_name / dedup を網羅）。Node 側のテストをコピーして PodColumn 用に書き換える。

- [ ] **Step 8: コミット**

```bash
git add src/app.rs src/features/pod/view/tab.rs src/features/pod/view/widgets/pod.rs src/features/pod/view/widgets/pod_columns_dialog.rs
git commit -m "feat(pod-config): build_pod_label_registry + resolve_pod_columns; thread registry"
```

---

## Task 4: Poller — label 値レンダリング

`get_pods_per_namespace` の行クロージャで `PodColumnSpec::Label { key, .. }` のとき `row.object.metadata.labels[key]` から値を読み出す（Node `get_node_table` の `NodeColumnSpec::Label` 分岐と同パターン、`src/features/node/kube/node.rs:125-133` 参照）。

**Files:**
- Modify: `src/features/pod/kube/pod.rs`

- [ ] **Step 1: クロージャの Label 分岐を `row.object` 経由に**

Task 1 Step 4 で書いた `PodColumnSpec::Label { .. } => String::new(),` の枝を次に置き換える（key を捕捉して使う）:

```rust
                                PodColumnSpec::Label { key, .. } => {
                                    row.object
                                        .as_ref()
                                        .and_then(|o| o.0.get("metadata"))
                                        .and_then(|m| m.get("labels"))
                                        .and_then(|l| l.get(key))
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string()
                                }
```

（`row.object` の型は `Option<RawExtension>`、`.0` で `serde_json::Value` を取れる前提。`get_node_table` での扱いと同じ。型が異なる場合は Node のコードを参照して合わせる。`pod_columns_specs.clone()` の closure capture は Task 1 で済んでいる前提。）

- [ ] **Step 2: テスト — label 値レンダリングをモック Table response で確認**

`src/features/pod/kube/pod.rs` の既存 `mod tests`（あれば）または新規追加。Node の `src/features/node/kube/node.rs` のテストを雛形に、TableRow に `object: Some(RawExtension(JsonValue::object(...)))` を仕込んで `metadata.labels[key]` から値が出ることを確認。最低限:
- builtin のみのケース（既存挙動）
- label 列を含み、ラベル値が存在するケース
- label 列を含み、ラベル値が無い（空文字）ケース

Node のテストを参考に Pod ハーネスで書き起こす。

- [ ] **Step 3: 全テスト**

Run: `cargo test --all 2>&1 | rg "test result:"`
Expected: 全テスト PASS（新規の label テスト含む）。

- [ ] **Step 4: コミット**

```bash
git add src/features/pod/kube/pod.rs
git commit -m "feat(pod-poller): render label-column values from row.object.metadata.labels"
```

---

## Task 5: Column dialog — label 候補対応

Node の `node_columns_dialog`（`src/features/node/view/widgets/node_columns_dialog.rs`）をミラー。CheckListItem の metadata に `kind=builtin|label` と必要情報を持たせ、保存時に正しく `PodColumnSpec` を組み立てる。

**Files:**
- Modify: `src/features/pod/view/widgets/pod_columns_dialog.rs`

- [ ] **Step 1: 本体を Node ミラーに置き換え**

`src/features/pod/view/widgets/pod_columns_dialog.rs` の全体を Node の `node_columns_dialog.rs` を雛形に Pod 用に書き換える。具体には次の関数群を Pod 化:

- `pod_columns_dialog(tx, default_columns, label_registry, theme)` — 既に Task 3 でシグネチャ更新済み。
- `candidate_specs(label_registry: &[PodLabelColumn]) -> Vec<PodColumnSpec>` — builtin（`PodColumn::iter().map(PodColumnSpec::Builtin)`）＋ registry の Label。
- `build_check_list_items(default_columns, label_registry)` — current の specs を先頭にチェック済みで、未選択の candidates を後ろに未チェックで並べる。
- `make_item(spec, checked)` — `label = spec.header()`、`required = matches!(spec, PodColumnSpec::Builtin(PodColumn::Name))`、`metadata = metadata_for(spec)`。
- `metadata_for(spec)` — Builtin は `{kind: "builtin", id: c.as_str()}`、Label は `{kind: "label", key, header}`。
- `spec_from_item(item)` — metadata から `PodColumnSpec` を復元。Builtin は `PodColumn::from_str(id)`、Label は `key`/`header` を読み戻し。
- `collect_columns(items)` — チェック済み（または required）の項目を `spec_from_item` で `Vec<PodColumnSpec>` に集めて `PodColumns::new(specs).ensure_name_column()`。
- `on_change(tx)` — `PodMessage::Request(collect_columns(widget.items()))` を送る。

Node のテスト 1-2 個（`選択列を先頭に...`、`label_spec(...)` ヘルパー使用）も Pod 用に書き換えてコピー。

- [ ] **Step 2: 全テスト**

Run: `cargo test --all 2>&1 | rg "test result:"`
Expected: 全テスト PASS。

- [ ] **Step 3: コミット**

```bash
git add src/features/pod/view/widgets/pod_columns_dialog.rs
git commit -m "feat(pod-dialog): show label columns as candidates; persist via metadata"
```

---

## Task 6: Filter — `pod_filter_applicator(label_registry, tx)` ＋ parser が registry を既知列に含める

namespace の Z 案内は順序的に最初に評価して維持。registry header の正規化を既知列集合に加える。

**Files:**
- Modify: `src/features/pod/filter.rs`
- Modify: `src/features/pod/filter/parser.rs`
- Modify: `src/features/pod/view/widgets/pod.rs`（applicator 呼び出し）

- [ ] **Step 1: `parse_pod_filter` に registry を渡す**

`src/features/pod/filter/parser.rs` の `parse_pod_filter` を次に置き換える:

```rust
use std::collections::HashSet;

use strum::IntoEnumIterator;

use crate::{
    features::pod::{PodColumn, PodLabelColumn},
    ui::widget::{normalize_column_name, parse_table_filter, TableFilterPredicate},
};

/// Parse a Pod-filter input string into a `TableFilterPredicate`.
///
/// `namespace:` is rejected with a guidance message (namespace is selected via
/// the namespace selector, not the filter). Other columns are validated
/// against the builtin `PodColumn` set plus the defined label-column headers
/// in `label_registry`; a column not in that set produces `unknown column '<x>'`.
pub fn parse_pod_filter(
    input: &str,
    label_registry: &[PodLabelColumn],
) -> Result<TableFilterPredicate, String> {
    let valid: HashSet<String> = PodColumn::iter()
        .map(|c| normalize_column_name(c.display()))
        .chain(
            label_registry
                .iter()
                .map(|lc| normalize_column_name(&lc.header)),
        )
        .collect();
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
```

既存の `mod tests` に追加（Node の `header_column_is_accepted` 同型）:

```rust
    fn registry_with(name: &str, header: &str) -> Vec<PodLabelColumn> {
        vec![PodLabelColumn {
            name: name.to_string(),
            key: "irrelevant.example.com/key".to_string(),
            header: header.to_string(),
        }]
    }

    #[test]
    fn registered_label_column_is_accepted() {
        let regs = registry_with("version", "VERSION");
        let p = parse_pod_filter("version:1.30", &regs).unwrap();
        assert!(p.column_includes.contains_key("version"));
    }

    #[test]
    fn namespace_guidance_takes_precedence_over_registry() {
        // 仮に "namespace" header の label を登録しても、namespace は Z 案内を優先。
        let regs = registry_with("namespace", "NAMESPACE");
        let err = parse_pod_filter("namespace:default", &regs).unwrap_err();
        assert_eq!(
            err,
            "namespace is selected via the namespace selector, not the filter"
        );
    }
```

既存テスト（`empty_input_yields_empty_predicate` 等）の `parse_pod_filter(...)` 呼び出しは全て `parse_pod_filter(..., &[])` のように空 registry を渡す形に修正（registry 引数追加に追随）。

- [ ] **Step 2: `pod_filter_applicator` に registry を渡す**

`src/features/pod/filter.rs` を次に置き換える（registry を capture）:

```rust
pub fn pod_filter_applicator(
    label_registry: Vec<PodLabelColumn>,
    tx: Sender<Message>,
) -> TableFilterApplicator {
    let parser: TableFilterParser =
        (move |input: &str| parse_pod_filter(input, &label_registry)).into();

    let tx_apply = tx.clone();
    let tx_cancel = tx;

    let on_apply: OnFilterApply = (move |predicate: &crate::ui::widget::TableFilterPredicate, _window: &mut Window| {
        tx_apply
            .send(PodMessage::Filter(predicate.label_selector.clone()).into())
            .expect("Failed to send PodMessage::Filter");
    })
    .into();

    let on_cancel: OnFilterCancel = (move |_window: &mut Window| {
        tx_cancel
            .send(PodMessage::Filter(None).into())
            .expect("Failed to send PodMessage::Filter(None) on cancel");
    })
    .into();

    TableFilterApplicator::new(parser, ApplyStrategy::EnterToConfirm)
        .with_help_dialog(POD_FILTER_HELP_DIALOG_ID)
        .with_on_apply(on_apply)
        .with_on_cancel(on_cancel)
}
```

import に `crate::features::pod::PodLabelColumn` を追加。

同ファイルのテストも registry 引数追加に追随:

```rust
    #[test]
    fn applicator_constructs_without_panic() {
        let (tx, _rx) = crossbeam::channel::bounded(1);
        let _ = pod_filter_applicator(Vec::new(), tx);
    }
```

- [ ] **Step 3: `pod_widget` が registry を applicator に渡す**

`src/features/pod/view/widgets/pod.rs` で:

```rust
        .filter_applicator(pod_filter_applicator(label_registry.clone(), tx.clone()))
```

（`label_registry` は Task 3 Step 5 で関数引数に追加済み。）

- [ ] **Step 4: 全テスト**

Run: `cargo test --all 2>&1 | rg "test result:"`
Expected: 全テスト PASS。

- [ ] **Step 5: コミット**

```bash
git add src/features/pod/filter.rs src/features/pod/filter/parser.rs src/features/pod/view/widgets/pod.rs
git commit -m "feat(pod-filter): include label-column headers in the known set (registry-aware)"
```

---

## Task 7: 全体検証

**Files:** なし（検証のみ）

- [ ] **Step 1: 全テスト**

Run: `cargo test --all 2>&1 | rg "test result:"`
Expected: 全テスト PASS（新規 Pod label テスト含む）。

- [ ] **Step 2: clippy**

Run: `cargo clippy --all-targets 2>&1 | rg "src/features/pod|src/app.rs|src/config/theme/pod"`
Expected: 変更ファイルに新規警告なし（未使用 import / 死コードが残っていないか）。

- [ ] **Step 3: format**

Run: `cargo +nightly fmt --check 2>&1 | rg "Diff in" | rg -v "store.rs"`
Expected: 変更ファイルに差分なし（出たら `cargo +nightly fmt` を実行して再コミット。store.rs の pre-existing drift は無視）。

- [ ] **Step 4: 手動スモーク（実クラスタ／KIND、不可なら省略明記）**

`~/.config/kubetui/config.yaml` に次のような設定を追加してから `cargo run` で Pod タブを開いて確認:

```yaml
theme:
  pod:
    label_columns:
      - name: app
        label: app.kubernetes.io/name
      - name: version
        label: app.kubernetes.io/version
    column_presets:
      detailed: ["name", "status", "app", "version", "age"]
```

確認項目:
1. `t` で列ダイアログを開く → `APP` と `VERSION` が候補に出る → チェックして閉じる → 各 pod 行に label 値が表示される（無いラベルは空セル）。
2. `/` → `app:<value>` → Enter → APP 列で絞り込まれる。`version:<value>` も同様。
3. `/` → `app: <value>` などの不正、または `nonexistent_label:foo` → `unknown column` エラー。
4. `/` → `namespace:default` → 従来どおり namespace の Z 案内（label_columns 追加で壊れていない）。
5. `t` で APP を非表示 → `(inactive: app)` バッジ表示、行は残る → 再表示で復活（framework 由来、Pod でも自動継承）。
6. CLI: `cargo run -- --pod-columns-preset detailed` → preset で起動時から label 列が出る。

- [ ] **Step 5: （必要なら）fmt 修正をコミット**

```bash
git add -A
git commit -m "style: cargo fmt"
```

---

## Self-Review

- **Spec カバレッジ:**
  - Config（spec §1）→ Task 2
  - 型（spec §2）→ Task 1
  - 解決（spec §3）→ Task 3
  - Poller（spec §4）→ Task 1 Step 4（spec 骨格）＋ Task 4（label 値）
  - Dialog（spec §5）→ Task 3 Step 6（シグネチャ）＋ Task 5（候補/メタデータ/persist）
  - Filter（spec §6）→ Task 6
  - 配線（spec §7）→ Task 3 Step 3-6 ＋ Task 6 Step 3
  - テスト（spec §8）→ 各タスク内＋ Task 7
  - スコープ・後方互換 → 設計どおり、各タスクで段階的に挙動不変を保ちながら遷移
- **プレースホルダ:** TBD/TODO なし。「Node 同型を雛形にする」箇所には明示的に Node のファイル名と関数を参照した。実装者は Node のコードを読んで Pod 用に書き換える。
- **型整合:** `PodColumnSpec`/`PodLabelColumn`（Task 1）と Task 3 以降の利用一致。`PodColumns::specs()` 名で統一。`pod_widget(tx, label_registry, theme)` / `pod_columns_dialog(tx, default_columns, label_registry, theme)` / `pod_filter_applicator(label_registry, tx)` のシグネチャは Task 3 と Task 6 で一致。`parse_pod_filter(&str, &[PodLabelColumn])` の引数順は applicator のクロージャと一致。
