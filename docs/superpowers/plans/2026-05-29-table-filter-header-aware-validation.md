# Table フィルタ header ベース列検証 実装計画

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 共有 Table フィルタの列検証を live header に対して行い、列名を正規化（空白・`-`・`_` 除去）することで、複数語の列をフィルタ可能にし（課題 I）、非表示列の指定を「全行が消える」から明示的な parse error に変える（課題 II）。

**Architecture:** `TableFilterParser` コールバックに現在の表示 header を渡すようシグネチャを拡張し、共有 `normalize_column_name` ヘルパーで `cell_of`（マッチ側）と Node パーサ（検証側）の列名比較を正規化で統一する。Node パーサは builtin enum / label registry のスナップショットではなく live header に対して検証するようになり、`node_filter_applicator` の `label_registry` 引数は不要になって削除される。`substring_applicator`（Pod/Config/Network）は header を無視するだけで挙動不変。Pod の column-aware 移行は本計画の対象外（Phase B）。

**Tech Stack:** Rust 2021、nom（パーサ）、regex、ratatui、strum、`#[cfg(test)]` インラインユニットテスト、pretty_assertions。

参照 spec: `docs/superpowers/specs/2026-05-29-table-filter-header-aware-validation-design.md`

---

## ファイル構成

修正対象（新規ファイルは無し）:

- `src/ui/widget/table/filter_applicator.rs` — `normalize_column_name` ヘルパー追加、`cell_of` を正規化比較に変更、`TableFilterParser` のシグネチャ変更、`substring_applicator` のクロージャ更新、ユニットテスト追加。
- `src/ui/widget/table.rs` — `run_parser_and_update_state` でパーサに `&header` を渡す。モジュール内テスト用パーサのクロージャ更新。
- `src/features/node/filter.rs` — `node_filter_applicator` から `label_registry` 引数を削除、クロージャが header を受け取り `parse_node_filter(input, header)` を呼ぶ。`NodeLabelColumn` import 削除。テスト更新。
- `src/features/node/filter/parser.rs` — `parse_node_filter` が `header: &[String]` を受け取る、`valid_columns` を header ベースに変更、列キーを正規化、エラー文言変更、空 header で検証スキップ。import 整理。テスト群を header ベースへ移行＋新規テスト追加。
- `src/features/node/view/widgets/node.rs` — `node_widget` から `label_registry` 引数を削除、`node_filter_applicator(tx.clone())` 呼び出しに変更。`NodeLabelColumn` import 削除。
- `src/features/node/view/tab.rs` — `node_widget` 呼び出しから `label_registry.clone()` を外す（`label_registry` は引き続き `node_columns_dialog` で使うため `NodeTab::new` の引数は残す）。
- `src/features/node/view/widgets/node_filter_help.rs` — ヘルプ文言を「表示中の列だけがフィルタ可能」へ更新。

---

## Task 1: `normalize_column_name` ヘルパーと `cell_of` の正規化

**Files:**
- Modify: `src/ui/widget/table/filter_applicator.rs:80-89`（`cell_of`）, 新規ヘルパー追加, テストモジュール `src/ui/widget/table/filter_applicator.rs:244-`

- [ ] **Step 1: 失敗するテストを書く**

`src/ui/widget/table/filter_applicator.rs` のテストモジュール（`mod tests { use super::*; ... }`、`make_item` ヘルパーがある箇所）に以下を追加する。

