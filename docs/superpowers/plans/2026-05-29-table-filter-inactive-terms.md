# Table フィルタ inactive terms 実装計画

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 非表示列を参照するフィルタ項を「全行消失/エラー」ではなく inactive（matching でスキップ・`(inactive: …)` バッジ・列再表示で自動復活）にし、未定義列のみ parse error にする 3-way モデルへ作り替える。

**Architecture:** 共有 `TableFilterPredicate::matches` を「列が header に無ければその制約をスキップ」に変更（inactive の本体）。Node parser は header ではなく「既知列＝builtin＋label registry」に対して検証し、未定義列を `unknown column` エラーにする（Phase A の header 配線 Task 2/3 を既知列検証へ作り替え、`TableFilterParser` を `Fn(&str)` に戻す）。`normalize_column_name`／cell_of 正規化（Phase A Task 1）は維持。`count_indicator()` に inactive バッジを統合。

**Tech Stack:** Rust 2021、nom、regex、ratatui、strum、`#[cfg(test)]` インラインテスト、pretty_assertions。

参照 spec: `docs/superpowers/specs/2026-05-29-table-filter-inactive-terms-design.md`

---

## ファイル構成（修正対象。新規ファイルなし）

- `src/ui/widget/table/filter_applicator.rs` — `matches` を inactive スキップに変更（`cell_of` を `column_index`＋`cell_text` に分割）。`TableFilterParser` を `Fn(&str)` に戻す。`substring_applicator` クロージャを `Fn(&str)` に戻す。`normalize_column_name` は維持。
- `src/ui/widget/table.rs` — `run_parser_and_update_state` のパーサ呼び出しを header なしに戻す。テスト用パーサのシグネチャを戻す。`count_indicator()` に inactive バッジを統合し `inactive_columns()` ヘルパーを追加。
- `src/features/node/filter.rs` — `node_filter_applicator(label_registry, tx)` に戻し、クロージャを `move |input| parse_node_filter(input, &label_registry)` に。`NodeLabelColumn` import 復帰。
- `src/features/node/filter/parser.rs` — `parse_node_filter(input, label_registry)` に戻す。`valid_columns(label_registry)`＝builtin＋registry。未定義列は `unknown column`。import（`strum`／`NodeColumn`／`NodeLabelColumn`）復帰。テスト復帰。
- `src/features/node/view/widgets/node.rs` — `node_widget(tx, label_registry, theme)` に戻す。`NodeLabelColumn` import 復帰。
- `src/features/node/view/tab.rs` — `node_widget` 呼び出しに `label_registry.clone()` を戻す。
- `src/features/node/view/widgets/node_filter_help.rs` — 文言を 3-way に更新。

---

## Task 1: matches() を inactive スキップに変更

**Files:**
- Modify: `src/ui/widget/table/filter_applicator.rs`（`matches` 52-71、`cell_of` 83-98、テスト `mod tests`）

- [ ] **Step 1: 失敗するテストを書く**

`src/ui/widget/table/filter_applicator.rs` のテストモジュール（`make_item` がある `mod tests`）に追加:

```rust
    #[test]
    fn matches_skips_include_when_column_absent_from_visible_columns() {
        // version 列が表示列に無い → version の include はスキップされ行は残る
        let visible_columns = vec!["NAME".to_string(), "STATUS".to_string()];
        let item = make_item(&["gke-a", "Ready"]);

        let mut includes = HashMap::new();
        includes.insert("version".to_string(), vec![Regex::new("1.30").unwrap()]);
        let pred = TableFilterPredicate {
            column_includes: includes,
            ..Default::default()
        };

        assert!(pred.matches(&item, &visible_columns));
    }

    #[test]
    fn matches_skips_exclude_when_column_absent_from_visible_columns() {
        let visible_columns = vec!["NAME".to_string(), "STATUS".to_string()];
        let item = make_item(&["gke-a", "Ready"]);

        let mut excludes = HashMap::new();
        excludes.insert("version".to_string(), vec![Regex::new("1.30").unwrap()]);
        let pred = TableFilterPredicate {
            column_excludes: excludes,
            ..Default::default()
        };

        assert!(pred.matches(&item, &visible_columns));
    }

    #[test]
    fn matches_still_applies_present_columns() {
        // status が表示列にある → 通常どおり効く
        let visible_columns = vec!["NAME".to_string(), "STATUS".to_string()];
        let ready = make_item(&["gke-a", "Ready"]);
        let not_ready = make_item(&["gke-b", "NotReady"]);

        let mut includes = HashMap::new();
        includes.insert("status".to_string(), vec![Regex::new("Ready").unwrap()]);
        let pred = TableFilterPredicate {
            column_includes: includes,
            ..Default::default()
        };

        assert!(pred.matches(&ready, &visible_columns));
        assert!(!pred.matches(&not_ready, &visible_columns));
    }
```

