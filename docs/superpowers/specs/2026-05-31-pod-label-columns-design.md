# Pod label columns（Node ミラー）

- 日付: 2026-05-31
- ステータス: Proposed（提案中）
- 対象範囲: Pod タブの設定・型・poller・列ダイアログ・フィルタに label 列のサポートを追加（Node の label_columns と同型）
- 対象外: Config/Network の column-aware 移行・Pod ログクエリ・Node 側の変更

## 背景・動機

Node は config の `theme.node.label_columns` に `{name, label}` の列定義を書くと、その label の値を列として表示・フィルタ可能になる仕組みを持つ（PR #920 Plan 3、`NodeLabelColumn`／`NodeColumnSpec::Label` で実装）。Pod も同等の機能が欲しい。

Pod の filter migration（PR #992）完了時のフォローアップ候補として明確に挙げていた項目。kubetui の framework（PR #988/#989/#991）と Pod のフィルタ（PR #992）は label 列を受け入れる準備が整っており、Pod 側の型・config・poller・dialog を Node に合わせて拡張すれば自然に乗る。

## 現状確認

- Pod config（`src/config/theme/pod.rs`）には既に `default_preset: Option<String>` と `column_presets: Option<HashMap<String, Vec<PodColumnConfig>>>` が存在する。ただし `PodColumnConfig` は `PodColumn` 厳格型のラッパで、preset 値に label 列名を書けない。`label_columns` フィールドは無い。
- `PodColumn`（`src/features/pod/pod_columns.rs`）は 9 種の builtin enum。Node の `NodeColumnSpec::{Builtin, Label}` のような区別は無い。
- Pod poller（`src/features/pod/kube/pod.rs::get_pods_per_namespace`）は `pod_columns.columns()`（`&[PodColumn]`）を builtin 列名の Vec に変換して `get_resource_per_namespace` に渡し、Table API レスポンスのセルを取り出す。label 値を読む経路は無い。
- 列ダイアログ（`src/features/pod/view/widgets/pod_columns_dialog.rs`）の候補は builtin 9 種をハードコード（`PodColumn::from_str` で逆変換）。
- フィルタ（`src/features/pod/filter/parser.rs`）の既知列は `PodColumn::iter()` のみ。registry を受け取らない。

Node が label 値をどう取得しているかは確認済み: `row.object.metadata.labels[key]` を Table API レスポンスから直接読む（**追加 API コール無し**）。Pod も同じ経路で動く。

## ゴール

1. config で `label_columns: [{ name, label }, ...]` を Pod でも宣言できる。
2. preset 値に label 列名を含められる（`column_presets.gpu = ["name", "mig", "status"]` のように、builtin 名と label 名を混ぜて書ける）。
3. 列ダイアログで builtin＋label 候補をトグルできる。
4. テーブルが label 値を表示する（行ごとの `metadata.labels[key]`、欠落時は空セル）。
5. フィルタ式で label 列を絞り込みに使える（既知列に registry header を含めて検証）。
6. namespace の Z 案内（PR #992）は破壊しない（特別扱いは順序的に最初に評価）。
7. Node と同型の構造で、将来 Config/Network の column-aware 移行時に同じパターンが再利用できる。

## 非ゴール

- Config/Network の column-aware 移行（別途、後続）。
- Pod ログクエリ（`src/features/pod/kube/filter.rs`）の変更。
- label 列に対する独自の表示書式（例: 色付け、表示幅）。label 値はそのまま文字列表示。

## 設計

### 1. Config スキーマ（`src/config/theme/pod.rs`）

`PodThemeConfig` を次のように変更:

- `column_presets: Option<HashMap<String, Vec<PodColumnConfig>>>` → `Option<HashMap<String, Vec<String>>>`。
- `label_columns: Option<Vec<LabelColumnConfig>>` を追加。
- `PodColumnConfig` 型と `serde_pod_column` モジュール、`From<&[PodColumnConfig]> for PodColumns` impl を**削除**（使われなくなる）。
- `LabelColumnConfig` は Node と同形（`{ name: String, label: String }`）。Node のものを共有しても良いが、現状はクレートをまたいでないので Pod 配下に同型を定義しても、Node のを再利用しても可。シンプルさのため**Node の `LabelColumnConfig` を再利用**（モジュール経路: `crate::config::theme::node::LabelColumnConfig`）。

YAML 後方互換: preset 値が `["name","ready"]` のような文字列リストなら現行 deserialize と同じ表面で通る。`PodColumnConfig` 経由の解析が `Vec<String>` の deserialize に置き換わる。invalid な名前のエラーは「deserialize 時」から「app.rs 解決時」に移るが、最終的にエラーは出る。

### 2. 型（`src/features/pod/pod_columns.rs`）— Node ミラー