```rust
    #[test]
    fn normalize_column_name_strips_space_hyphen_underscore_and_lowercases() {
        assert_eq!(normalize_column_name("NOMINATED NODE"), "nominatednode");
        assert_eq!(normalize_column_name("Internal-IP"), "internalip");
        assert_eq!(normalize_column_name("Readiness_Gates"), "readinessgates");
        assert_eq!(normalize_column_name("name"), "name");
    }

    #[test]
    fn matches_resolves_multiword_column_via_normalized_key() {
        let header = vec!["NAME".to_string(), "NOMINATED NODE".to_string()];
        let item = make_item(&["pod-a", "node-x"]);

        let mut includes = HashMap::new();
        includes.insert(
            "nominatednode".to_string(),
            vec![Regex::new("node-x").unwrap()],
        );
        let pred = TableFilterPredicate {
            column_includes: includes,
            ..Default::default()
        };

        assert!(pred.matches(&item, &header));
    }

    #[test]
    fn matches_resolves_hyphenated_header_from_compact_key() {
        let header = vec!["NAME".to_string(), "INTERNAL-IP".to_string()];
        let item = make_item(&["pod-a", "10.0.0.1"]);

        let mut includes = HashMap::new();
        includes.insert(
            "internalip".to_string(),
            vec![Regex::new(r"10\.0").unwrap()],
        );
        let pred = TableFilterPredicate {
            column_includes: includes,
            ..Default::default()
        };

        assert!(pred.matches(&item, &header));
    }
```

注: テストモジュール冒頭は `use super::*;` で、親モジュールは `HashMap`（`std::collections`）と `Regex`（`regex`）を import 済み。`make_item(&[...])` は既存ヘルパー（`TableItem` を作る）。

- [ ] **Step 2: テストを実行して失敗を確認**

Run: `cargo test --lib ui::widget::table::filter_applicator 2>&1 | tail -30`
Expected: コンパイルエラー `cannot find function 'normalize_column_name'`（まだ未定義のため）。

- [ ] **Step 3: `normalize_column_name` を実装し `cell_of` を更新**

`src/ui/widget/table/filter_applicator.rs` の `cell_of` 関数（現状 80-89 行）を以下に置き換える。

```rust
/// Normalize a column name for case/format-insensitive comparison: lowercase,
/// with spaces, hyphens, and underscores removed. This lets `nominatednode`,
/// `nominated-node`, and `Nominated_Node` all match the `NOMINATED NODE`
/// header, and keeps hyphenated headers (e.g. `INTERNAL-IP`) matchable from a
/// single whitespace-delimited token.
pub fn normalize_column_name(s: &str) -> String {
    s.to_lowercase().replace([' ', '-', '_'], "")
}

/// Returns the ANSI-stripped text of the column named `col_name` in `item`,
/// or `None` if no header column normalizes to the same key.
fn cell_of(item: &TableItem, header: &[String], col_name: &str) -> Option<String> {
    let key = normalize_column_name(col_name);
    let idx = header
        .iter()
        .position(|h| normalize_column_name(h) == key)?;

    item.item
        .get(idx)
        .map(|c| c.styled_graphemes_symbols().concat())
}
```

（既存の `cell_of` 上にあった `// TODO(perf): ...` コメントはそのまま残してよい。`pub fn normalize_column_name` は後続 Task で Node パーサからも使う。）

- [ ] **Step 4: テストを実行して成功を確認**

Run: `cargo test --lib ui::widget::table::filter_applicator 2>&1 | tail -30`
Expected: PASS（新規3テスト＋既存テストすべて green）。

- [ ] **Step 5: コミット**

```bash
git add src/ui/widget/table/filter_applicator.rs
git commit -m "feat(table-filter): normalize column names (space/-/_) in cell_of matching"
```

---

## Task 2: `TableFilterParser` に header を渡す（挙動不変のプラミング）

このタスクはコールバックのシグネチャ変更で、Rust のコンパイル単位として原子的（全クロージャを同時に更新しないとビルドできない）。挙動は変えない（Node は引き続き `label_registry` で検証、header は無視）。

**Files:**
- Modify: `src/ui/widget/table/filter_applicator.rs`（`define_callback!` 行, `substring_applicator`）
- Modify: `src/ui/widget/table.rs`（`run_parser_and_update_state`, テスト用パーサ）
- Modify: `src/features/node/filter.rs`（クロージャに `_header` 引数追加）

