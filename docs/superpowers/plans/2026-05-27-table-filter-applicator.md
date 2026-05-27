# Table widget filter_applicator 化 Implementation Plan (PR A)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `Table` ウィジェットの組み込みフィルタを pluggable 化し、`TableFilterApplicator`（parser + strategy + help_dialog_id + on_apply）を受け取って動作する設計にする。既存タブはすべて `SubstringFilterApplicator` に乗り換え、`filtered_key` / `filtered_word` ベースの旧パスを完全削除する。

**Architecture:** `TableFilterApplicator` 構造体に parser/strategy/help_dialog_id/on_apply を 1 ショットで束ね、`Table::builder().filter_applicator(...)` で渡す。Table 内部では parser を呼んで `TableFilterPredicate`（列内 OR・列間 AND・exclude any-match-excludes の統一構造体）を得て、`filter_state` に保持、行フィルタリングは `predicate.matches()` 経由。parse 失敗時は `filter_error` を粘着的に保持し、テーブル本体置換でエラー表示する。セル比較は既存実装に揃えて ANSI 制御コードを除去（`styled_graphemes_symbols().concat()`）してから regex マッチする。

**Tech Stack:** Rust, ratatui, regex, kubetui の既存パターン（`define_callback!`、enum_dispatch ベースの Widget、`InnerItem` 構造）。

**前提:** PR #980（`feat/table-optional-filter`、Table の `filter_form: Option<FilterForm>` 化）が前提。本 PR はその上にスタックする。ブランチは `feat/table-filter-applicator` を `feat/table-optional-filter` から切る。

**設計スペック:** `docs/superpowers/specs/2026-05-27-table-filter-redesign.md`。

**ANSI 除去について:** 現状の kubetui は kube ワーカー側でステータス値等に ANSI 色付けを埋め込んでいる（本来は Table 側でカラムごとに色付けすべきだが、それは別タスク）。既存 substring filter は `styled_graphemes_symbols().concat()` で表示可視部分のみ抽出してから比較しており、新フィルタもこれに揃える。これで既存タブの挙動完全互換、かつ regex の anchor 系（`^`, `$`）も正しく動く。

---

## ファイル構成

**新規作成:**
- `src/ui/widget/table/filter_applicator.rs` — `TableFilterApplicator`, `ApplyStrategy`, `TableFilterPredicate`, `TableFilterParser`, `OnFilterApply`, `substring_applicator()`, `cell_of()` ヘルパ

**変更:**
- `src/ui/widget/table.rs` — 3 フィールド追加（filter_applicator, filter_state, filter_error）、builder method、on_key_event の Live/Enter ハンドリング、`?`/`help` 入力分岐、`item_passes_filter`、render の filter_error 優先、`filtered_key` 完全削除
- `src/ui/widget/table/item.rs` — `update_filter` / `inner_filter_items` / `filtered_word` / `filtered_key` / `filtered_index` を削除、外部から渡された predicate で filter する `apply_filter` を追加
- `src/ui/widget.rs` — `pub use table::filter_applicator::...` re-export
- `src/features/pod/view/widgets/pod.rs` — `.filtered_key(...)` → `.filter_applicator(substring_applicator(...))`
- `src/features/config/view/widgets/config.rs` — 同上
- `src/features/network/view/widgets/network.rs` — 同上
- `src/features/api_resources/view/dialog.rs` — 同上
- `src/features/yaml/view/dialogs/name.rs` — 同上
- `src/features/yaml/view/dialogs/kind.rs` — 同上
- `src/features/context/view/dialog.rs` — 同上
- `src/features/namespace/view/single_namespace_dialog.rs` — 同上
- `src/features/namespace/view/multiple_namespaces_dialog.rs` — 同上

**触らない:**
- Tab 階層の `widget_error`（`Tab.error_states` 等。既存仕組みのまま）
- Node 関連（PR B のスコープ）
- kube ワーカーの ANSI 埋め込み（暫定的に共存。Table 側カラム色付けは別タスク）

---

## Task 0: ブランチ作成

**Files:** なし（ブランチ操作のみ）

- [ ] **Step 1: PR #980 ブランチへ切替**

```bash
git fetch origin
git checkout feat/table-optional-filter
git pull origin feat/table-optional-filter
```

- [ ] **Step 2: 新ブランチ作成**

```bash
git checkout -b feat/table-filter-applicator
```

- [ ] **Step 3: ベース確認**

```bash
git log --oneline -3
```
Expected: トップは `feat(table): make filter_form optional via Option<FilterForm>`（PR #980）。

---

## Task 1: TableFilterPredicate + matches + ANSI 除去 + 単体テスト

**Files:**
- Create: `src/ui/widget/table/filter_applicator.rs`
- Modify: `src/ui/widget/table.rs`（モジュール宣言）

- [ ] **Step 1: モジュール宣言を追加**

`src/ui/widget/table.rs` の冒頭部分（`mod filter;` の隣）に:
```rust
mod filter_applicator;
```

- [ ] **Step 2: filter_applicator.rs を作成**