```rust
pub enum PodColumnSpec {
    Builtin(PodColumn),
    Label { key: String, header: String },
}

impl PodColumnSpec {
    pub fn header(&self) -> String {
        match self {
            PodColumnSpec::Builtin(c) => c.display().to_string(),
            PodColumnSpec::Label { header, .. } => header.clone(),
        }
    }
}

pub struct PodLabelColumn {
    pub name: String,
    pub key: String,
    pub header: String,
}

pub struct PodColumns {
    columns: Vec<PodColumnSpec>,    // 内部型変更
}

impl PodColumns {
    pub fn new(specs: impl IntoIterator<Item = PodColumnSpec>) -> Self;
    pub fn from_builtins(columns: impl IntoIterator<Item = PodColumn>) -> Self;  // 既存のデフォルト経路
    pub fn full() -> Self;            // 9 builtin を全部
    pub fn specs(&self) -> &[PodColumnSpec];   // (旧 columns() を改名)
    pub fn ensure_name_column(self) -> Self;
    pub fn dedup_columns(self) -> Self;
}

pub const DEFAULT_POD_COLUMNS: &[PodColumn] = &[Name, Ready, Status, Age];

impl Default for PodColumns {
    fn default() -> Self { Self::from_builtins(DEFAULT_POD_COLUMNS.iter().copied()) }
}
```

Node の `NodeColumns`/`NodeColumnSpec`/`NodeLabelColumn` と命名・形状を揃える。

### 3. 解決ロジック（`src/app.rs`）

Node の `build_node_label_registry` / `resolve_columns`（Node 版）と同形を追加:

```rust
fn build_pod_label_registry(label_columns: &Option<Vec<LabelColumnConfig>>) -> Result<Vec<PodLabelColumn>>
fn resolve_pod_columns(names: &[String], registry: &[PodLabelColumn]) -> Result<PodColumns>
```

- `build_pod_label_registry`: 各エントリの `header = name.to_uppercase()`、`key = label`、`name = name`。builtin との衝突は `PodColumn::normalize_column(&name)` を `PodColumn::from_str` してみて成功すれば衝突エラー。
- `resolve_pod_columns`: 各 name を `PodColumn::normalize_column` で正規化し、まず builtin として解決を試み、失敗したら label registry から `name` 一致で探す。"full" 単独なら `PodColumn` 全種に展開。未定義名はエラー。最終的に `ensure_name_column` ＋ `dedup_columns`。

config 読み込み箇所で `build_pod_label_registry(&config.theme.pod.label_columns)?` を呼び、`Vec<PodLabelColumn>` を `pod_label_registry` として保持。`default_preset` が指定されていれば `resolve_pod_columns(preset_names, &pod_label_registry)?` で初期 `PodColumns` を作成。

### 4. Poller（`src/features/pod/kube/pod.rs`）

`get_pods_per_namespace` のセル構築を spec 駆動に変更:

- 関数の `pod_columns: &PodColumns` から `specs = pod_columns.specs()` を取り、`builtin_targets: Vec<&str>` は `Builtin` の `c.as_str()` のみ集める。
- `get_resource_per_namespace` には builtin 列名のみを渡し、Table API レスポンスのインデックスを得る（既存と同じ）。
- 行ごとのクロージャ内で `specs` を順に走査し、`Builtin` は `row.cells[builtin_indexes[i]]`、`Label { key, .. }` は `row.object.as_ref().and_then(|o| o.0.get("metadata")).and_then(|m| m.get("labels")).and_then(|l| l.get(key)).and_then(|v| v.as_str()).unwrap_or("").to_string()`（Node `get_node_table:125-133` と同パターン）。
- NAMESPACE 列の動的挿入（multi-ns）、`pod_highlight_rules` の status 着色、`KubeTableRow` 構築は不変。
- `name_index` / `status_index` の決定は spec ベースに（`matches!(s, PodColumnSpec::Builtin(PodColumn::Name))` 等）。

`get_pod_info` の `display_columns` 構築も `specs().iter().map(|s| s.header()).collect()` に変更（builtin/label 同一 API）。

### 5. 列ダイアログ（`src/features/pod/view/widgets/pod_columns_dialog.rs`）

Node の `node_columns_dialog` を雛形に:

- `pod_columns_dialog(tx, default_columns, label_registry, theme)` の引数に `label_registry: Vec<PodLabelColumn>` を追加。
- `candidate_specs(label_registry: &[PodLabelColumn]) -> Vec<PodColumnSpec>` で builtin 9 種＋registry の `Label` エントリを返す。
- チェックリスト項目は各 spec の `header()` を表示用、metadata に `key`（Label の場合）を保持する形にする（保存時の逆引きのため）。Node の dialog の構造に倣う。
- 選択結果を `PodColumns` に構築して `PodMessage::Request(PodColumns)` で送る。

### 6. フィルタ（`src/features/pod/filter.rs` / `filter/parser.rs`）

```rust
pub fn pod_filter_applicator(
    label_registry: Vec<PodLabelColumn>,
    tx: Sender<Message>,
) -> TableFilterApplicator { ... }

pub fn parse_pod_filter(
    input: &str,
    label_registry: &[PodLabelColumn],
) -> Result<TableFilterPredicate, String> {
    let valid: HashSet<String> = PodColumn::iter()
        .map(|c| normalize_column_name(c.display()))
        .chain(label_registry.iter().map(|lc| normalize_column_name(&lc.header)))
        .collect();
    parse_table_filter(input, |column| {
        let normalized = normalize_column_name(column);
        if normalized == "namespace" {
            return Err("namespace is selected via the namespace selector, not the filter".to_string());
        }
        if valid.contains(&normalized) {
            Ok(())
        } else {
            Err(format!("unknown column '{}'", column))
        }
    })
}
```