- [ ] **Step 1: `TableFilterParser` のシグネチャを変更**

`src/ui/widget/table/filter_applicator.rs` の define_callback 行（現状 102 行付近）:

```rust
define_callback!(pub TableFilterParser, Fn(&str) -> Result<TableFilterPredicate, String>);
```

を次に変更:

```rust
define_callback!(pub TableFilterParser, Fn(&str, &[String]) -> Result<TableFilterPredicate, String>);
```

- [ ] **Step 2: `substring_applicator` のクロージャを更新**

同ファイルの `substring_applicator`（現状 216-242 行）のクロージャ先頭:

```rust
    let parser: TableFilterParser = (move |input: &str| {
```

を次に変更（header を受け取って無視する）:

```rust
    let parser: TableFilterParser = (move |input: &str, _header: &[String]| {
```

（クロージャ本体・`ApplyStrategy::Live` はそのまま。）

- [ ] **Step 3: `run_parser_and_update_state` で header を渡す**

`src/ui/widget/table.rs` の `run_parser_and_update_state`（現状 831-850 行）を次に置き換える。

```rust
    fn run_parser_and_update_state(&mut self) -> Option<TableFilterPredicate> {
        let header = self.items.header().original().to_vec();
        let applicator = self.filter_applicator.as_ref()?;
        let input = self
            .filter_form
            .as_ref()
            .map(|f| f.content())
            .unwrap_or_default();

        match (applicator.parser.closure)(&input, &header) {
            Ok(predicate) => {
                self.filter_error = None;
                self.filter_state = Some(predicate.clone());
                Some(predicate)
            }
            Err(msg) => {
                self.filter_error = Some(msg);
                None
            }
        }
    }
```

（`self.items.header().original().to_vec()` は同ファイルの `filter_items` 等で既に使われている既存 API。header を先に owned で取得してから `applicator` を借用することで借用衝突を避ける。）

- [ ] **Step 4: table.rs のテスト用パーサを更新**

`src/ui/widget/table.rs` のテスト内（`filter_cancel_returns_some_callback_when_applicator_has_on_cancel`、現状 1120 行付近）:

```rust
                TableFilterParser::from(move |_: &str| {
                    Ok(crate::ui::widget::TableFilterPredicate::default())
                }),
```

を次に変更:

```rust
                TableFilterParser::from(move |_: &str, _: &[String]| {
                    Ok(crate::ui::widget::TableFilterPredicate::default())
                }),
```

- [ ] **Step 5: Node のクロージャを更新（挙動不変）**

`src/features/node/filter.rs` の `node_filter_applicator` 内のパーサ構築（現状 47-48 行）:

```rust
    let parser: TableFilterParser =
        (move |input: &str| parse_node_filter(input, &label_registry)).into();
```

を次に変更（header 引数を受け取るが、この Task ではまだ無視して従来どおり `label_registry` で検証）:

```rust
    let parser: TableFilterParser =
        (move |input: &str, _header: &[String]| parse_node_filter(input, &label_registry)).into();
```

- [ ] **Step 6: ビルドと全テストを実行して green を確認**

Run: `cargo test --all 2>&1 | tail -30`
Expected: コンパイル成功、全テスト PASS（挙動不変のため既存テストはそのまま通る）。

- [ ] **Step 7: コミット**

```bash
git add src/ui/widget/table/filter_applicator.rs src/ui/widget/table.rs src/features/node/filter.rs
git commit -m "refactor(table-filter): thread live header through TableFilterParser"
```

---

## Task 3: Node パーサを header ベース検証へ＋`label_registry` 削除

課題 I/II の本体。Node パーサが live header に対して列を検証し、列キーを正規化し、非表示/未知列を「not in the current view」エラーにする。`label_registry` 引数は不要になり、`node_filter_applicator` → `node_widget` → `tab.rs` のカスケードで削除する（`tab.rs` はダイアログ用に保持して止まる）。