注: `make_item(&[...])` は既存ヘルパー、`HashMap`/`Regex` は親モジュールで import 済み。

- [ ] **Step 2: テストを実行して失敗を確認**

Run: `cargo test --lib ui::widget::table::filter_applicator 2>&1 | tail -30`
Expected: `matches_skips_include_when_column_absent_from_visible_columns` が FAIL（現状は absent 列を空文字扱いし include が全行を弾くため）。

- [ ] **Step 3: matches と補助関数を実装**

`src/ui/widget/table/filter_applicator.rs` の `matches`（現状 52-71）を次に置き換える:

```rust
    /// Returns `true` when `item` passes all active filters.
    ///
    /// A constraint whose column is not present in `visible_columns` (e.g. the
    /// column was hidden via the column dialog) is **inactive**: it is skipped
    /// rather than failing the row, so the remaining visible-column constraints
    /// still apply and rows stay visible.
    pub fn matches(&self, item: &TableItem, visible_columns: &[String]) -> bool {
        // --- column_includes (AND across columns, OR within) ---
        for (column, patterns) in &self.column_includes {
            let Some(idx) = column_index(visible_columns, column) else {
                continue; // inactive: column not among the visible columns
            };
            let cell = cell_text(item, idx);
            if !patterns.iter().any(|r| r.is_match(&cell)) {
                return false;
            }
        }

        // --- column_excludes (AND across columns, OR within → exclude) ---
        for (column, patterns) in &self.column_excludes {
            let Some(idx) = column_index(visible_columns, column) else {
                continue; // inactive: column not among the visible columns
            };
            let cell = cell_text(item, idx);
            if patterns.iter().any(|r| r.is_match(&cell)) {
                return false;
            }
        }

        true
    }
```

同ファイルの `cell_of`（現状 83-98、`// TODO(perf)` コメント含む）を次の2関数に置き換える:

```rust
/// Index of the visible column whose normalized name equals `column_name`'s,
/// or `None` if no such column is currently displayed (the constraint is inactive).
// TODO(perf): column_index() is called per column × per row × per render. Each
// invocation re-normalizes the column names. If profiling shows this in the
// hot path, pre-compute a column-name → index map at filter_state set time.
fn column_index(visible_columns: &[String], column_name: &str) -> Option<usize> {
    let key = normalize_column_name(column_name);
    visible_columns
        .iter()
        .position(|h| normalize_column_name(h) == key)
}

/// ANSI-stripped text of the cell at `idx` in `item` (empty string if missing).
fn cell_text(item: &TableItem, idx: usize) -> String {
    item.item
        .get(idx)
        .map(|c| c.styled_graphemes_symbols().concat())
        .unwrap_or_default()
}
```

（`normalize_column_name` はそのまま。`cell_of` は削除し、上記2関数に置換。他に `cell_of` を呼ぶ箇所は無い（private・matches 専用）。）

- [ ] **Step 4: テストを実行して成功を確認**

Run: `cargo test --lib ui::widget::table::filter_applicator 2>&1 | tail -30`
Expected: PASS（新規3テスト＋既存テスト全て green）。

- [ ] **Step 5: コミット**

```bash
git add src/ui/widget/table/filter_applicator.rs
git commit -m "feat(table-filter): skip (inactive) constraints whose column is not in the header"
```

---