namespace の Z 案内は順序的に**最初**に評価（registry に "namespace" 由来の header が無くても先に弾く）。`pod_filter_applicator` から `parse_pod_filter` を呼ぶクロージャは registry を capture する（Node 同様）。

### 7. 配線

- `PodTab::new(...)` に `label_registry: Vec<PodLabelColumn>` を追加。`pod_widget` と `pod_columns_dialog` に `.clone()` で渡す（Node の `NodeTab::new` 同形）。
- `pod_widget(tx, label_registry, theme)` に変更。`.filter_applicator(pod_filter_applicator(label_registry, tx.clone()))` で applicator に渡す。
- app.rs から `PodTab::new(..., label_registry, ...)` で `build_pod_label_registry` 結果を渡す。

### 8. テスト

- `theme/pod.rs`: 新スキーマ（`label_columns` + `column_presets: Vec<String>`）のシリアライズ／デシリアライズ、デフォルト、preset に label 名を含むケース。
- `app.rs`: `build_pod_label_registry`（衝突エラー含む）、`resolve_pod_columns`（builtin・label・混合・"full"・未定義名エラー・ensure_name・dedup）。
- `pod_columns.rs`: `PodColumns::from_builtins` / `specs()` / `ensure_name_column` / `dedup_columns`、`PodColumnSpec::header()`。
- Poller: builtin のみのケース（既存挙動回帰）、label 列を含むケース（`row.object.metadata.labels` 経由で値が取れる、欠落時は空文字）。mock の Table response に `object` を含める。
- 列ダイアログ: `candidate_specs(registry)` が builtin＋label を返す、選択結果が PodColumns に正しく構築される。
- `parse_pod_filter`: registry を渡したとき label 名が known に入る、namespace 特例は registry の有無に関わらず最初に発火。
- `pod_filter_applicator`: 構築テスト（registry 引数追加）。
- `cargo test --all`／`cargo clippy --all-targets`／`cargo +nightly fmt --check`。

### 9. スコープ・分割

1 PR で実装（types/config/解決/poller/dialog/filter は密結合で、分割すると中間状態が無価値）。Node の PR #920 Plan 3（Node label columns 追加）と同等の単位。

## 影響を受けるファイル

- `src/config/theme/pod.rs` — スキーマ変更、`PodColumnConfig` 関連削除。
- `src/features/pod/pod_columns.rs` — `PodLabelColumn`／`PodColumnSpec` 追加、`PodColumns` 内部型変更、API 改名（`columns()` → `specs()`）。
- `src/features/pod.rs` — 再エクスポートの追加（`PodLabelColumn`・`PodColumnSpec`）。
- `src/app.rs` — `build_pod_label_registry`・`resolve_pod_columns`、`PodTab::new` への配線。
- `src/features/pod/kube/pod.rs` — poller の spec 駆動セル構築、label 値読み出し。
- `src/features/pod/view/widgets/pod_columns_dialog.rs` — registry 引数追加、`candidate_specs`、保存ロジック。
- `src/features/pod/view/widgets/pod.rs` — `pod_widget(tx, label_registry, theme)`、`pod_filter_applicator(label_registry, tx.clone())`。
- `src/features/pod/view/tab.rs` — `PodTab::new` に `label_registry` 追加。
- `src/features/pod/filter.rs` — `pod_filter_applicator(label_registry, tx)`。
- `src/features/pod/filter/parser.rs` — `parse_pod_filter(input, label_registry)`、既知列に registry header を含める。
- `src/workers/render/window.rs` — 必要なら（registry は PodTab を経由するので window.rs は変わらない見込み）。

## リスク / 後方互換

- **YAML 互換**: `column_presets` 値が文字列リストである限り既存 config はそのまま通る。新フィールド `label_columns` は省略可。
- **エラー位置の移動**: 不正な preset 名は deserialize 時 → app.rs の `resolve_pod_columns` 時に移る。最終的にエラーは出るが、ユーザー視点ではエラーメッセージの出元が変わる。
- **`PodMessage::Request(PodColumns)`**: payload の内部構造（`Vec<PodColumn>` → `Vec<PodColumnSpec>`）が変わるが、外向きの enum variant は不変。
- **API 改名**: `PodColumns::columns()` を `PodColumns::specs()` に。呼び出し元（poller/dialog/test 等）の更新が必要だが範囲は限定的。
- **Pod log query／Pod 一覧表示の他機能**: 触らない（独立）。

## フォローアップ（本 spec の範囲外）

- Config/Network の column-aware 移行と label 列対応（共通 `parse_table_filter`/registry パターンを再利用）。
- `substring_applicator` のキー正規化（Pod/Config/Network が全て column-aware に揃えば自然消滅）。
- `?labelSelector=` の URL エンコード（Node・Pod 共通の hardening）。