**Files:**
- Modify: `src/features/node/filter/parser.rs`（`parse_node_filter`, `valid_columns`, imports, テスト群）
- Modify: `src/features/node/filter.rs`（`node_filter_applicator` から `label_registry` 削除）
- Modify: `src/features/node/view/widgets/node.rs`（`node_widget` から `label_registry` 削除）
- Modify: `src/features/node/view/tab.rs`（`node_widget` 呼び出し更新）

- [ ] **Step 1: parser.rs のテストを header ベースへ移行＋新規テスト追加**

`src/features/node/filter/parser.rs` のテストモジュール（`mod tests`）で以下を行う。

(a) ヘルパー `no_label_cols`（現状 322-324 行）と `registry_with`（現状 466-472 行）を削除し、代わりに次の `header` ヘルパーを追加する。

```rust
    fn header() -> Vec<String> {
        [
            "NAME",
            "STATUS",
            "ROLES",
            "AGE",
            "VERSION",
            "INTERNAL-IP",
            "EXTERNAL-IP",
            "OS-IMAGE",
            "KERNEL-VERSION",
            "CONTAINER-RUNTIME",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    }
```

(b) テストモジュール内の `parse_node_filter(..., &no_label_cols())` の呼び出しをすべて `parse_node_filter(..., &header())` に置換する（quoting/escape テストを含む全箇所）。

(c) `registered_label_column_header_is_accepted` テスト（現状 500-505 行）を次に置き換える。

```rust
    #[test]
    fn header_column_is_accepted() {
        let mut h = header();
        h.push("ZONE".to_string());
        let p = parse_node_filter("zone:us-west", &h).unwrap();
        assert!(p.column_includes.contains_key("zone"));
    }
```

(d) 2つのエラーテストのアサーションを新エラー文言に合わせる。

`unknown_column_produces_parse_error`（現状 474-482 行）:

```rust
    #[test]
    fn unknown_column_produces_parse_error() {
        let err = parse_node_filter("statusu:Ready", &header()).unwrap_err();
        assert!(
            err.contains("not in the current view") && err.contains("statusu"),
            "error should explain the column is not shown: {}",
            err
        );
    }
```

`unknown_column_in_exclude_also_errors`（現状 484-492 行）:

```rust
    #[test]
    fn unknown_column_in_exclude_also_errors() {
        let err = parse_node_filter("!agee:1h", &header()).unwrap_err();
        assert!(
            err.contains("not in the current view") && err.contains("agee"),
            "error should explain the column is not shown: {}",
            err
        );
    }
```

(e) 新規テストを3つ追加する（課題 I/II ＋空 header）。

```rust
    #[test]
    fn multiword_column_with_space_is_filterable_via_compact_token() {
        // 課題 I: 空白入りの列名（例 NOMINATED NODE）が圧縮トークンで指定でき、
        // 正規化キーで格納される。
        let h = vec!["NAME".to_string(), "NOMINATED NODE".to_string()];
        let p = parse_node_filter("nominatednode:foo", &h).unwrap();
        assert!(p.column_includes.contains_key("nominatednode"));
    }

    #[test]
    fn column_not_in_header_produces_not_in_view_error() {
        // 課題 II: VERSION は実在する builtin 列名だが header に無ければエラー。
        let h = vec!["NAME".to_string(), "STATUS".to_string()];
        let err = parse_node_filter("version:1.2", &h).unwrap_err();
        assert!(
            err.contains("not in the current view") && err.contains("version"),
            "error should explain the column is not shown: {}",
            err
        );
    }

    #[test]
    fn empty_header_skips_column_validation() {
        // 最初のポーリング前は header が空になり得る。その間は検証しない。
        let p = parse_node_filter("status:Ready", &[]).unwrap();
        assert!(p.column_includes.contains_key("status"));
    }
```

