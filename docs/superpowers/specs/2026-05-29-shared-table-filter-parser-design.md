# 共有 column-aware フィルタパーサコアの抽出（refactor）

- 日付: 2026-05-29
- ステータス: Proposed（提案中）
- 種別: behavior-preserving refactor（ユーザー向け変更なし）
- 対象範囲: 共有 Table widget（`src/ui/widget/table/`）＋ Node フィルタパーサ（`src/features/node/filter/parser.rs`）の載せ替え
- 位置づけ: Pod の column-aware 移行（別 spec, PR B）の**先行 PR（B-0）**。本 PR では Pod は変更しない。

## 背景・動機

column-aware フィルタの構文（`COL:val` / `!COL:val` / `label:sel` / bare→NAME、quoting）を解釈する nom パーサが、現状 Node の `src/features/node/filter/parser.rs` に閉じている。ここには汎用的な部分（トークナイザ・quoting・`Term`・述語組み立て）と Node 固有の部分（既知列＝builtin `NodeColumn` ＋ label registry の検証）が混在している。

Pod を column-aware 化する（PR B）と**2つ目の同型パーサ**が必要になる。実際 Node parser には「Quoting helpers (copied from pod/kube/filter/parser.rs to avoid cross-feature dep)」という注記があり、既に重複が始まっている。Pod でさらに複製すると、将来の Config/Network 移行でも増殖する。

そこで Pod 移行の前に、**汎用部分を共有モジュールへ抽出**し、Node をその上に載せ替える。タブ固有なのは「どの列名を既知とみなすか（列バリデータ）」だけにする。

## ゴール

1. column-aware パーサの汎用コア（トークナイザ・quoting・`Term`・`parse_token`・述語組み立て）を1箇所に集約し、複数タブが**列検証だけ差し替えて**再利用できるようにする。
2. Node をその共有コアに載せ替える。**Node の挙動は不変**（既存パーサテストが変更なしで通ることが証明）。
3. Pod（PR B）/ 将来の Config/Network が同コアを使える土台を用意する。

## 非ゴール

- Pod の column-aware 移行（PR B、別 spec）。本 PR では Pod・Config・Network は一切変更しない。
- 構文・挙動の変更。あくまで抽出のみ。

## 設計

### 1. 共有モジュール `src/ui/widget/table/filter_parser.rs`（新規）

`src/features/node/filter/parser.rs` から以下を**移設**する:
- quoting ヘルパー: `quoted` / `unquoted` / `value_string` / `column_name`（現 parser.rs:33-123）。
- `Term` enum（`Bare` / `Include` / `Exclude` / `Label`、現 131-140）。
- `parse_token`（現 151-208）。

加えて、現 `parse_node_filter` の本体から**列検証を除いた汎用部分**を、次の関数として共有モジュールに置く:

```rust
/// Parse a column-aware filter string into a `TableFilterPredicate`.
///
/// `validate_column` is called with each `COL:`/`!COL:` column token (original
/// casing) and returns `Ok(())` if the column is acceptable, or `Err(message)`
/// to abort parsing with that message — this is the only tab-specific part.
/// Stored predicate keys are normalized via `normalize_column_name`. Bare values
/// map to the `name` include; `label:` is captured verbatim (last wins);
/// values may be quoted with the log-query escape rules.
pub fn parse_table_filter(
    input: &str,
    validate_column: impl Fn(&str) -> Result<(), String>,
) -> Result<TableFilterPredicate, String>
```

挙動（現 `parse_node_filter` と等価）:
- 空入力 → 空 predicate。
- nom でホワイトスペース区切りトークン化（`parse_token`）。残余があれば `unexpected input near` エラー。
- `Term::Bare(v)` → `column_includes["name"]` に regex を push。
- `Term::Include{column,value}` → `validate_column(&column)?`、`normalize_column_name(&column)` をキーに regex を push。regex 不正は `invalid regex` エラー。
- `Term::Exclude{...}` → 同様に `column_excludes` へ。
- `Term::Label(sel)` → `label_selector = Some(sel)`（最後勝ち）。

`TableFilterPredicate` と `normalize_column_name` は同じ `src/ui/widget/table/` 配下（`filter_applicator.rs`）にあり参照可能。

### 2. モジュール配線

`src/ui/widget/table.rs` に `mod filter_parser;` を追加し、`pub use filter_parser::parse_table_filter;`（および `crate::ui::widget` 経由の再エクスポート、`normalize_column_name` と同様）。`Term` / `parse_token` / quoting ヘルパーは共有モジュール内に閉じる（`pub` 不要、必要なら `pub(crate)`）。

### 3. Node パーサの載せ替え（`src/features/node/filter/parser.rs`）

- 移設した quoting/`Term`/`parse_token` を削除。不要になった nom 系 import（`branch`/`bytes`/`character`/`combinator`/`multi`/`sequence` など）と `Cow` を整理。`Regex`/`HashSet`/`strum`/`NodeColumn`/`NodeLabelColumn` は valid_columns で引き続き使用。
- `valid_columns(label_registry: &[NodeLabelColumn]) -> HashSet<String>` は**そのまま残す**（builtin `NodeColumn::iter()` の display 正規化＋registry header 正規化）。
- `parse_node_filter` を薄いラッパに:

```rust
pub fn parse_node_filter(
    input: &str,
    label_registry: &[NodeLabelColumn],
) -> Result<TableFilterPredicate, String> {
    let valid = valid_columns(label_registry);
    parse_table_filter(input, |column| {
        if valid.contains(&normalize_column_name(column)) {
            Ok(())
        } else {
            Err(format!("unknown column '{}'", column))
        }
    })
}
```

- `parse_table_filter` と `normalize_column_name` を `crate::ui::widget` から import。

### 4. テスト

- **Node の既存テストは変更しない**（`parse_node_filter` のシグネチャ・挙動は不変）。これが behavior-preservation の主たる証明。
- 共有モジュールに `parse_table_filter` の単体テストを追加: bare→name、include/exclude、label（最後勝ち）、quoting（`"..."`/`'...'`/escape）、`validate_column` が `Err` を返すと parse error になる、不正 regex エラー、空入力。
- `cargo test --all` / `cargo clippy --all-targets` / `cargo +nightly fmt --check`。

## 影響を受けるファイル

- `src/ui/widget/table/filter_parser.rs` — 新規（quoting/`Term`/`parse_token`/`parse_table_filter`＋テスト）。
- `src/ui/widget/table.rs` — `mod filter_parser;` ＋ `parse_table_filter` の再エクスポート。
- `src/features/node/filter/parser.rs` — 移設分を削除、`parse_node_filter` をラッパ化、import 整理。`valid_columns` は維持。

## リスク / 後方互換

- behavior-preserving。Node の挙動は不変で、既存テストが回帰検知になる。
- 主なリスクは「抽出時のトークナイザ/quoting の取りこぼし」だが、Node テスト（quoting/escape ケース含む）が網羅しているため検出可能。

## フォローアップ（PR B、別 spec）

Pod の column-aware 移行は本共有コアの上に構築する: `parse_pod_filter` = `parse_table_filter(input, |col| PodColumn 既知列検証＋namespace 案内メッセージ)`、`pod_filter_applicator`、`PodMessage::Filter(Option<String>)`＋`SharedPodFilter`＋poller の per-namespace `?labelSelector=`、`POD_FILTER_HELP_DIALOG_ID` help dialog、widget 載せ替え。namespace はフィルタ列にせず（スコープ＝選択軸）、`namespace:` には専用案内を返す。