## Task 2: parser を既知列検証へ戻す＋header 配線を撤去

`TableFilterParser` のシグネチャを `Fn(&str)` に戻す原子的変更（全クロージャ同時更新が必要）。Node parser は既知列（builtin＋label registry）で検証し未定義列は `unknown column` エラー、非表示の既知列は受理（inactive は Task 1 の matching 側で表現）。

**Files:**
- Modify: `src/ui/widget/table/filter_applicator.rs`（`TableFilterParser`、`substring_applicator`）
- Modify: `src/ui/widget/table.rs`（`run_parser_and_update_state`、テスト用パーサ）
- Modify: `src/features/node/filter.rs`
- Modify: `src/features/node/filter/parser.rs`
- Modify: `src/features/node/view/widgets/node.rs`
- Modify: `src/features/node/view/tab.rs`

- [ ] **Step 1: parser.rs のテストを既知列ベースへ戻す＋未定義列テスト**

`src/features/node/filter/parser.rs` の `mod tests` で:

(a) `header()` ヘルパー（現状）を削除し、次の2ヘルパーを追加:

```rust
    fn no_label_cols() -> Vec<NodeLabelColumn> {
        Vec::new()
    }

    fn registry_with(name: &str, header: &str) -> Vec<NodeLabelColumn> {
        vec![NodeLabelColumn {
            name: name.to_string(),
            key: "irrelevant.example.com/key".to_string(),
            header: header.to_string(),
        }]
    }
```

(b) モジュール内の `parse_node_filter(<input>, &header())` 呼び出しを全て `parse_node_filter(<input>, &no_label_cols())` に置換。

(c) `header_column_is_accepted` テスト（現状）を次に置換:

```rust
    #[test]
    fn registered_label_column_header_is_accepted() {
        let regs = registry_with("zone", "ZONE");
        let p = parse_node_filter("zone:us-west", &regs).unwrap();
        assert!(p.column_includes.contains_key("zone"));
    }
```

(d) 2つのエラーテストを `unknown column` 期待に戻す:

```rust
    #[test]
    fn unknown_column_produces_parse_error() {
        let err = parse_node_filter("statusu:Ready", &no_label_cols()).unwrap_err();
        assert!(
            err.contains("unknown column") && err.contains("statusu"),
            "error should mention the bad column: {}",
            err
        );
    }

    #[test]
    fn unknown_column_in_exclude_also_errors() {
        let err = parse_node_filter("!agee:1h", &no_label_cols()).unwrap_err();
        assert!(
            err.contains("unknown column") && err.contains("agee"),
            "error should mention the bad column: {}",
            err
        );
    }
```

(e) 現状の Phase A 新規テスト `multiword_column_with_space_is_filterable_via_compact_token` / `column_not_in_header_produces_not_in_view_error` / `empty_header_skips_column_validation` を削除し、代わりに次を追加:

```rust
    #[test]
    fn builtin_columns_are_accepted() {
        // name と status は builtin → エラーにならない
        assert!(parse_node_filter("name:n status:s", &no_label_cols()).is_ok());
    }

    #[test]
    fn hyphenated_builtin_column_is_accepted_via_normalization() {
        // INTERNAL-IP は builtin。internalip / internal-ip いずれでも受理。
        let p = parse_node_filter("internalip:10.", &no_label_cols()).unwrap();
        assert!(p.column_includes.contains_key("internalip"));
        let p2 = parse_node_filter("internal-ip:10.", &no_label_cols()).unwrap();
        assert!(p2.column_includes.contains_key("internalip"));
    }
```

- [ ] **Step 2: テストを実行して失敗（コンパイルエラー）を確認**

Run: `cargo test --lib features::node::filter 2>&1 | tail -30`
Expected: コンパイルエラー（`parse_node_filter` のシグネチャが `&[String]` のまま、削除した `header()` 参照、未復帰の `NodeLabelColumn` 等）。

- [ ] **Step 3: parser.rs の imports / valid_columns / parse_node_filter を既知列ベースに戻す**

`src/features/node/filter/parser.rs` の import（現状 17-19）を次に:

```rust
use regex::Regex;
use strum::IntoEnumIterator;

use crate::{
    features::node::node_columns::{NodeColumn, NodeLabelColumn},
    ui::widget::{normalize_column_name, TableFilterPredicate},
};
```

`valid_columns`（現状 210-214）を次に置換:

```rust
/// Build the set of valid (known) column names: builtin Node columns plus any
/// defined label columns, normalized so matching is case/format-insensitive.
fn valid_columns(label_registry: &[NodeLabelColumn]) -> HashSet<String> {
    let mut set: HashSet<String> = NodeColumn::iter()
        .map(|c| normalize_column_name(c.display()))
        .collect();
    for lc in label_registry {
        set.insert(normalize_column_name(&lc.header));
    }
    set
}
```

`parse_node_filter`（現状 220-302、doc コメント含む）を次に置換:

```rust
/// Parse a Node-filter input string into a `TableFilterPredicate`.
///
/// Column references are validated against the set of *known* columns (builtin
/// Node columns plus defined label columns in `label_registry`); an unknown
/// column produces a parse error. A known column that is not currently
/// displayed is accepted here and becomes inactive at match time.
///
/// Values may be quoted (`"..."` / `'...'`) with the same escape rules as the
/// Pod log query parser: `\"` → `"`  `\'` → `'`  `\\` → `\`  `\<other>` verbatim.
pub fn parse_node_filter(
    input: &str,
    label_registry: &[NodeLabelColumn],
) -> Result<TableFilterPredicate, String> {
    let valid = valid_columns(label_registry);

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
                if !valid.contains(&col) {
                    return Err(format!("unknown column '{}'", column));
                }
                let rx =
                    Regex::new(&value).map_err(|e| format!("invalid regex '{}': {}", value, e))?;
                column_includes.entry(col).or_default().push(rx);
            }
            Term::Exclude { column, value } => {
                let col = normalize_column_name(&column);
                if !valid.contains(&col) {
                    return Err(format!("unknown column '{}'", column));
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

（`parse_token`／`Term`／quoting ヘルパーは不変。）

- [ ] **Step 4: `TableFilterParser` を `Fn(&str)` に戻す＋substring_applicator**

`src/ui/widget/table/filter_applicator.rs` の define_callback（現状 111）を:

```rust
define_callback!(pub TableFilterParser, Fn(&str) -> Result<TableFilterPredicate, String>);
```

`substring_applicator` のクロージャ先頭（現状 `(move |input: &str, _header: &[String]| {`）を:

```rust
    let parser: TableFilterParser = (move |input: &str| {
```

- [ ] **Step 5: table.rs の呼び出しとテスト用パーサを戻す**

`src/ui/widget/table.rs` の `run_parser_and_update_state` を次に置換（header 取得を削除し `&input` のみ渡す）:

```rust
    fn run_parser_and_update_state(&mut self) -> Option<TableFilterPredicate> {
        let applicator = self.filter_applicator.as_ref()?;
        let input = self
            .filter_form
            .as_ref()
            .map(|f| f.content())
            .unwrap_or_default();

        match (applicator.parser.closure)(&input) {
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

同ファイル内の**テスト用パーサのクロージャを全て単一引数に戻す**。`TableFilterParser::from(move |_: &str, _: &[String]| ...)` や `move |input: &str, _header: &[String]| ...` の形を探し、第2引数を削除する。具体的には `filter_cancel_returns_some_callback_when_applicator_has_on_cancel` 内、および `help_dispatch` モジュール内の2クロージャ（計3箇所）。例:

```rust
                TableFilterParser::from(move |_: &str| {
                    Ok(crate::ui::widget::TableFilterPredicate::default())
                }),
```

（`filter_applicator.rs` のテスト内 `invoke_parser` ヘルパーも `&[]` を渡している場合は単一引数呼び出しに戻す。grep: `parser.closure` / `&[String]` / `|_: &str,` で全箇所確認。）

- [ ] **Step 6: node/filter.rs を `label_registry` 受け取りに戻す**

`src/features/node/filter.rs` の import（現状 15）を:

```rust
    features::{
        component_id::NODE_FILTER_HELP_DIALOG_ID,
        node::{message::NodeMessage, node_columns::NodeLabelColumn},
    },
```

関数シグネチャとクロージャ（現状 39-41）を:

```rust
pub fn node_filter_applicator(
    label_registry: Vec<NodeLabelColumn>,
    tx: Sender<Message>,
) -> TableFilterApplicator {
    let parser: TableFilterParser =
        (move |input: &str| parse_node_filter(input, &label_registry)).into();
```

（`tx_apply`/`on_apply`/`on_cancel`/`TableFilterApplicator::new(...)` 以降は不変。）

同ファイルのテスト（現状 71-75）を:

```rust
    #[test]
    fn applicator_constructs_without_panic() {
        let (tx, _rx) = crossbeam::channel::bounded(1);
        let _ = node_filter_applicator(Vec::new(), tx);
    }
```

- [ ] **Step 7: node.rs と tab.rs を `label_registry` 経路に戻す**

`src/features/node/view/widgets/node.rs`:
- import に `NodeLabelColumn` を戻す（`node::{ filter::node_filter_applicator, message::NodeDetailMessage, node_columns::NodeLabelColumn }`）。
- シグネチャを `pub fn node_widget(tx: Sender<Message>, label_registry: Vec<NodeLabelColumn>, theme: WidgetThemeConfig) -> Widget<'static>` に。
- filter 配線を `.filter_applicator(node_filter_applicator(label_registry, tx.clone()))` に。

`src/features/node/view/tab.rs`:
- `node_widget` 呼び出しを `let node_widget = node_widget(tx.clone(), label_registry.clone(), theme.clone());` に（`label_registry` は `node_columns_dialog` でも使用中なので `.clone()`）。

- [ ] **Step 8: 全テストを実行して green を確認**

Run: `cargo test --all 2>&1 | tail -40`
Expected: コンパイル成功、全テスト PASS。

`cargo clippy --all-targets 2>&1 | rg "src/features/node|src/ui/widget/table"` を実行し、変更ファイルに新規警告が無いこと（未使用 import 等が残っていないか）を確認。

- [ ] **Step 9: コミット**

```bash
git add src/ui/widget/table/filter_applicator.rs src/ui/widget/table.rs src/features/node/filter.rs src/features/node/filter/parser.rs src/features/node/view/widgets/node.rs src/features/node/view/tab.rs
git commit -m "feat(node-filter): validate against known columns; hidden known columns become inactive"
```

---

## Task 3: inactive バッジを count_indicator に統合

**Files:**
- Modify: `src/ui/widget/table.rs`（`count_indicator` 290-305、新規 `inactive_columns` ヘルパー、テスト）

- [ ] **Step 1: 失敗するテストを書く**

`src/ui/widget/table.rs` のテスト（`count_indicator` 系のテストモジュール、`table_with_three` 等がある箇所）に追加。`HashMap`/`Regex` はそのテストモジュールで使用実績あり（無ければ `use` を足す）。

```rust
        #[test]
        fn count_indicator_appends_inactive_badge_for_hidden_filtered_column() {
            use std::collections::HashMap;
            use regex::Regex;

            let mut table = Table::builder()
                .header(["NAME".to_string(), "STATUS".to_string()])
                .items([TableItem::new(vec!["a".to_string(), "Ready".to_string()], None)])
                .build();

            let mut includes = HashMap::new();
            includes.insert("version".to_string(), vec![Regex::new("1.30").unwrap()]);
            table.filter_state = Some(crate::ui::widget::TableFilterPredicate {
                column_includes: includes,
                raw: "version:1.30".to_string(),
                ..Default::default()
            });

            let ind = table.count_indicator();
            assert!(
                ind.contains("(inactive: version)"),
                "indicator should flag the hidden filtered column: {}",
                ind
            );
        }

        #[test]
        fn count_indicator_has_no_inactive_badge_when_all_columns_visible() {
            use std::collections::HashMap;
            use regex::Regex;

            let mut table = Table::builder()
                .header(["NAME".to_string(), "STATUS".to_string()])
                .items([TableItem::new(vec!["a".to_string(), "Ready".to_string()], None)])
                .build();

            let mut includes = HashMap::new();
            includes.insert("status".to_string(), vec![Regex::new("Ready").unwrap()]);
            table.filter_state = Some(crate::ui::widget::TableFilterPredicate {
                column_includes: includes,
                raw: "status:Ready".to_string(),
                ..Default::default()
            });

            let ind = table.count_indicator();
            assert!(
                !ind.contains("inactive"),
                "no badge when all filtered columns are visible: {}",
                ind
            );
        }
```

- [ ] **Step 2: テストを実行して失敗を確認**

Run: `cargo test --lib ui::widget::table 2>&1 | tail -30`
Expected: `count_indicator_appends_inactive_badge_for_hidden_filtered_column` が FAIL（バッジ未実装）。

- [ ] **Step 3: `inactive_columns` を追加し `count_indicator` に統合**

`src/ui/widget/table.rs` の `count_indicator`（現状 290-305）を次に置換:

```rust
    pub fn count_indicator(&self) -> String {
        let index = self.state.selected().map(|i| i + 1).unwrap_or(0);
        let visible = self.items.len();
        let total = self.items.original_len();

        let filter_active = self
            .filter_state
            .as_ref()
            .is_some_and(|predicate| !predicate.raw.is_empty());

        let mut indicator = if filter_active {
            format!(" [{}/{} ({})]", index, visible, total)
        } else {
            format!(" [{}/{}]", index, visible)
        };

        let inactive = self.inactive_columns();
        if !inactive.is_empty() {
            indicator.push_str(&format!(" (inactive: {})", inactive.join(", ")));
        }

        indicator
    }

    /// Names of filtered columns that are not currently displayed in the header
    /// (their constraints are inactive). Sorted and de-duplicated for a stable
    /// title. Empty when no filter is active or every filtered column is shown.
    fn inactive_columns(&self) -> Vec<String> {
        let Some(state) = self.filter_state.as_ref() else {
            return Vec::new();
        };
        let visible_column_keys: Vec<String> = self
            .items
            .header()
            .original()
            .iter()
            .map(|h| normalize_column_name(h))
            .collect();
        // Collect into a BTreeSet so the result is unique + sorted (a stable
        // title) in one step — avoids the manual sort-then-dedup and its
        // "dedup only removes consecutive duplicates" footgun. A column can be
        // in both includes and excludes, so de-duplication is required.
        state
            .column_includes
            .keys()
            .chain(state.column_excludes.keys())
            .filter(|c| !visible_column_keys.contains(c))
            .cloned()
            .collect::<std::collections::BTreeSet<String>>()
            .into_iter()
            .collect()
    }
```

注: `filter_state` のキーは parser が `normalize_column_name` で正規化済みなので `visible_column_keys.contains(c)` でそのまま比較できる。`normalize_column_name` は `table.rs` の `pub use filter_applicator::{…}` 再エクスポートでスコープ内（Phase A で追加済み）。

- [ ] **Step 4: テストを実行して成功を確認**

Run: `cargo test --lib ui::widget::table 2>&1 | tail -30`
Expected: PASS（新規2テスト＋既存 count_indicator テスト全て green）。

- [ ] **Step 5: コミット**

```bash
git add src/ui/widget/table.rs
git commit -m "feat(table-filter): show (inactive: ...) badge for hidden filtered columns"
```

---

## Task 4: Node フィルタヘルプ文言を 3-way に更新

**Files:**
- Modify: `src/features/node/view/widgets/node_filter_help.rs`（`content()` 末尾段落）

- [ ] **Step 1: 文言を更新**

`src/features/node/view/widgets/node_filter_help.rs` の `content()` 内、末尾段落（Phase A で入れた "Filterable columns are the ones currently shown..." の段落）を次に置換:

```
        Columns must be builtin or defined label columns; unknown
        columns produce an error. A term on a column that is not
        currently shown becomes inactive (kept, but not applied) until
        that column is shown again; the title shows (inactive: ...).
        Column names ignore case, spaces, '-' and '_'. Press Enter to
        apply, Esc to cancel. Type ? or help in the filter input to
        open this help.
```

（インデントは既存の indoc ブロックに合わせる。他の行は変更しない。）

- [ ] **Step 2: ビルド確認**

Run: `cargo build 2>&1 | tail -15`
Expected: コンパイル成功（文字列のみ変更）。

- [ ] **Step 3: コミット**

```bash
git add src/features/node/view/widgets/node_filter_help.rs
git commit -m "docs(node-filter): help text describes inactive terms and unknown-column errors"
```

---

## Task 5: 全体検証

**Files:** なし（検証のみ）

- [ ] **Step 1: 全テスト**

Run: `cargo test --all 2>&1 | rg "test result:"`
Expected: 全テスト PASS。

- [ ] **Step 2: clippy**

Run: `cargo clippy --all-targets 2>&1 | tail -30`
Expected: 変更ファイル（filter_applicator.rs / table.rs / node/filter*.rs / node/view/*）に新規警告なし。`NodeColumn`/`NodeLabelColumn`/`strum::IntoEnumIterator` を復帰させたので未使用警告が出ていないことも確認。

- [ ] **Step 3: format**

Run: `cargo +nightly fmt --check 2>&1 | tail -20`
Expected: 差分なし。出たら `cargo +nightly fmt` を実行して再コミット。

- [ ] **Step 4: 手動スモーク（実クラスタ／KIND、不可なら省略明記）**

`cargo run` で Node タブを開き確認:
1. `version:1.30 status:Ready` を適用（VERSION 表示中）→ 条件どおり絞られる。
2. `t` で VERSION を非表示 → **行は消えず**残りの `status:Ready` で絞られ、タイトルに `(inactive: version)` が出る。
3. `t` で VERSION を再表示 → version 条件が**自動復活**し元の結果に戻る。
4. `stauts:Ready`（タイポ）→ `unknown column 'stauts'` エラー。
5. `internal-ip:10.` と `internalip:10.` が同結果（builtin の正規化）。
6. `label:role=worker` がサーバーサイドで効く（非回帰）。
7. `?` でヘルプが更新後の文言で開く。

- [ ] **Step 5: （必要なら）fmt 修正をコミット**

```bash
git add -A
git commit -m "style: cargo fmt"
```

---

## Self-Review（計画作成者によるチェック結果）

- **Spec カバレッジ:** matching の inactive スキップ（spec §1）→ Task 1。parser 既知列検証・未定義エラー・header 配線撤去（spec §2・§5）→ Task 2。inactive バッジ（spec §3）→ Task 3。ヘルプ文言（spec §4）→ Task 4。エッジケース（空 header／exclude／重複）→ Task 1（exclude skip）・Task 3（dedup）・parser は header 非依存で空 header 特例不要（spec エッジケース通り）。テスト → 各 Task＋Task 5。Phase A 差分（Task1 維持・Task2/3 作り替え）→ Task 1/2 に反映。全項目に対応タスクあり。
- **プレースホルダ:** TBD/TODO なし。テストの一括置換（Task 2 Step 1(b)）は対象文字列が一意（`&header()`→`&no_label_cols()`）の機械的変換。テスト用パーサ revert（Task 2 Step 5）は grep 指定済み。
- **型整合:** `parse_node_filter(&str, &[NodeLabelColumn]) -> Result<TableFilterPredicate, String>`（Task 2）と `node_filter_applicator(Vec<NodeLabelColumn>, Sender<Message>)`・`node_widget(Sender<Message>, Vec<NodeLabelColumn>, WidgetThemeConfig)`（Task 2 Step 6/7）一致。`TableFilterParser=Fn(&str)->…`（Task 2 Step 4）と全クロージャ・`(parser.closure)(&input)`（Step 5）一致。`column_index(&[String], &str)->Option<usize>`・`cell_text(&TableItem, usize)->String`（Task 1）と `matches` 内呼び出し一致。`inactive_columns(&self)->Vec<String>`（Task 3）と `count_indicator` 呼び出し一致。`normalize_column_name`（filter_applicator、Phase A 維持）を parser・table.rs 双方で使用。