- [ ] **Step 2: テストを実行して失敗（コンパイルエラー）を確認**

Run: `cargo test --lib features::node::filter 2>&1 | tail -30`
Expected: コンパイルエラー（`parse_node_filter` のシグネチャが旧来 `&[NodeLabelColumn]` のままで、`&header()`（`&Vec<String>`）/`&[]` を渡せない、また削除した `no_label_cols`/`registry_with` 参照の不整合）。

- [ ] **Step 3: `parse_node_filter` と `valid_columns` を header ベースに実装**

`src/features/node/filter/parser.rs` の import（現状 18-23 行）を次に整理する。

```rust
use regex::Regex;

use crate::ui::widget::{normalize_column_name, TableFilterPredicate};
```

（`use strum::IntoEnumIterator;` と `use crate::features::node::node_columns::{NodeColumn, NodeLabelColumn};` を削除。nom 系の import（1-16 行）はそのまま。`std::collections::{HashMap, HashSet}` と `std::borrow::Cow` はそのまま使う。）

`valid_columns`（現状 217-225 行）を次に置き換える。

```rust
/// Build the set of valid column names from the current table header,
/// normalized so matching is case/format-insensitive.
fn valid_columns(header: &[String]) -> HashSet<String> {
    header.iter().map(|h| normalize_column_name(h)).collect()
}
```

`parse_node_filter`（現状 241-315 行、`#[allow(dead_code)]` 含む）を次に置き換える。

```rust
/// Parse a Node-filter input string into a `TableFilterPredicate`.
///
/// `header` is the table's current display header. Column references are
/// validated (case/format-insensitive) against it; a column not in the current
/// view produces a parse error. When `header` is empty (e.g. before the first
/// poll populates the table) column validation is skipped.
///
/// Values may be quoted (`"..."` / `'...'`) with the same escape rules as the
/// Pod log query parser: `\"` → `"`  `\'` → `'`  `\\` → `\`  `\<other>` verbatim.
pub fn parse_node_filter(
    input: &str,
    header: &[String],
) -> Result<TableFilterPredicate, String> {
    let valid = valid_columns(header);
    let validate = !header.is_empty();

    let trimmed = input.trim();
    let mut column_includes: HashMap<String, Vec<Regex>> = HashMap::new();
    let mut column_excludes: HashMap<String, Vec<Regex>> = HashMap::new();
    let mut label_selector: Option<String> = None;

    if trimmed.is_empty() {
        return Ok(TableFilterPredicate {
            column_includes,
            column_excludes,
            label_selector,
            raw: trimmed.to_string(),
        });
    }

    // Parse the whole trimmed input as whitespace-separated tokens.
    type E<'a> = nom::error::Error<&'a str>;
    let parse_result = delimited(
        multispace0,
        separated_list0(multispace1, parse_token::<E>),
        multispace0,
    )
    .parse(trimmed);

    let (remaining, terms) = parse_result.map_err(|e| format!("parse error: {}", e))?;

    if !remaining.is_empty() {
        return Err(format!("unexpected input near: {:?}", remaining));
    }

    for term in terms {
        match term {
            Term::Bare(v) => {
                let rx = Regex::new(&v).map_err(|e| format!("invalid regex '{}': {}", v, e))?;
                column_includes
                    .entry("name".to_string())
                    .or_default()
                    .push(rx);
            }
            Term::Include { column, value } => {
                let col = normalize_column_name(&column);
                if validate && !valid.contains(&col) {
                    return Err(format!("column '{}' is not in the current view", column));
                }
                let rx =
                    Regex::new(&value).map_err(|e| format!("invalid regex '{}': {}", value, e))?;
                column_includes.entry(col).or_default().push(rx);
            }
            Term::Exclude { column, value } => {
                let col = normalize_column_name(&column);
                if validate && !valid.contains(&col) {
                    return Err(format!("column '{}' is not in the current view", column));
                }
                let rx =
                    Regex::new(&value).map_err(|e| format!("invalid regex '{}': {}", value, e))?;
                column_excludes.entry(col).or_default().push(rx);
            }
            Term::Label(sel) => {
                // Last label: term wins (k8s API accepts only one labelSelector value).
                label_selector = Some(sel);
            }
        }
    }

    Ok(TableFilterPredicate {
        column_includes,
        column_excludes,
        label_selector,
        raw: trimmed.to_string(),
    })
}
```

