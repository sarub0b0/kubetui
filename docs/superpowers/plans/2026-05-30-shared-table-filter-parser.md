# 共有 column-aware フィルタパーサコア抽出 実装計画 (B-0)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** column-aware フィルタパーサの汎用部分（トークナイザ・quoting・`Term`・`parse_token`・述語組み立て）を共有モジュールに抽出し、Node を載せ替える（挙動不変）。

**Architecture:** 新規 `src/ui/widget/table/filter_parser.rs` に汎用 `parse_table_filter(input, validate_column: impl Fn(&str)->Result<(),String>)` と nom トークナイザを置く。Node の `parse_node_filter` は「`valid_columns` で列検証する closure を渡すだけ」の薄いラッパになる。タブ固有なのは列検証のみ。Pod・Config・Network は本 PR では不変。

**Tech Stack:** Rust 2021、nom、regex、strum、`#[cfg(test)]` インラインテスト、pretty_assertions。

参照 spec: `docs/superpowers/specs/2026-05-29-shared-table-filter-parser-design.md`
注: kubetui は binary crate。テストは `cargo test <path>` / `cargo test --all`（`--lib` は不可）。

---

## ファイル構成

- Create: `src/ui/widget/table/filter_parser.rs` — quoting/`Term`/`parse_token`（Node から移設）＋汎用 `parse_table_filter`＋単体テスト。
- Modify: `src/ui/widget/table.rs` — `mod filter_parser;` ＋ `parse_table_filter` を `crate::ui::widget` から使えるよう再エクスポート（`normalize_column_name` と同じ経路）。
- Modify: `src/features/node/filter/parser.rs` — 移設分を削除、`parse_node_filter` をラッパ化、import 整理。`valid_columns` とテストは維持。

---

## Task 1: 共有モジュール `filter_parser.rs` を新規作成

この時点では Node 側は変更しない（一時的にトークナイザが2箇所に存在するが、両方ともコンパイル可能・Node は従来コードを使用）。

**Files:**
- Create: `src/ui/widget/table/filter_parser.rs`
- Modify: `src/ui/widget/table.rs`（`mod filter_parser;` ＋ 再エクスポート）

- [ ] **Step 1: 新規ファイルにトークナイザを移設（verbatim コピー）＋`parse_table_filter` を実装**

`src/ui/widget/table/filter_parser.rs` を新規作成し、先頭に次の import とドックコメントを置く:

```rust
//! Shared column-aware filter parser core.
//!
//! Tokenizer + quoting + `Term` + `parse_table_filter`, used by every
//! column-aware tab filter (Node now; Pod/Config/Network later). The only
//! tab-specific part is the `validate_column` closure passed by the caller.

use std::borrow::Cow;
use std::collections::HashMap;

use nom::{
    branch::alt,
    bytes::complete::{is_not, tag},
    character::complete::{anychar, char, multispace0, multispace1},
    combinator::{map, value, verify},
    error::{ContextError, ParseError},
    multi::{fold_many0, separated_list0},
    sequence::{delimited, preceded},
    IResult,
    Parser,
};
use regex::Regex;

use crate::ui::widget::{normalize_column_name, TableFilterPredicate};
```

続けて、`src/features/node/filter/parser.rs` から次の項目を **verbatim（一字一句そのまま）コピー**する:
- `quoted`（現 node parser.rs 33-99 行）
- `unquoted`（103-109 行）
- `value_string`（112-116 行）
- `column_name`（119-123 行）
- `enum Term`（130-140 行、`#[derive(Debug)]` 付き）
- `parse_token`（151-208 行）

（コメント行 `// Quoting helpers (copied from pod/kube/filter/parser.rs ...)` 等は適宜残してよい。これらは Node 側から後の Task 2 で削除する。）

その後に、新規の汎用パース関数を追加する:

```rust
/// Parse a column-aware filter string into a `TableFilterPredicate`.
///
/// `validate_column` is called with each `COL:`/`!COL:` column token (already
/// lowercased by the tokenizer) and returns `Ok(())` if acceptable, or
/// `Err(message)` to abort parsing with that message — the only tab-specific
/// part. Stored predicate keys are normalized via `normalize_column_name`.
/// Bare values map to the `name` include; `label:` is captured verbatim (last
/// wins); values may be quoted with the log-query escape rules.
pub fn parse_table_filter(
    input: &str,
    validate_column: impl Fn(&str) -> Result<(), String>,
) -> Result<TableFilterPredicate, String> {
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
                validate_column(&column)?;
                let rx =
                    Regex::new(&value).map_err(|e| format!("invalid regex '{}': {}", value, e))?;
                column_includes
                    .entry(normalize_column_name(&column))
                    .or_default()
                    .push(rx);
            }
            Term::Exclude { column, value } => {
                validate_column(&column)?;
                let rx =
                    Regex::new(&value).map_err(|e| format!("invalid regex '{}': {}", value, e))?;
                column_excludes
                    .entry(normalize_column_name(&column))
                    .or_default()
                    .push(rx);
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

- [ ] **Step 2: `mod filter_parser;` 宣言と再エクスポートを追加**

`src/ui/widget/table.rs` の先頭付近、既存の `mod filter_applicator;` の隣に `mod filter_parser;` を追加する。そして `normalize_column_name` が `crate::ui::widget::normalize_column_name` として使える経路と同じ形で、`parse_table_filter` を再エクスポートする。具体的には、table.rs に既にある `pub use filter_applicator::{ … };` の近くに次を追加する:

```rust
pub use filter_parser::parse_table_filter;
```

`crate::ui::widget` から `parse_table_filter` を import できることを次の grep/ビルドで確認する。`src/ui/widget.rs`（または `src/ui/widget/mod.rs`）が table モジュールの pub アイテムを再エクスポートしている（`normalize_column_name` がそこ経由で届いている）。同じ仕組みで `parse_table_filter` も届くはず。もし届かなければ、`normalize_column_name` を再エクスポートしている行に `parse_table_filter` を追記する。

- [ ] **Step 3: `parse_table_filter` の単体テストを追加**

`src/ui/widget/table/filter_parser.rs` の末尾に追加する。`validate_column` には「全許可」と「特定列を弾く」の2種クロージャを使う。

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn allow_all(_: &str) -> Result<(), String> {
        Ok(())
    }

    #[test]
    fn empty_input_yields_empty_predicate() {
        let p = parse_table_filter("", allow_all).unwrap();
        assert!(p.column_includes.is_empty());
        assert!(p.column_excludes.is_empty());
        assert_eq!(p.label_selector, None);
        assert_eq!(p.raw, "");
    }

    #[test]
    fn bare_value_becomes_name_include() {
        let p = parse_table_filter("worker", allow_all).unwrap();
        assert!(p.column_includes.contains_key("name"));
    }

    #[test]
    fn include_and_exclude_store_normalized_keys() {
        let p = parse_table_filter("Status:Ready !INTERNAL-IP:10.", allow_all).unwrap();
        assert!(p.column_includes.contains_key("status"));
        assert!(p.column_excludes.contains_key("internalip"));
    }

    #[test]
    fn label_is_captured_last_wins() {
        let p = parse_table_filter("label:a=1 label:b=2", allow_all).unwrap();
        assert_eq!(p.label_selector.as_deref(), Some("b=2"));
    }

    #[test]
    fn quoted_value_with_whitespace() {
        let p = parse_table_filter(r#"name:"foo bar""#, allow_all).unwrap();
        let patterns = p.column_includes.get("name").unwrap();
        assert!(patterns[0].is_match("foo bar"));
    }

    #[test]
    fn validate_column_error_aborts_parse() {
        let reject_status = |c: &str| {
            if c == "status" {
                Err("nope".to_string())
            } else {
                Ok(())
            }
        };
        let err = parse_table_filter("status:Ready", reject_status).unwrap_err();
        assert_eq!(err, "nope");
    }

    #[test]
    fn invalid_regex_errors() {
        let err = parse_table_filter("name:[", allow_all).unwrap_err();
        assert!(err.contains("invalid regex"), "got: {}", err);
    }
}
```

- [ ] **Step 4: ビルドとテストを実行**

Run: `cargo test ui::widget::table::filter_parser 2>&1 | tail -30`
Expected: コンパイル成功、新規テスト全て PASS。Node 側は未変更なので `cargo build` も通る。

- [ ] **Step 5: コミット**

```bash
git add src/ui/widget/table/filter_parser.rs src/ui/widget/table.rs
git commit -m "feat(table-filter): add shared parse_table_filter core (tokenizer + quoting)"
```

---

## Task 2: Node パーサを共有コアへ載せ替え＋重複削除

**Files:**
- Modify: `src/features/node/filter/parser.rs`

- [ ] **Step 1: 移設済みヘルパーを削除し import を整理**

`src/features/node/filter/parser.rs` から次を**削除**する（Task 1 で共有モジュールへ移したもの）:
- `quoted` / `unquoted` / `value_string` / `column_name` の各関数（およびその上の `// Quoting helpers ...` コメントブロック）。
- `enum Term`。
- `parse_token`。