`src/ui/widget/table/filter_applicator.rs`:
```rust
use std::collections::HashMap;

use regex::Regex;

use crate::ui::widget::{styled_graphemes::StyledGraphemes, TableItem};

/// 列ベースのフィルタ条件。
///
/// 意味論:
/// - include: 同一列内は OR、列間は AND（`AND_col(OR_pat in col)`）
/// - exclude: いずれかにマッチしたら除外（any-match-excludes）
/// - キーは小文字に正規化した列名
#[derive(Debug, Default, Clone)]
pub struct TableFilterPredicate {
    pub column_includes: HashMap<String, Vec<Regex>>,
    pub column_excludes: HashMap<String, Vec<Regex>>,
    /// k8s labelSelector（Node 系のみ。matches では参照されず on_apply 経由で別ワーカーへ）
    pub label_selector: Option<String>,
    /// 入力文字列（タイトル表示・デバッグ用）
    pub raw: String,
}

impl TableFilterPredicate {
    pub fn matches(&self, item: &TableItem, header: &[String]) -> bool {
        for (col, regexes) in &self.column_includes {
            let cell = cell_of(item, header, col).unwrap_or_default();
            if !regexes.iter().any(|r| r.is_match(&cell)) {
                return false;
            }
        }
        for (col, regexes) in &self.column_excludes {
            let cell = cell_of(item, header, col).unwrap_or_default();
            if regexes.iter().any(|r| r.is_match(&cell)) {
                return false;
            }
        }
        true
    }

    pub fn is_empty(&self) -> bool {
        self.column_includes.is_empty()
            && self.column_excludes.is_empty()
            && self.label_selector.is_none()
    }
}

/// 指定列のセル値を「ANSI 制御コード除去後の表示文字列」として返す。
///
/// 既存 substring フィルタの `styled_graphemes_symbols().concat()` と同じ
/// 経路。kube ワーカー側がステータス値等に色付けで `\x1b[...m` を埋め込む
/// 現状を吸収するため、フィルタ比較は表示可視部分のみで行う。
///
/// 列名のマッチは大小区別なし。
fn cell_of(item: &TableItem, header: &[String], col: &str) -> Option<String> {
    let idx = header.iter().position(|h| h.eq_ignore_ascii_case(col))?;
    let raw = item.item.get(idx)?;
    Some(raw.styled_graphemes_symbols().concat())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn item(cells: &[&str]) -> TableItem {
        TableItem::new(
            cells.iter().map(ToString::to_string).collect::<Vec<_>>(),
            None,
        )
    }

    fn header(cols: &[&str]) -> Vec<String> {
        cols.iter().map(ToString::to_string).collect()
    }

    fn rx(s: &str) -> Regex {
        Regex::new(s).expect("test regex must compile")
    }

    #[test]
    fn empty_predicate_matches_anything() {
        let p = TableFilterPredicate::default();
        let h = header(&["NAME", "STATUS"]);
        assert!(p.matches(&item(&["x", "y"]), &h));
        assert!(p.is_empty());
    }

    #[test]
    fn includes_within_column_use_or() {
        let mut p = TableFilterPredicate::default();
        p.column_includes
            .insert("status".to_string(), vec![rx("Ready"), rx("Pending")]);
        let h = header(&["NAME", "STATUS"]);
        assert!(p.matches(&item(&["x", "Ready"]), &h));
        assert!(p.matches(&item(&["x", "Pending"]), &h));
        assert!(!p.matches(&item(&["x", "Failed"]), &h));
    }

    #[test]
    fn includes_across_columns_use_and() {
        let mut p = TableFilterPredicate::default();
        p.column_includes
            .insert("status".to_string(), vec![rx("Ready")]);
        p.column_includes
            .insert("name".to_string(), vec![rx("nginx")]);
        let h = header(&["NAME", "STATUS"]);
        assert!(p.matches(&item(&["nginx-x", "Ready"]), &h));
        assert!(!p.matches(&item(&["web-y", "Ready"]), &h));
        assert!(!p.matches(&item(&["nginx-x", "Pending"]), &h));
    }

    #[test]
    fn excludes_any_match_excludes() {
        let mut p = TableFilterPredicate::default();
        p.column_excludes
            .insert("status".to_string(), vec![rx("Pending"), rx("Failed")]);
        let h = header(&["NAME", "STATUS"]);
        assert!(p.matches(&item(&["x", "Ready"]), &h));
        assert!(!p.matches(&item(&["x", "Pending"]), &h));
        assert!(!p.matches(&item(&["x", "Failed"]), &h));
    }

    #[test]
    fn excludes_across_columns_block_on_any_match() {
        let mut p = TableFilterPredicate::default();
        p.column_excludes
            .insert("status".to_string(), vec![rx("Pending")]);
        p.column_excludes
            .insert("name".to_string(), vec![rx("kube")]);
        let h = header(&["NAME", "STATUS"]);
        assert!(p.matches(&item(&["nginx-x", "Ready"]), &h));
        assert!(!p.matches(&item(&["nginx-x", "Pending"]), &h));
        assert!(!p.matches(&item(&["kube-proxy", "Ready"]), &h));
    }

    #[test]
    fn includes_and_excludes_combine() {
        let mut p = TableFilterPredicate::default();
        p.column_includes
            .insert("status".to_string(), vec![rx("Ready")]);
        p.column_excludes
            .insert("name".to_string(), vec![rx("kube")]);
        let h = header(&["NAME", "STATUS"]);
        assert!(p.matches(&item(&["nginx-x", "Ready"]), &h));
        assert!(!p.matches(&item(&["kube-proxy", "Ready"]), &h));
        assert!(!p.matches(&item(&["nginx-x", "Pending"]), &h));
    }

    #[test]
    fn column_name_matching_is_case_insensitive() {
        let mut p = TableFilterPredicate::default();
        p.column_includes
            .insert("status".to_string(), vec![rx("Ready")]);
        let h = header(&["NAME", "STATUS"]); // ヘッダ大文字、キー小文字
        assert!(p.matches(&item(&["x", "Ready"]), &h));
    }

    #[test]
    fn unknown_column_yields_empty_cell_so_fails_match() {
        let mut p = TableFilterPredicate::default();
        p.column_includes
            .insert("nonexistent".to_string(), vec![rx("anything")]);
        let h = header(&["NAME", "STATUS"]);
        assert!(!p.matches(&item(&["x", "y"]), &h));
    }

    #[test]
    fn ansi_escape_in_cell_is_stripped_before_match() {
        // kube ワーカーが埋め込む色付け前提のセル値（例: STATUS=Ready を緑で）
        // ユーザーが素の "Ready" を指定して通ること。
        let colored_status = "\x1b[32mReady\x1b[0m";
        let mut p = TableFilterPredicate::default();
        p.column_includes
            .insert("status".to_string(), vec![rx("Ready")]);
        let h = header(&["NAME", "STATUS"]);
        assert!(p.matches(&item(&["nginx", colored_status]), &h));
    }

    #[test]
    fn ansi_escape_does_not_pollute_anchor_match() {
        // ^Ready$ (anchor 付き完全一致) は raw だと ESC で壊れるが、
        // ANSI 除去後の "Ready" に対しては正しく完全一致する。
        let colored_status = "\x1b[31mReady\x1b[0m";
        let mut p = TableFilterPredicate::default();
        p.column_includes
            .insert("status".to_string(), vec![rx("^Ready$")]);
        let h = header(&["NAME", "STATUS"]);
        assert!(p.matches(&item(&["nginx", colored_status]), &h));
    }

    #[test]
    fn ansi_escape_in_cell_not_matched_as_part_of_value() {
        // 「\x1b[31m が含まれる」みたいな regex は ANSI 除去後の値には
        // ヒットしない（除去によって ESC が消えるため）。
        let colored_status = "\x1b[31mFailed\x1b[0m";
        let mut p = TableFilterPredicate::default();
        p.column_includes
            .insert("status".to_string(), vec![rx(r"\[31m")]);
        let h = header(&["NAME", "STATUS"]);
        assert!(!p.matches(&item(&["nginx", colored_status]), &h));
    }
}
```

- [ ] **Step 3: テスト実行**

```bash
cargo test --bin kubetui ui::widget::table::filter_applicator::tests
```
Expected: 11 passed; 0 failed。

- [ ] **Step 4: フルビルド・全テスト**

```bash
cargo build && cargo test
```
Expected: ビルド green、既存全テスト pass。

- [ ] **Step 5: コミット**

```bash
git add -A
git commit -m "feat(table): add TableFilterPredicate with unified semantics

Column-internal OR, cross-column AND, any-match-excludes. Single struct
covering both Substring (single-column) and Node-style (multi-column +
labelSelector) uses; the predicate itself only handles client-side
matching, label_selector is exposed for callers that need server-side
filtering (Node tab via on_apply).

cell_of() strips ANSI escape sequences via styled_graphemes_symbols()
before regex match, matching the existing inner_filter_items behavior.
This accommodates the current architecture where kube workers embed
SGR codes (e.g., Pod status coloring) in cell values; without stripping,
anchored regexes like ^Ready\$ would fail against colored cells."
```

---

## Task 2: TableFilterApplicator・ApplyStrategy・コールバック型

**Files:**
- Modify: `src/ui/widget/table/filter_applicator.rs`

- [ ] **Step 1: コールバック型と Applicator を追加**

`src/ui/widget/table/filter_applicator.rs` の **テスト mod の前** に追加:
```rust
use crate::{define_callback, ui::{event::EventResult, Window}};

define_callback!(
    pub TableFilterParser,
    Fn(&str) -> Result<TableFilterPredicate, String>
);

define_callback!(
    pub OnFilterApply,
    Fn(&TableFilterPredicate, &mut Window)
);

/// フィルタ適用のタイミング戦略。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyStrategy {
    /// 毎キーで parser を呼び、自動的に filter_state を更新する。
    /// 既存タブの "live" substring 動作で使う。
    Live,
    /// Enter のみで parser を呼ぶ。複雑な構文（Node 系）で使う。
    EnterToConfirm,
}

/// Table の filter 機能を構成する。parser / strategy / help dialog id /
/// apply hook を 1 ショットで束ねる。Table::builder().filter_applicator()
/// で渡す。
pub struct TableFilterApplicator {
    pub(crate) parser: TableFilterParser,
    pub(crate) strategy: ApplyStrategy,
    pub(crate) help_dialog_id: Option<&'static str>,
    pub(crate) on_apply: Option<OnFilterApply>,
}

impl std::fmt::Debug for TableFilterApplicator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TableFilterApplicator")
            .field("strategy", &self.strategy)
            .field("help_dialog_id", &self.help_dialog_id)
            .field("parser", &"<fn>")
            .field("on_apply", &self.on_apply.as_ref().map(|_| "<fn>"))
            .finish()
    }
}

impl TableFilterApplicator {
    pub fn new(parser: TableFilterParser, strategy: ApplyStrategy) -> Self {
        Self {
            parser,
            strategy,
            help_dialog_id: None,
            on_apply: None,
        }
    }

    pub fn with_help_dialog(mut self, id: &'static str) -> Self {
        self.help_dialog_id = Some(id);
        self
    }

    pub fn with_on_apply(mut self, cb: OnFilterApply) -> Self {
        self.on_apply = Some(cb);
        self
    }
}

// `EventResult` は OnFilterApply の return 型では未使用だが、将来 ApplyStrategy
// 拡張時のため import 維持。現状は unused なので silence。
#[allow(unused_imports)]
use EventResult as _;
```

- [ ] **Step 2: ビルド**

```bash
cargo build 2>&1 | grep -E "^error" | head
```
Expected: error 0。

- [ ] **Step 3: 全テスト**

```bash
cargo test
```
Expected: 既存全 pass + Task 1 の 11 pass。

- [ ] **Step 4: コミット**

```bash
git add -A
git commit -m "feat(table): add TableFilterApplicator / ApplyStrategy / callback types

Bundles parser + apply strategy + help dialog id + on_apply callback into
one factory that tabs pass to Table::builder().filter_applicator()."
```

---

## Task 3: Table widget へフィールド追加 + builder method

**Files:**
- Modify: `src/ui/widget/table.rs`

- [ ] **Step 1: filter_applicator モジュール公開を確認**

`src/ui/widget/table.rs` 既存の `pub use filter::{FilterForm, FilterFormTheme};` の隣に:
```rust
pub use filter_applicator::{
    substring_applicator,
    ApplyStrategy,
    OnFilterApply,
    TableFilterApplicator,
    TableFilterParser,
    TableFilterPredicate,
};
```
注: `substring_applicator` は Task 4 で実装するが、re-export はここで宣言しておくと一括変更を抑えられる。Task 4 未完なら一時的に `substring_applicator` を抜く形で構わない。

- [ ] **Step 2: TableBuilder に filter_applicator フィールド追加**

`pub struct TableBuilder` の `filter_form: Option<FilterForm>,` の直後に:
```rust
filter_applicator: Option<TableFilterApplicator>,
```

- [ ] **Step 3: TableBuilder に filter_applicator setter 追加**

`pub fn filter_form(...)` の直後に追加:
```rust
/// Enable rich filter parsing with the given applicator. Replaces the
/// default substring-only filter behavior with parser-driven filtering.
pub fn filter_applicator(mut self, applicator: TableFilterApplicator) -> Self {
    self.filter_applicator = Some(applicator);
    self
}
```

- [ ] **Step 4: Table struct にフィールド 3 つ追加**

`pub struct Table<'a>` の `filter_form: Option<FilterForm>,` の直後に:
```rust
filter_applicator: Option<TableFilterApplicator>,
filter_state: Option<TableFilterPredicate>,
filter_error: Option<String>,
```

- [ ] **Step 5: TableBuilder::build で受け渡し**

`fn build(self) -> Table<'static>` 内、Table 初期化部分（既存の `filter_form: self.filter_form,` の隣）:
```rust
filter_form: self.filter_form,
filter_applicator: self.filter_applicator,
filter_state: None,
filter_error: None,
```

- [ ] **Step 6: ビルド・全テスト**

```bash
cargo build 2>&1 | grep -E "^error" | head
cargo test
```
Expected: error 0、全テスト pass（フィールド追加のみで挙動変化なし）。

- [ ] **Step 7: コミット**

```bash
git add -A
git commit -m "feat(table): add filter_applicator/filter_state/filter_error fields"
```

---

## Task 4: substring_applicator ファクトリ + テスト

**Files:**
- Modify: `src/ui/widget/table/filter_applicator.rs`

- [ ] **Step 1: ファクトリ実装**

`filter_applicator.rs`、テスト mod の前に追加:
```rust
/// 既存タブが使う「単一列の部分一致」フィルタを構成する。
/// `column` は対象列名（大小区別なし、内部で小文字化）。
///
/// 挙動は既存の `filtered_key + split(' ').any(contains)` と等価:
/// - 空入力 → フィルタなし（全行 pass）
/// - スペース区切りの複数 pattern → OR
/// - 各 pattern は `regex::escape` でリテラル化（ユーザーは regex を
///   意識しなくていい、`.` `*` 等が混入しても安全）
pub fn substring_applicator(column: &str) -> TableFilterApplicator {
    let col = column.to_string().to_lowercase();
    let parser: TableFilterParser = (move |input: &str| {
        let raw = input.to_string();
        let patterns: Result<Vec<Regex>, _> = input
            .split_whitespace()
            .map(regex::escape)
            .map(|p| Regex::new(&p))
            .collect();
        let patterns = patterns.map_err(|e| e.to_string())?;

        let mut column_includes = HashMap::new();
        if !patterns.is_empty() {
            column_includes.insert(col.clone(), patterns);
        }

        Ok(TableFilterPredicate {
            column_includes,
            column_excludes: HashMap::new(),
            label_selector: None,
            raw,
        })
    })
    .into();

    TableFilterApplicator::new(parser, ApplyStrategy::Live)
}
```

- [ ] **Step 2: substring_applicator のテスト追加**

`mod tests` 内に追加:
```rust
fn invoke_parser(a: &TableFilterApplicator, input: &str) -> TableFilterPredicate {
    (a.parser.closure)(input).expect("test input must parse")
}

#[test]
fn substring_applicator_empty_input_matches_everything() {
    let a = substring_applicator("NAME");
    let p = invoke_parser(&a, "");
    let h = header(&["NAME"]);
    assert!(p.matches(&item(&["nginx"]), &h));
    assert!(p.matches(&item(&[""]), &h));
    assert!(p.is_empty());
}

#[test]
fn substring_applicator_single_pattern_substring_match() {
    let a = substring_applicator("NAME");
    let p = invoke_parser(&a, "nginx");
    let h = header(&["NAME"]);
    assert!(p.matches(&item(&["abc-nginx-prod"]), &h));
    assert!(!p.matches(&item(&["web-server"]), &h));
}

#[test]
fn substring_applicator_space_separated_is_or() {
    // 既存挙動と等価: "nginx web" は NAME に nginx OR web
    let a = substring_applicator("NAME");
    let p = invoke_parser(&a, "nginx web");
    let h = header(&["NAME"]);
    assert!(p.matches(&item(&["nginx-x"]), &h));
    assert!(p.matches(&item(&["web-y"]), &h));
    assert!(!p.matches(&item(&["api-z"]), &h));
}

#[test]
fn substring_applicator_special_chars_are_escaped() {
    // "a.b" がリテラルとして扱われる（regex の `.` ではない）
    let a = substring_applicator("NAME");
    let p = invoke_parser(&a, "a.b");
    let h = header(&["NAME"]);
    assert!(p.matches(&item(&["x-a.b-y"]), &h));
    assert!(!p.matches(&item(&["x-a_b-y"]), &h)); // regex の . なら通るがリテラルなので落ちる
}

#[test]
fn substring_applicator_strategy_is_live() {
    let a = substring_applicator("NAME");
    assert_eq!(a.strategy, ApplyStrategy::Live);
}

#[test]
fn substring_applicator_help_dialog_id_is_none() {
    let a = substring_applicator("NAME");
    assert_eq!(a.help_dialog_id, None);
}

#[test]
fn substring_applicator_on_apply_is_none() {
    let a = substring_applicator("NAME");
    assert!(a.on_apply.is_none());
}
```

注: `a.parser.closure` は `define_callback!` が生成する内部フィールド (`Rc<dyn Fn>`)。`(closure)(args)` の形で呼べる。

- [ ] **Step 3: テスト実行**

```bash
cargo test --bin kubetui ui::widget::table::filter_applicator::tests
```
Expected: 18 passed; 0 failed（既存 11 + 新規 7）。

- [ ] **Step 4: コミット**

```bash
git add -A
git commit -m "feat(table): add substring_applicator factory (live + OR semantics)"
```

---

## Task 5: Table の on_key_event で Live キー入力を分岐

**Files:**
- Modify: `src/ui/widget/table.rs`

- [ ] **Step 1: 既存の on_key_event を確認**

```bash
grep -n "fn on_key_event\|Mode::FilterInput" src/ui/widget/table.rs | head
awk 'NR>=605 && NR<=650' src/ui/widget/table.rs
```
Expected: `Mode::FilterInput` 内で `KeyCode::Enter`, `KeyCode::Esc`, `_ =>` の 3 分岐。`_ =>` は `filter_form.on_key_event(ev)` を呼んで filter_items する。

- [ ] **Step 2: run_parser_and_update_state ヘルパを追加**

`impl Table<'_>` 内（既存ヘルパ `match_action` 等の近く）に追加:
```rust
/// 現在の filter_form 入力を parser に渡し、結果で filter_state / filter_error を
/// 更新する。
///
/// 成功時は (新 predicate, true)、失敗時は (None, false)。
/// Live モードでは毎キー、EnterToConfirm モードでは Enter 時に呼ぶ。
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
            // filter_state は変更しない（前回成功状態が残っても、render は
            // filter_error 優先で行を隠す）
            None
        }
    }
}
```

- [ ] **Step 3: Mode::FilterInput の `_ =>` 分岐を更新**

既存:
```rust
_ => {
    let ev = if let Some(filter_form) = self.filter_form.as_mut() {
        filter_form.on_key_event(ev)
    } else {
        EventResult::Ignore
    };

    self.filter_items();

    return ev;
}
```

これを以下に置き換え:
```rust
_ => {
    let result = if let Some(filter_form) = self.filter_form.as_mut() {
        filter_form.on_key_event(ev)
    } else {
        EventResult::Ignore
    };

    // Live strategy: 毎キーで parse → state/error を更新
    if let Some(applicator) = self.filter_applicator.as_ref() {
        if applicator.strategy == ApplyStrategy::Live {
            self.run_parser_and_update_state();
        }
    }

    self.filter_items();

    return result;
}
```

- [ ] **Step 4: ビルド・全テスト**

```bash
cargo build && cargo test
```
Expected: green。filter_state は Live モードで更新されるが、行絞り込みはまだ未反映（Task 7 で `filter_items` を新パス経由に置き換える）。

- [ ] **Step 5: コミット**

```bash
git add -A
git commit -m "feat(table): dispatch parser on key input under Live strategy"
```

---

## Task 6: Table の Enter で EnterToConfirm パース + on_apply 呼び出し

**Files:**
- Modify: `src/ui/widget/table.rs`

- [ ] **Step 1: on_filter_apply_callback ヘルパを追加**

`impl Table<'_>` 内、`on_select_callback` の隣に追加:
```rust
/// 直近 parse 成功した predicate と applicator の on_apply を捕捉して、
/// Window 渡しの Callback に詰めて返す。
fn on_filter_apply_callback(
    &self,
    predicate: TableFilterPredicate,
) -> Option<Callback> {
    let on_apply = self.filter_applicator.as_ref()?.on_apply.clone()?;
    Some(Callback::from(move |w: &mut Window| {
        (on_apply.closure)(&predicate, w);
        EventResult::Nop
    }))
}
```

注: `Callback::from(closure)` は既存 `Callback` 型が `From<F>` を持つはず（`define_callback!` の expand 内で `impl<T: ...Fn> From<T> for $cb_name` が定義されている、Task の Edit でマクロを確認）。

- [ ] **Step 2: Mode::FilterInput の Enter ハンドラを差し替え**

既存:
```rust
KeyCode::Enter => {
    self.mode.filter_confirm();
}
```

これを以下に置き換え:
```rust
KeyCode::Enter => {
    // EnterToConfirm 戦略では Enter で初めて parser を呼ぶ。
    // Live 戦略では既にタイプ中に state が更新されているが、parse を
    // 再走させてエラー状態を最終確定する。
    let parsed = self.run_parser_and_update_state();

    // パース失敗時は FilterInput モード継続（filter_error が立っている）
    if self.filter_error.is_some() {
        return EventResult::Nop;
    }

    self.mode.filter_confirm();

    // 成功時は applicator の on_apply 副作用を Window 経由で呼ぶ。
    if let Some(predicate) = parsed {
        if let Some(cb) = self.on_filter_apply_callback(predicate) {
            return EventResult::Callback(cb);
        }
    }
}
```

- [ ] **Step 3: ビルド・全テスト**

```bash
cargo build && cargo test
```
Expected: green。filter_state は更新されるが行絞り込みはまだ未反映。

- [ ] **Step 4: コミット**

```bash
git add -A
git commit -m "feat(table): parse on Enter, dispatch on_apply via callback"
```

---

## Task 7: filter_state ベースの行フィルタリング（filtered_word を置換）

**Files:**
- Modify: `src/ui/widget/table.rs`, `src/ui/widget/table/item.rs`

- [ ] **Step 1: InnerItem に新しい apply_filter API を追加**

`src/ui/widget/table/item.rs` の `impl InnerItem<'_>` に追加:
```rust
/// 外部から渡された predicate で original_items を filtered_items に
/// 絞り込む。filter_state ベースの新パスで使う。
pub fn apply_filter<F>(&mut self, mut predicate: F)
where
    F: FnMut(&TableItem) -> bool,
{
    self.filtered_items = self
        .original_items
        .iter()
        .filter(|i| predicate(i))
        .cloned()
        .collect();
    self.inner_update_rendered_items();
}
```

- [ ] **Step 2: Table::filter_items を新パス経由に置き換え**

`src/ui/widget/table.rs` の `fn filter_items(&mut self)`:
```rust
fn filter_items(&mut self) {
    let old_len = self.items.len();
    let header = self.items.header().original().to_vec();
    let state = self.filter_state.clone();

    self.items.apply_filter(|item| {
        state.as_ref().map(|p| p.matches(item, &header)).unwrap_or(true)
    });

    self.adjust_selected(old_len, self.items.len());
    self.update_row_bounds();
}
```

- [ ] **Step 3: Table::update_header_and_rows も新パス経由に**

`fn update_header_and_rows(&mut self, ...)` 内の以下を:
```rust
let word = self.filter_word();
self.items.update_filter(word);
```

これを以下に置き換え:
```rust
let header = self.items.header().original().to_vec();
let state = self.filter_state.clone();
self.items.apply_filter(|item| {
    state.as_ref().map(|p| p.matches(item, &header)).unwrap_or(true)
});
```

- [ ] **Step 4: filter_word ヘルパは削除**

`fn filter_word(&self) -> String` を `src/ui/widget/table.rs` から削除。

- [ ] **Step 5: ビルド・全テスト**

```bash
cargo build 2>&1 | grep -E "^error" | head
cargo test
```
Expected: error 0。既存タブは `filter_applicator` 未設定 → filter_state も None → 全行 pass。挙動同じ。全テスト green。

- [ ] **Step 6: コミット**

```bash
git add -A
git commit -m "feat(table): filter rows via filter_state predicate (replacing filtered_word path)"
```

---

## Task 8: filter_error の render（テーブル本体置換）

**Files:**
- Modify: `src/ui/widget/table.rs`

- [ ] **Step 1: render_widget_error のシグネチャを確認**

```bash
grep -n "pub fn render_widget_error" src/ui/widget/error.rs
sed -n "$(grep -n 'pub fn render_widget_error' src/ui/widget/error.rs | head -1 | cut -d: -f1),+10p" src/ui/widget/error.rs
```
Expected: シグネチャ `pub fn render_widget_error(f: &mut Frame, chunk: Rect, block: Block, lines: &[String], theme: &ErrorTheme)`（要確認）。Theme 型と引数順を把握する。

- [ ] **Step 2: render に filter_error 分岐を追加**

`fn render(&mut self, f: &mut Frame<'_>, is_active: bool, is_mouse_over: bool)` 内、既存の `let chunk = self.chunk();` の **直後** に挿入:
```rust
// filter_error は粘着エラー。Tab 階層の widget_error は Tab.render が
// 先にチェックしてここ自体が呼ばれない（widget_error > filter_error）。
if let Some(err) = self.filter_error.as_ref() {
    let lines = vec![err.clone()];
    let error_theme = crate::config::theme::ErrorTheme::default();
    let error_theme_resolved: crate::ui::widget::error::ErrorTheme =
        error_theme.into();
    crate::ui::widget::error::render_widget_error(
        f,
        chunk,
        block.clone(),
        &lines,
        &error_theme_resolved,
    );

    // filter_form は引き続き描画（ユーザーが入力を直せるよう）
    match self.mode {
        Mode::Normal => {}
        Mode::FilterInput | Mode::FilterConfirm => {
            if let Some(filter_form) = self.filter_form.as_mut() {
                filter_form.render(f, self.mode.is_filter_input() && is_active, false);
            }
        }
    }
    return;
}
```

注: `ErrorTheme::default()` で OK か、または Table が `theme: TableTheme` に error 用フィールドを持つべきかは実装時判断。今は最小実装として `default()` で進める。実装時にコンパイルエラーが出れば調整。

- [ ] **Step 3: 単体テスト（filter_error 表示）**

`src/ui/widget/table.rs` のテスト mod 内 `mod カラムの幅` の隣に追加:
```rust
mod filter_error_render {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn filter_error_replaces_table_body() {
        let backend = TestBackend::new(40, 6);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut table = Table::builder()
            .header(["NAME".to_string(), "STATUS".to_string()])
            .items([TableItem::new(
                vec!["node-a".to_string(), "Ready".to_string()],
                None,
            )])
            .build();
        table.filter_error = Some("invalid regex 'foo['".to_string());
        table.update_chunk(Rect::new(0, 0, 40, 6));

        terminal
            .draw(|f| table.render(f, true, false))
            .unwrap();

        let buffer = terminal.backend().buffer().clone();
        let dump: String = (0..buffer.area.height)
            .flat_map(|y| (0..buffer.area.width).map(move |x| buffer[(x, y)].symbol().to_string()))
            .collect();

        assert!(
            dump.contains("invalid regex"),
            "error text should be rendered: {}",
            dump
        );
        assert!(
            !dump.contains("node-a"),
            "rows should NOT be rendered when filter_error is set: {}",
            dump
        );
    }
}
```

- [ ] **Step 4: テスト実行**

```bash
cargo test --bin kubetui ui::widget::table::tests::filter_error_render
```
Expected: 1 passed; 0 failed。

- [ ] **Step 5: 全テスト**

```bash
cargo test
```
Expected: green。

- [ ] **Step 6: コミット**

```bash
git add -A
git commit -m "feat(table): render filter_error as body-replacement (filter_form stays)"
```

---

## Task 9: 入力中の `?`/`help` でヘルプダイアログを開く

**Files:**
- Modify: `src/ui/widget/table.rs`

- [ ] **Step 1: ヘルプ判定ヘルパを追加**

`impl Table<'_>` 内に追加:
```rust
/// 現在の入力 + 押下キーがヘルプトリガーになるかを判定。
/// applicator が help_dialog_id を持ち、確定後の文字列が "?" または
/// "help" と完全一致する場合に Some(help_id) を返す。
fn would_be_help_command(&self, ev: KeyEvent) -> Option<&'static str> {
    let help_id = self.filter_applicator.as_ref()?.help_dialog_id?;
    let current = self
        .filter_form
        .as_ref()
        .map(|f| f.content())
        .unwrap_or_default();
    let typed = match key_event_to_code(ev) {
        KeyCode::Char(c) => c,
        _ => return None,
    };
    let pending = format!("{}{}", current, typed);
    if pending == "?" || pending == "help" {
        Some(help_id)
    } else {
        None
    }
}
```

- [ ] **Step 2: Mode::FilterInput の `_ =>` 分岐の冒頭にヘルプ判定**

Task 5 で書き換えた `_ =>` の冒頭、`filter_form.on_key_event(ev)` を呼ぶ **前** に挿入:
```rust
// `?` または `help` 入力でヘルプダイアログを開く（applicator が
// help_dialog_id を持つ場合のみ）。Pod log query の慣習に合わせる。
if let Some(help_id) = self.would_be_help_command(ev) {
    if let Some(filter_form) = self.filter_form.as_mut() {
        filter_form.clear();
    }
    self.mode.normal();
    return EventResult::Callback(Callback::from(move |w: &mut Window| {
        w.open_dialog(help_id);
        EventResult::Nop
    }));
}
```

- [ ] **Step 3: 動作確認テスト**

`src/ui/widget/table.rs` のテスト mod に追加:
```rust
mod help_dispatch {
    use super::*;

    fn dummy_applicator_with_help() -> TableFilterApplicator {
        let parser: TableFilterParser =
            (|_input: &str| Ok(TableFilterPredicate::default())).into();
        TableFilterApplicator::new(parser, ApplyStrategy::EnterToConfirm)
            .with_help_dialog("test-help-dialog")
    }

    #[test]
    fn typing_question_mark_returns_help_callback() {
        let mut table = Table::builder()
            .filter_form(FilterForm::builder().build())
            .filter_applicator(dummy_applicator_with_help())
            .build();
        // FilterInput モードへ
        let _ = table.on_key_event(KeyEvent::from(KeyCode::Char('/')));
        // `?` を打つ
        let result = table.on_key_event(KeyEvent::from(KeyCode::Char('?')));

        assert!(matches!(result, EventResult::Callback(_)));
    }

    #[test]
    fn typing_normal_char_does_not_open_help() {
        let mut table = Table::builder()
            .filter_form(FilterForm::builder().build())
            .filter_applicator(dummy_applicator_with_help())
            .build();
        let _ = table.on_key_event(KeyEvent::from(KeyCode::Char('/')));
        let result = table.on_key_event(KeyEvent::from(KeyCode::Char('n')));

        // n を打っても help_callback は返さない
        assert!(!matches!(result, EventResult::Callback(_)));
    }

    #[test]
    fn help_does_not_open_without_help_dialog_id() {
        // help_dialog_id を持たない applicator
        let parser: TableFilterParser =
            (|_input: &str| Ok(TableFilterPredicate::default())).into();
        let applicator = TableFilterApplicator::new(parser, ApplyStrategy::Live);

        let mut table = Table::builder()
            .filter_form(FilterForm::builder().build())
            .filter_applicator(applicator)
            .build();
        let _ = table.on_key_event(KeyEvent::from(KeyCode::Char('/')));
        let result = table.on_key_event(KeyEvent::from(KeyCode::Char('?')));

        assert!(!matches!(result, EventResult::Callback(_)));
    }
}
```

- [ ] **Step 4: テスト・ビルド**

```bash
cargo test --bin kubetui ui::widget::table::tests::help_dispatch
cargo build && cargo test
```
Expected: 3 passed + 全 test pass。

- [ ] **Step 5: コミット**

```bash
git add -A
git commit -m "feat(table): open applicator-declared help dialog on ?/help input"
```

---

## Task 10: 既存タブ一斉移行（filtered_key → filter_applicator(substring_applicator)）

**Files:** 各タブの widget 生成ファイル

- [ ] **Step 1: substring_applicator の re-export を src/ui/widget.rs に追加**

```bash
grep -n "substring_applicator" src/ui/widget.rs
```
無ければ、`src/ui/widget.rs` の既存の `pub use table::...` 行付近に追加:
```rust
pub use table::substring_applicator;
```
（Task 3 で `table.rs` 側に `pub use filter_applicator::substring_applicator` を入れてあれば、ここは `pub use table::substring_applicator;` で連鎖再エクスポート。確認のうえ最小限の修正に。）

- [ ] **Step 2: Pod tab を移行**

`src/features/pod/view/widgets/pod.rs`、`Table::builder()` チェーン内:
```bash
grep -n '\.filtered_key' src/features/pod/view/widgets/pod.rs
```
該当行を:
```rust
// Before:
.filtered_key("NAME")

// After:
.filter_applicator(crate::ui::widget::substring_applicator("NAME"))
```

ビルド確認:
```bash
cargo build 2>&1 | grep -E "^error" | head
```
Expected: error 0。

- [ ] **Step 3: Config tab を移行**

`src/features/config/view/widgets/config.rs`、同様置換。ビルド確認。

- [ ] **Step 4: Network tab を移行**

`src/features/network/view/widgets/network.rs`、同様置換。ビルド確認。

- [ ] **Step 5: API dialog を移行**

`src/features/api_resources/view/dialog.rs`、同様置換。ビルド確認。

- [ ] **Step 6: Yaml name dialog を移行**

`src/features/yaml/view/dialogs/name.rs`、同様置換。ビルド確認。

- [ ] **Step 7: Yaml kind dialog を移行**

`src/features/yaml/view/dialogs/kind.rs`、同様置換。ビルド確認。

- [ ] **Step 8: Context dialog を移行**

`src/features/context/view/dialog.rs`、同様置換。ビルド確認。

- [ ] **Step 9: Single namespace dialog を移行**

`src/features/namespace/view/single_namespace_dialog.rs`、同様置換。ビルド確認。

- [ ] **Step 10: Multiple namespaces dialog を移行**

`src/features/namespace/view/multiple_namespaces_dialog.rs`、同様置換。ビルド確認。

- [ ] **Step 11: 旧 `.filtered_key(` 残骸ゼロ確認**

```bash
grep -rn '\.filtered_key' src/features/ src/ui/
```
Expected: 0 件（テストコード除く。テストコードに残っていれば Task 11 で削除）。

- [ ] **Step 12: 全テスト**

```bash
cargo test
```
Expected: green。挙動は等価（substring + OR + live）なので既存タブ別テストが通る。

- [ ] **Step 13: コミット**

```bash
git add -A
git commit -m "refactor: migrate all existing tabs to substring_applicator

Behavior preserved: live + single-column substring with OR semantics for
space-separated patterns. Sites updated: pod, config, network, api dialog,
yaml name/kind, context, namespace single/multiple."
```

---

## Task 11: 旧コード削除（filtered_key, filtered_word, 関連 InnerItem コード）

**Files:**
- Modify: `src/ui/widget/table.rs`, `src/ui/widget/table/item.rs`

- [ ] **Step 1: Table 構造体から filtered_key を削除**

`src/ui/widget/table.rs`:
- `pub struct TableBuilder` の `filtered_key: String,` フィールド削除
- `impl TableBuilder` の `pub fn filtered_key(...)` setter 削除
- `pub struct Table<'a>` の `filtered_key: String,` フィールド削除
- `fn build(self)` 内、Table 初期化の `filtered_key: self.filtered_key.clone(),` 削除、`InnerItem::builder().filtered_key(self.filtered_key)` 呼び出しの `.filtered_key(...)` 削除
- `fn clear(&mut self)` 内、`InnerItem::builder()...filtered_key(self.filtered_key.clone())...` の `.filtered_key(...)` 削除
- `fn update_header_and_rows(&mut self, ...)` 内、`InnerItem::builder()...filtered_key(self.filtered_key.clone())...` の `.filtered_key(...)` 削除

- [ ] **Step 2: InnerItem から filtered_word / filtered_key / filtered_index / update_filter / inner_filter_items を削除**

`src/ui/widget/table/item.rs`:
- `InnerItemBuilder` から `filtered_key: String,` フィールドと `pub fn filtered_key()` setter 削除
- `InnerItemBuilder::build` 内、InnerItem 初期化の `filtered_key: self.filtered_key,` 削除
- `pub struct InnerItem` から `filtered_word: String,` と `filtered_key: String,` フィールド削除
- `impl InnerItem<'_>` から `pub fn update_filter(...)` 削除
- `impl InnerItem<'_>` から `fn inner_filter_items(...)` 削除
- `impl InnerItem<'_>` から `fn filtered_index(...)` 削除
- `pub fn update_items` 内、`self.inner_filter_items()` 呼び出しを `self.filtered_items = self.original_items.clone()` に置き換え
  - 理由: update_filter ベースの「現在のフィルタを再適用」相当は、新パスでは `Table::filter_items` が外部から `apply_filter` を呼ぶことで実現される。`update_items` 直後は filtered_items を素のコピーにし、その後 Table 側で `apply_filter` を呼び直す前提。
  - もしくは `update_items` のシグネチャを引数で predicate を取るよう変更（よりクリーン）。Task 11 の Step 6 で要選択。
  - 暫定: `self.filtered_items = self.original_items.clone();` のシンプル化、Table 側で次の filter_items を呼ぶ。

- [ ] **Step 3: filtered_index 関連テストを削除**

`src/ui/widget/table/item.rs`:
- `mod tests` 内 `mod filtered_index { ... }` 全体を削除

- [ ] **Step 4: Table 側の関連テスト更新**

`src/ui/widget/table.rs` の `mod tests`:
- 既存テストで `filtered_key` を使っていないか確認:
```bash
grep -n 'filtered_key' src/ui/widget/table.rs
```
- ヒットした箇所は削除（テストコード内も含めて）。
- `mod filter_form_option` の既存テスト（PR #980 で追加）が `.filter_form(FilterForm::builder().build())` を使っているので影響なし、引き続き pass。

- [ ] **Step 5: update_items 呼び出し側の補正**

`update_items` 呼び出し箇所（Table 側）で、呼出後に `filter_items` を続けて呼ぶ:
```bash
grep -n 'update_items' src/ui/widget/table.rs
```
- `update_widget_item` 内で `self.items.update_items(items.table())` の直後に `self.filter_items()` を呼ぶ（既存挙動: 新しい items が来たら現フィルタを再適用）。

- [ ] **Step 6: ビルド・全テスト**

```bash
cargo build 2>&1 | grep -E "^error|^warning: unused" | head
cargo test
```
Expected: error 0、unused warning 0（旧コードゼロ参照）、全テスト green。

- [ ] **Step 7: コミット**

```bash
git add -A
git commit -m "refactor(table): remove filtered_key/filtered_word legacy filter path

All filtering now goes through filter_applicator -> filter_state ->
TableFilterPredicate::matches. The if/else between old and new paths is
eliminated. update_items now resets filtered_items to original; the
caller (Table::update_widget_item) re-runs filter_items() to apply
filter_state."
```

---

## Task 12: 仕上げ（fmt / clippy / 全テスト ＋ push 準備）

**Files:** 全変更ファイル

- [ ] **Step 1: fmt**

```bash
cargo +nightly fmt
git diff --stat
```
Expected: 差分があれば formatter による軽微な調整のみ。

- [ ] **Step 2: clippy**

```bash
cargo clippy --all-targets 2>&1 | grep -E "warning:|^error" | grep -v "generated" | head
```
Expected: 新規 warning なし（pre-existing のみ）。新規 warning があれば直す。

- [ ] **Step 3: 全テスト**

```bash
cargo test 2>&1 | grep -E "test result:|^error"
```
Expected: 全 PASS。

- [ ] **Step 4: fmt 差分があればコミット**

```bash
git add -A
git status --short
# 差分があれば:
git commit -m "chore: cargo +nightly fmt"
```

- [ ] **Step 5: push 準備（user 承認後に push + PR 作成）**

実装完了時点でいったん停止。user 承認後に:
```bash
git push -u origin feat/table-filter-applicator
```

- [ ] **Step 6: PR 作成は user 承認後**

PR 本文の骨子:
- **base**: `feat/table-optional-filter`（PR #980 へスタック）
- **title**: `feat(table): pluggable filter applicator (column-OR / cross-AND semantics)`
- **body**: spec ファイル `docs/superpowers/specs/2026-05-27-table-filter-redesign.md` への参照、PR A スコープ（widget 拡張 + 既存タブ移行 + 旧コード削除）を明記、PR B（Node 実装）が続くこと、検証結果（test count、clippy 0 new warnings、既存タブ挙動完全互換）を記載

---

## Self-review

書き終わって spec と照合:

**1. Spec coverage**:
- `TableFilterPredicate` struct → Task 1 ✓
- `matches()` 列内 OR・列間 AND・exclude OR → Task 1 のテスト 5 ケース + 4 ANSI ケースで網羅 ✓
- `cell_of()` ANSI 除去 → Task 1 ✓
- `TableFilterApplicator` / `ApplyStrategy` / `TableFilterParser` / `OnFilterApply` → Task 2 ✓
- Table widget の filter_applicator / filter_state / filter_error フィールド → Task 3 ✓
- builder method → Task 3 ✓
- Live キー入力 dispatch → Task 5 ✓
- EnterToConfirm + on_apply → Task 6 ✓
- filter_state ベース行絞り込み → Task 7 ✓
- filter_error render → Task 8 ✓
- `?`/`help` → Task 9 ✓
- substring_applicator → Task 4 ✓
- 既存タブ移行（9 サイト）→ Task 10 ✓
- 旧 filtered_key/filtered_word 削除 → Task 11 ✓
- 仕上げ → Task 12 ✓

**2. Placeholder スキャン**:
- "TBD" / "TODO" なし
- 各タスクに具体的なコード or 具体的なコマンドあり
- "Similar to Task N" 表現なし
- 型・関数名は一貫（TableFilterPredicate, ApplyStrategy::Live/EnterToConfirm, substring_applicator, cell_of）

**3. 型整合性**:
- `TableFilterPredicate` の field 名（column_includes / column_excludes / label_selector / raw）は Task 1 と Task 4 で一致 ✓
- `parser.closure` 直接アクセス: `define_callback!` の生成形に依存。Task 4/5 で一貫して `(callback.closure)(args)` 呼び出し。実装時に macro 展開を確認 ✓
- `Callback::from` の使用: Task 6/9 で一貫。`define_callback!` の `impl<T: Fn> From<T> for $cb_name` を活用 ✓
- `cell_of` の戻り型は `Option<String>`（ANSI 除去で allocate）で一貫 ✓
- `render_widget_error` のシグネチャは Task 8 Step 1 で確認ステップ込み ✓
- `update_items` の挙動変更（filtered_items = original_items.clone()）と、呼び出し側の補正は Task 11 Step 5 で明示 ✓