（`parse_token` / `Term` / quoting ヘルパー群はそのまま。`#[allow(dead_code)]` は不要なので削除する — `node_filter_applicator` から実際に使われる。）

- [ ] **Step 4: `node_filter_applicator` から `label_registry` を削除**

`src/features/node/filter.rs` を更新する。import から `NodeLabelColumn` を削除する（現状 17 行の `node::{message::NodeMessage, node_columns::NodeLabelColumn}` を `node::message::NodeMessage` に）。

関数シグネチャとパーサ構築（現状 43-48 行）を次に置き換える。

```rust
pub fn node_filter_applicator(tx: Sender<Message>) -> TableFilterApplicator {
    let parser: TableFilterParser =
        (move |input: &str, header: &[String]| parse_node_filter(input, header)).into();
```

（`tx_apply` / `tx_cancel` / `on_apply` / `on_cancel` / `TableFilterApplicator::new(...)` 以降はそのまま。）

同ファイルのテスト（現状 78-82 行付近）を次に更新する。

```rust
    #[test]
    fn applicator_constructs_without_panic() {
        let (tx, _rx) = crossbeam::channel::bounded(1);
        let _ = node_filter_applicator(tx);
    }
```

- [ ] **Step 5: `node_widget` から `label_registry` を削除**

`src/features/node/view/widgets/node.rs` を更新する。import から `NodeLabelColumn` を削除する（現状 7-11 行の `node::{ filter::node_filter_applicator, message::NodeDetailMessage, node_columns::NodeLabelColumn }` を `node::{ filter::node_filter_applicator, message::NodeDetailMessage }` に）。

関数シグネチャ（現状 31-35 行）と filter 配線（現状 51 行）を更新する。

```rust
pub fn node_widget(tx: Sender<Message>, theme: WidgetThemeConfig) -> Widget<'static> {
```

```rust
        .filter_applicator(node_filter_applicator(tx.clone()))
```

- [ ] **Step 6: `tab.rs` の `node_widget` 呼び出しを更新**

`src/features/node/view/tab.rs:44` を次に変更する（`label_registry` は 47 行の `node_columns_dialog` で引き続き使うため `NodeTab::new` の引数・import はそのまま）。

```rust
        let node_widget = node_widget(tx.clone(), theme.clone());
```

- [ ] **Step 7: テストを実行して green を確認**

Run: `cargo test --all 2>&1 | tail -40`
Expected: コンパイル成功、全テスト PASS（移行・新規した Node parser テストを含む）。

- [ ] **Step 8: コミット**

```bash
git add src/features/node/filter/parser.rs src/features/node/filter.rs src/features/node/view/widgets/node.rs src/features/node/view/tab.rs
git commit -m "feat(node-filter): validate columns against live header; drop label_registry"
```

---

## Task 4: Node フィルタヘルプ文言の更新

**Files:**
- Modify: `src/features/node/view/widgets/node_filter_help.rs:38-79`（`content()`）

- [ ] **Step 1: ヘルプ末尾の説明を更新**

`src/features/node/view/widgets/node_filter_help.rs` の `content()` 内、末尾の段落（現状 72-74 行）:

```
        Column names are case-insensitive. Unknown columns produce a
        parse error. Press Enter to apply, Esc to cancel. Type ? or
        help in the filter input to open this help.
```

を次に置き換える（空白・`-`・`_` が無視されること、フィルタ可能なのは表示中の列だけであることを明示）。