先頭の import を次に置き換える（不要になった nom 群・`Cow`・`HashMap`・`Regex` を除去。`HashSet`/`strum`/`NodeColumn`/`NodeLabelColumn` は `valid_columns` で使用、`parse_table_filter`/`normalize_column_name`/`TableFilterPredicate` を追加）:

```rust
use std::collections::HashSet;

use strum::IntoEnumIterator;

use crate::{
    features::node::node_columns::{NodeColumn, NodeLabelColumn},
    ui::widget::{normalize_column_name, parse_table_filter, TableFilterPredicate},
};
```

- [ ] **Step 2: `parse_node_filter` をラッパ化**

`valid_columns`（builtin `NodeColumn::iter()` の display 正規化＋registry header 正規化）は**そのまま残す**。`parse_node_filter` を次に置き換える:

```rust
/// Parse a Node-filter input string into a `TableFilterPredicate`.
///
/// Column references are validated against the set of *known* columns (builtin
/// Node columns plus defined label columns in `label_registry`); an unknown
/// column produces a parse error. A known column that is not currently
/// displayed is accepted here and becomes inactive at match time. Tokenization
/// and predicate building are delegated to the shared `parse_table_filter`.
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

（`#[cfg(test)] mod tests { … }` ブロックは**一切変更しない** — これが挙動不変の証明。テストは `parse_node_filter(input, &registry)` を呼び続け、quoting/escape/未定義列エラー等を網羅している。）

- [ ] **Step 3: テストを実行（Node 挙動不変の確認）**

Run: `cargo test features::node::filter 2>&1 | tail -30`
Expected: Node parser の既存テストが**変更なしで全て PASS**。

- [ ] **Step 4: 全体ビルド・clippy**

Run: `cargo test --all 2>&1 | tail -20`
Expected: 全テスト PASS。

Run: `cargo clippy --all-targets 2>&1 | rg "filter_parser|node/filter|widget/table"`
Expected: 変更ファイルに新規警告なし（特に node parser.rs に未使用 import が残っていないこと）。

- [ ] **Step 5: コミット**

```bash
git add src/features/node/filter/parser.rs
git commit -m "refactor(node-filter): delegate to shared parse_table_filter (behavior-preserving)"
```

---

## Task 3: 全体検証

**Files:** なし（検証のみ）

- [ ] **Step 1: 全テスト**

Run: `cargo test --all 2>&1 | rg "test result:"`
Expected: 全テスト PASS。

- [ ] **Step 2: clippy**

Run: `cargo clippy --all-targets 2>&1 | tail -30`
Expected: 変更ファイル（filter_parser.rs / table.rs / node/filter/parser.rs）に新規警告なし。

- [ ] **Step 3: format**

Run: `cargo +nightly fmt --check 2>&1 | tail -30`
Expected: 差分なし。出たら `cargo +nightly fmt` を実行して再コミット。注: 既存の無関係な fmt drift があれば変更ファイルに限定して確認する。

- [ ] **Step 4:（必要なら）fmt 修正をコミット**

```bash
git add -A
git commit -m "style: cargo fmt"
```

---

## Self-Review（計画作成者によるチェック結果）

- **Spec カバレッジ:** 共有モジュール＋`parse_table_filter`（spec §1）→ Task 1。mod 宣言・再エクスポート（spec §2）→ Task 1 Step 2。Node 載せ替え＋import 整理＋valid_columns 維持（spec §3）→ Task 2。テスト（spec §4：Node 既存テスト不変＋共有モジュール単体テスト）→ Task 1 Step 3／Task 2 Step 3／Task 3。全項目に対応タスクあり。
- **プレースホルダ:** TBD/TODO なし。移設は「node parser.rs の該当関数を verbatim コピー」という一意・機械的指示（行番号付き）。新規コード（imports・`parse_table_filter`・テスト）は全文記載。
- **型整合:** `parse_table_filter(&str, impl Fn(&str)->Result<(),String>) -> Result<TableFilterPredicate, String>`（Task 1）と Node のラッパ呼び出し（Task 2 Step 2、closure が `Fn(&str)->Result<(),String>`）一致。`normalize_column_name`/`TableFilterPredicate`/`parse_table_filter` は `crate::ui::widget` 経由で両モジュールから参照。`valid_columns(&[NodeLabelColumn])->HashSet<String>`（維持）と closure 内 `valid.contains(&normalize_column_name(column))` 一致。