```
        Filterable columns are the ones currently shown in the table.
        Column names ignore case, spaces, '-' and '_'. A column not in
        the current view produces an error (add it via the columns
        dialog). Press Enter to apply, Esc to cancel. Type ? or help in
        the filter input to open this help.
```

- [ ] **Step 2: ビルドを確認**

Run: `cargo build 2>&1 | tail -15`
Expected: コンパイル成功（文字列リテラルのみの変更）。

- [ ] **Step 3: コミット**

```bash
git add src/features/node/view/widgets/node_filter_help.rs
git commit -m "docs(node-filter): help text reflects header-based filterable columns"
```

---

## Task 5: 全体検証（テスト / lint / format / 手動スモーク）

**Files:** なし（検証のみ）

- [ ] **Step 1: 全テスト**

Run: `cargo test --all 2>&1 | tail -20`
Expected: 全テスト PASS。

- [ ] **Step 2: clippy**

Run: `cargo clippy --all-targets 2>&1 | tail -30`
Expected: 警告・エラー無し（特に未使用 import / 未使用変数が残っていないこと。`NodeLabelColumn`/`NodeColumn`/`strum::IntoEnumIterator` を消し残していないか確認）。

- [ ] **Step 3: format**

Run: `cargo +nightly fmt --check 2>&1 | tail -20`
Expected: 差分無し。差分が出たら `cargo +nightly fmt` を実行して再コミット。

- [ ] **Step 4: 手動スモーク（実クラスタまたは KIND）**

実行できる環境があれば `cargo run` で起動し Node タブで以下を確認する（TUI のため自動テスト不可、不可なら省略する旨を明記）。

1. デフォルト列で `status:Ready` → STATUS が Ready の行だけ残る。
2. デフォルト列（INTERNAL-IP 非表示）で `internalip:10.` → 「not in the current view」エラーがテーブル本体に表示され、全行が消えない（課題 II）。
3. 列ダイアログ（`t`）で INTERNAL-IP を表示に追加 → 再度 `internalip:10.` がエラーにならずフィルタされる。
4. `internal-ip:10.` と `internalip:10.`（ハイフン有無）が同じ結果になる（課題 I・正規化）。
5. `label:role=worker` がサーバーサイドで効く（既存挙動の非回帰）。
6. `?` でヘルプダイアログが開き、更新後の文言が表示される。

- [ ] **Step 5: （必要なら）format 修正をコミット**

```bash
git add -A
git commit -m "style: cargo fmt"
```

---

## Self-Review（計画作成者によるチェック結果）

- **Spec カバレッジ:** 課題 I（正規化）→ Task 1＋Task 3、課題 II（非表示列→parse error）→ Task 3、header をパーサへ渡す → Task 2、`substring_applicator` 挙動不変 → Task 2 Step 2、`label_registry` 削除 → Task 3 Step 4-6、ヘルプ文言 → Task 4、テスト → 各 Task ＋ Task 5。spec の全項目に対応タスクあり。
- **プレースホルダ:** TBD/TODO 無し（既存コードの `// TODO(perf)` は意図的に保持）。全コードステップに実コードを記載。テストの一括置換（Task 3 Step 1(b)）は機械的で対象文字列が一意（`&no_label_cols()` → `&header()`）。
- **型整合:** `normalize_column_name(&str) -> String`（Task 1 で定義、Task 3 で `crate::ui::widget` から import して使用）一致。`TableFilterParser` のシグネチャ `Fn(&str, &[String]) -> Result<TableFilterPredicate, String>` は Task 2 の全クロージャ・呼び出し（`(closure)(&input, &header)`）と一致。`parse_node_filter(&str, &[String])` は filter.rs クロージャ・テストと一致。`node_filter_applicator(Sender<Message>)`／`node_widget(Sender<Message>, WidgetThemeConfig)` は呼び出し側（node.rs/tab.rs）と一致。
