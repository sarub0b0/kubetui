# Node Filter Applicator Implementation Plan (PR B)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the Node tab's column-aware filter using the `TableFilterApplicator` framework shipped in PR A, replacing the Plan 5 standalone dialog widget.

**Architecture:** Build a nom-based parser that produces `TableFilterPredicate` directly (no Node-specific predicate type). Wire `node_filter_applicator()` into `Table::builder().filter_applicator(...)`. Drop the standalone `node_filter` dialog widget. Reduce `NodeFilterMessage::Apply` payload to `Option<String>` (only `labelSelector` needs to reach the server; client-side regex matching is handled by `TableFilterPredicate::matches`). Update help dialog content for the new `<col>:<val>` syntax.

**Tech Stack:** Rust 2021, nom 7 (parser), regex, ratatui (widgets), crossbeam (channels), tokio (poller). Builds on PR A's `TableFilterApplicator` / `TableFilterPredicate` / `ApplyStrategy::EnterToConfirm` / `define_callback!(OnFilterApply)`.

**Spec:** `docs/superpowers/specs/2026-05-27-table-filter-redesign.md` §Node タブのフィルタ実装 (lines 305–371).

**Reference implementation (NOT cherry-picked; copy logic only):** `920-node-filter` branch, Plan 5 commits:
- `f64f9483` feat(node): NodeFilter AST and nom-based parser
- `dabeda86` feat(node): thread SharedNodeFilter through controller and poller
- `3692753a` feat(node): apply NodeFilter in poller (labelSelector + name regex)
- `a0a257cb` feat(node): NodeFilterMessage::Apply and controller handler
- `b5351dd9` feat(node): add node_filter input dialog (/ to open, Enter to apply)
- `871abe79` feat(node): add node_filter help dialog (type ? in filter input)
- `19dfe5c6` feat(node): show current filter and match counts in table title
- `9e55a7e6` chore(node): allow dead_code on NodeFilter::is_empty

These are NOT cherry-picked. The plan reproduces the relevant logic as fresh commits adapted to the PR A framework so the resulting PR has a clean, focused diff.

---

## Prerequisites and branch state

PR B depends on two parallel stacks both reaching the same base:

1. **PR A (#982)** — `feat/table-filter-applicator`, which adds `TableFilterApplicator` / `TableFilterPredicate` / `substring_applicator`. Currently OPEN, stacked on `feat/table-optional-filter` (#980).
2. **Node stack** — `920-add-node-tab` (#972) → `920-node-columns-dialog` (#974) → `920-node-label-columns` (#975) → `920-node-detail-pane` (#979). All OPEN.

Plan 5 was the next layer on this stack (`920-node-filter`) but is being replaced by PR B. **PR B itself depends on the Plan 4 head AND PR A.** Practical paths to a viable branch state for execution:

- **Path X (preferred):** Both stacks merged to `main` first → PR B branches off `main`.
- **Path Y:** Create an integration branch from PR A's head (`feat/table-filter-applicator`) and merge `920-node-detail-pane` into it → PR B is built on that integration branch.

The plan assumes whichever path produced a working tree where:
- `src/features/node/{kube,view,message.rs,node_columns.rs}` exists (from Plan 1–4).
- `src/ui/widget/table/filter_applicator.rs` exports `TableFilterApplicator`, `TableFilterPredicate`, `ApplyStrategy`, `OnFilterApply`, `TableFilterParser` (from PR A).
- `Table::builder()` has `.filter_applicator(...)` and `.filter_form(...)` (from PR A).

Task 0 verifies this concretely before any code is written.

---

## File structure

### New files

- **`src/features/node/filter.rs`** — replaces Plan 5's `NodeFilter` module. Contains the `node_filter_applicator()` factory only; re-exports parser as needed.
- **`src/features/node/filter/parser.rs`** — replaces Plan 5's parser. nom-based, returns `Result<TableFilterPredicate, String>`. Knows the set of valid column names (built from `label_registry` plus the builtin column headers).

### Modified files

- **`src/features/node/view/widgets/node.rs`** — switch widget construction from `.action('/', open_node_filter_dialog())` to `.filter_form(FilterForm::default()).filter_applicator(node_filter_applicator(label_registry, tx))`. Re-introduce title block_injection that reads `TableFilterPredicate.raw` from `Table.filter_state()`.
- **`src/features/node/view/widgets/node_filter_help.rs`** — rewrite `content()` for the new `<col>:` / `!<col>:` / `label:` syntax (Plan 5 wording was `node:` / `!node:` / `label:`).
- **`src/features/node/message.rs`** — change `NodeFilterMessage::Apply` payload from `Option<NodeFilter>` to `Option<String>` (only `label_selector` reaches the server).
- **`src/features/node/kube/node.rs`** — drop `matches_name` client-side filtering; `SharedNodeFilter = Arc<RwLock<Option<String>>>`; URL construction takes `Option<&str>` (label selector).
- **`src/workers/kube/controller.rs`** — `NodeFilterMessage::Apply(Option<String>)` handler writes to `SharedNodeFilter`.
- **`src/features/node/view/tab.rs`** — remove standalone `node_filter` dialog registration; keep `NODE_FILTER_HELP_DIALOG_ID` registration. Thread `label_registry` and `tx` into `node_widget()`.

### Deleted files

- **`src/features/node/view/widgets/node_filter.rs`** — the standalone dialog widget. Its UX role is taken over by the inline `FilterForm` on the Table widget.

### Component ID changes (`src/features/component_id.rs`)

- Remove `NODE_FILTER_WIDGET_ID`.
- Keep `NODE_FILTER_HELP_DIALOG_ID` (the help dialog is dispatched via `TableFilterApplicator::with_help_dialog(NODE_FILTER_HELP_DIALOG_ID)`).

---

## Parser syntax (recap from spec)

| Input | Effect |
|---|---|
| `foo` | bare value → `NAME` column include (regex `foo`) |
| `foo bar` | two bare values → `NAME` includes `foo` OR `bar` (column-internal OR) |
| `NAME:gke.*worker` | `NAME` include (regex) |
| `STATUS:Ready` | `STATUS` include |
| `STATUS:Ready STATUS:Pending` | same column → OR |
| `STATUS:Ready NAME:nginx` | cross-column → AND |
| `!NS:kube-system` | `NAMESPACE` exclude |
| `label:role=worker` | server-side labelSelector (last-wins if multiple) |

Column names: case-insensitive; canonical form is lowercase (stored as lowercase in `TableFilterPredicate::column_includes` keys). `cell_of` in PR A already case-insensitively matches column headers.

Unknown column name → **parse error** (e.g. `STATUSU:Ready` → `"unknown column 'STATUSU'"`). Validation set = builtin column display names + `label_registry` headers, all lowercased.

---

## Tasks

### Task 0: Verify branch prerequisites

**Purpose:** Confirm the working tree has both PR A's framework and Plans 1–4's Node code before any new commits. Surfaces issues at minute zero rather than mid-implementation.

**Files (read-only):** working tree introspection only.

- [ ] **Step 1: Verify PR A framework presence**

Run:
```bash
test -f src/ui/widget/table/filter_applicator.rs && \
  grep -q 'pub struct TableFilterApplicator' src/ui/widget/table/filter_applicator.rs && \
  grep -q 'pub struct TableFilterPredicate' src/ui/widget/table/filter_applicator.rs && \
  grep -q 'pub enum ApplyStrategy' src/ui/widget/table/filter_applicator.rs && \
  echo "PR A: OK" || echo "PR A: MISSING"
```
Expected: `PR A: OK`. If `MISSING`, stop and merge/integrate PR A first.

- [ ] **Step 2: Verify Plans 1–4 Node code presence**

Run:
```bash
test -d src/features/node && \
  test -f src/features/node/view/widgets/node.rs && \
  test -f src/features/node/node_columns.rs && \
  test -f src/features/node/message.rs && \
  test -f src/features/node/kube/node.rs && \
  echo "Node base: OK" || echo "Node base: MISSING"
```
Expected: `Node base: OK`. If `MISSING`, the integration branch has not been prepared.

- [ ] **Step 3: Verify NodeLabelColumn is in scope**

Run:
```bash
grep -n 'pub struct NodeLabelColumn' src/features/node/node_columns.rs
```
Expected: a line like `pub struct NodeLabelColumn` is printed. This is the type the parser will use to validate label-column names.

- [ ] **Step 4: Confirm Plan 5 artifacts to remove are still present**

Run:
```bash
ls src/features/node/filter.rs src/features/node/filter/parser.rs \
   src/features/node/view/widgets/node_filter.rs \
   src/features/node/view/widgets/node_filter_help.rs 2>&1 | head
```
- If all four exist (integrated from `920-node-filter`): the plan replaces / deletes them in later tasks.
- If only `node_filter_help.rs` exists (Plan 5 partially integrated): adjust later tasks to skip the deletes.
- If none of them exist (Plans 1–4 only, no Plan 5): later "delete" steps become no-ops; "rewrite" steps become "create from scratch."

Record observed state in a scratch note (you will need it for Tasks 5, 9, 10).

- [ ] **Step 5: Confirm baseline build is green**

Run:
```bash
cargo build 2>&1 | tail -3 && cargo test --all 2>&1 | tail -3
```
Expected: both succeed with no errors. If anything is red on the prerequisite branch, **stop and fix that first** — PR B must start from a green tree.

- [ ] **Step 6: Decide branch name and create**

Run (replace `<base>` with the actual branch you started from, e.g. the integration branch name):
```bash
git checkout -b feat/node-filter-applicator
git status
```
Expected: clean working tree, on `feat/node-filter-applicator`.

No commit at this task.

---

### Task 1: Parser skeleton — bare values default to NAME (TDD)

**Purpose:** Establish the parser's module shape and the simplest happy path (bare tokens map to `NAME` column includes). Subsequent tasks layer syntax on top.

**Files:**
- Create: `src/features/node/filter/parser.rs`
- Create: `src/features/node/filter.rs`
- Modify: `src/features/node/mod.rs` (or wherever the node module lists submodules) — add `pub mod filter;` if not present

Reference for parser shape: `920-node-filter:src/features/node/filter/parser.rs` (the AST/nom structure; the new code returns a different output type).

- [ ] **Step 1: Add `nom` dependency check**

Run:
```bash
grep -n '"nom"' Cargo.toml
```
Expected: a `nom = "..."` entry. If absent, add `nom = "7"` to `[dependencies]` in `Cargo.toml`. (Plan 5 already added it on `920-node-filter`, so on the integration branch this is usually a no-op.)

- [ ] **Step 2: Stub `src/features/node/filter.rs`**

Write the file:

```rust
//! Node tab filter: parser + `TableFilterApplicator` factory.
//!
//! The parser produces a `TableFilterPredicate` directly (no Node-specific
//! predicate type). The factory wires the parser into the Table widget's
//! filter framework with `ApplyStrategy::EnterToConfirm`, a help-dialog
//! dispatch, and an `on_apply` callback that forwards the parsed
//! `labelSelector` to the Node poller via `NodeFilterMessage::Apply`.

mod parser;

pub use parser::parse_node_filter;
```

- [ ] **Step 3: Stub `src/features/node/filter/parser.rs`**

Write the file:

```rust
use std::collections::HashMap;

use regex::Regex;

use crate::{
    features::node::node_columns::NodeLabelColumn,
    ui::widget::TableFilterPredicate,
};

/// Parse a Node-filter input string into a `TableFilterPredicate`.
///
/// `label_registry` supplies the set of valid label-column headers (in
/// addition to the builtin Node column headers). Unknown column names
/// produce a parse error.
pub fn parse_node_filter(
    input: &str,
    label_registry: &[NodeLabelColumn],
) -> Result<TableFilterPredicate, String> {
    let _ = label_registry; // used by later tasks
    let trimmed = input.trim();

    let mut column_includes: HashMap<String, Vec<Regex>> = HashMap::new();
    if !trimmed.is_empty() {
        let regexes: Result<Vec<Regex>, _> = trimmed
            .split_whitespace()
            .map(Regex::new)
            .collect();
        let regexes = regexes.map_err(|e| format!("invalid regex: {}", e))?;
        column_includes.insert("name".to_string(), regexes);
    }

    Ok(TableFilterPredicate {
        column_includes,
        column_excludes: HashMap::new(),
        label_selector: None,
        raw: trimmed.to_string(),
    })
}
```

- [ ] **Step 4: Write failing tests**

Append to `src/features/node/filter/parser.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn no_label_cols() -> Vec<NodeLabelColumn> {
        Vec::new()
    }

    #[test]
    fn empty_input_yields_empty_predicate() {
        let p = parse_node_filter("", &no_label_cols()).unwrap();
        assert!(p.column_includes.is_empty());
        assert!(p.column_excludes.is_empty());
        assert_eq!(p.label_selector, None);
        assert_eq!(p.raw, "");
    }

    #[test]
    fn whitespace_only_input_yields_empty_predicate() {
        let p = parse_node_filter("   \t  ", &no_label_cols()).unwrap();
        assert!(p.column_includes.is_empty());
        assert_eq!(p.raw, "");
    }

    #[test]
    fn single_bare_value_becomes_name_include() {
        let p = parse_node_filter("worker", &no_label_cols()).unwrap();
        assert_eq!(p.column_includes.len(), 1);
        let patterns = p.column_includes.get("name").expect("name column");
        assert_eq!(patterns.len(), 1);
        assert!(patterns[0].is_match("gke-worker-1"));
        assert!(!patterns[0].is_match("gke-control-1"));
        assert_eq!(p.raw, "worker");
    }

    #[test]
    fn multiple_bare_values_become_name_or() {
        let p = parse_node_filter("foo bar", &no_label_cols()).unwrap();
        let patterns = p.column_includes.get("name").expect("name column");
        assert_eq!(patterns.len(), 2);
        assert_eq!(p.raw, "foo bar");
    }
}
```

- [ ] **Step 5: Run tests to verify they pass against the stub**

Run:
```bash
cargo test --bin kubetui features::node::filter::parser::tests 2>&1 | tail -15
```
Expected: 4 tests pass. (The stub already handles bare values correctly.)

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml src/features/node/filter.rs src/features/node/filter/parser.rs
git status  # confirm src/features/node/mod.rs is registered if needed
git commit -m "feat(node): parser skeleton — bare values map to NAME column includes

Introduce src/features/node/filter/{,parser}.rs as the home for the new
column-aware Node filter parser. The skeleton handles empty input and
bare whitespace-separated tokens (which become regex includes on the
NAME column, matching the spec's bare-value alias). Column-prefixed
syntax (NAME:.., !COL:.., label:..) lands in subsequent tasks.

label_registry is accepted in the signature but not yet consumed; it
becomes load-bearing in Task 5 (unknown-column validation)."
```

---

### Task 2: Parser — `<col>:<val>` include syntax (TDD)

**Purpose:** Add explicit column-include syntax. Same-column repeats accumulate (OR); different columns coexist in the predicate (AND across columns is handled by `TableFilterPredicate::matches` in PR A).

**Files:**
- Modify: `src/features/node/filter/parser.rs`

- [ ] **Step 1: Write failing tests**

Append to the existing `tests` module:

```rust
    #[test]
    fn explicit_column_include_creates_column_entry() {
        let p = parse_node_filter("status:Ready", &no_label_cols()).unwrap();
        assert_eq!(p.column_includes.len(), 1);
        let patterns = p.column_includes.get("status").expect("status column");
        assert_eq!(patterns.len(), 1);
        assert!(patterns[0].is_match("Ready"));
        assert_eq!(p.raw, "status:Ready");
    }

    #[test]
    fn column_names_are_case_insensitive_canonicalized_lowercase() {
        let p = parse_node_filter("STATUS:Ready Name:worker", &no_label_cols()).unwrap();
        assert!(p.column_includes.contains_key("status"));
        assert!(p.column_includes.contains_key("name"));
    }

    #[test]
    fn same_column_includes_accumulate_in_order() {
        let p = parse_node_filter("status:Ready status:Pending", &no_label_cols()).unwrap();
        let patterns = p.column_includes.get("status").expect("status column");
        assert_eq!(patterns.len(), 2);
        assert!(patterns[0].is_match("Ready"));
        assert!(patterns[1].is_match("Pending"));
    }

    #[test]
    fn different_columns_coexist_in_predicate() {
        let p = parse_node_filter("status:Ready name:worker", &no_label_cols()).unwrap();
        assert_eq!(p.column_includes.len(), 2);
    }

    #[test]
    fn bare_and_column_includes_mix() {
        // `foo status:Ready` → NAME has `foo`, STATUS has `Ready`
        let p = parse_node_filter("foo status:Ready", &no_label_cols()).unwrap();
        assert_eq!(p.column_includes.len(), 2);
        assert_eq!(p.column_includes.get("name").unwrap().len(), 1);
        assert_eq!(p.column_includes.get("status").unwrap().len(), 1);
    }
```

- [ ] **Step 2: Run tests to confirm they fail**

Run:
```bash
cargo test --bin kubetui features::node::filter::parser::tests 2>&1 | tail -20
```
Expected: the 5 new tests fail (the stub treats every token as a NAME include and would put `status:Ready` literally into NAME).

- [ ] **Step 3: Implement column-aware tokenization**

Replace the body of `parse_node_filter` in `src/features/node/filter/parser.rs`. Add a private helper:

```rust
use std::collections::HashMap;

use regex::Regex;

use crate::{
    features::node::node_columns::NodeLabelColumn,
    ui::widget::TableFilterPredicate,
};

/// One parsed term from the input.
#[derive(Debug)]
enum Term {
    /// Bare value (no prefix) → defaults to NAME include.
    Bare(String),
    /// `<col>:<value>` include.
    Include { column: String, value: String },
}

fn parse_term(token: &str) -> Term {
    if let Some((col, val)) = token.split_once(':') {
        // Empty column or empty value is treated as Bare so the user sees
        // a regex error later (or no-op). Stricter validation happens in
        // Task 5 (column-name validation).
        if !col.is_empty() && !val.is_empty() {
            return Term::Include {
                column: col.to_lowercase(),
                value: val.to_string(),
            };
        }
    }
    Term::Bare(token.to_string())
}

pub fn parse_node_filter(
    input: &str,
    label_registry: &[NodeLabelColumn],
) -> Result<TableFilterPredicate, String> {
    let _ = label_registry; // consumed in Task 5

    let trimmed = input.trim();
    let mut column_includes: HashMap<String, Vec<Regex>> = HashMap::new();

    for token in trimmed.split_whitespace() {
        match parse_term(token) {
            Term::Bare(v) => {
                let rx = Regex::new(&v).map_err(|e| format!("invalid regex '{}': {}", v, e))?;
                column_includes.entry("name".to_string()).or_default().push(rx);
            }
            Term::Include { column, value } => {
                let rx = Regex::new(&value)
                    .map_err(|e| format!("invalid regex '{}': {}", value, e))?;
                column_includes.entry(column).or_default().push(rx);
            }
        }
    }

    Ok(TableFilterPredicate {
        column_includes,
        column_excludes: HashMap::new(),
        label_selector: None,
        raw: trimmed.to_string(),
    })
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run:
```bash
cargo test --bin kubetui features::node::filter::parser::tests 2>&1 | tail -15
```
Expected: all 9 tests pass (4 from Task 1 + 5 new).

- [ ] **Step 5: Commit**

```bash
git add src/features/node/filter/parser.rs
git commit -m "feat(node): parser — column-prefixed include syntax (COL:val)

Tokens of the form '<col>:<val>' become include entries on the named
column (column name lowercased). Same-column repeats accumulate as an
ordered Vec; cross-column entries coexist. Bare and column-prefixed
tokens can be mixed freely in the same input."
```

---

### Task 3: Parser — `!<col>:<val>` exclude syntax (TDD)

**Files:**
- Modify: `src/features/node/filter/parser.rs`

- [ ] **Step 1: Write failing tests**

Append to `tests`:

```rust
    #[test]
    fn excludes_prefixed_with_bang_populate_column_excludes() {
        let p = parse_node_filter("!ns:kube-system", &no_label_cols()).unwrap();
        assert!(p.column_includes.is_empty());
        let patterns = p.column_excludes.get("ns").expect("ns column");
        assert_eq!(patterns.len(), 1);
        assert!(patterns[0].is_match("kube-system"));
    }

    #[test]
    fn includes_and_excludes_coexist() {
        let p = parse_node_filter("status:Ready !ns:kube-system", &no_label_cols()).unwrap();
        assert_eq!(p.column_includes.len(), 1);
        assert_eq!(p.column_excludes.len(), 1);
    }

    #[test]
    fn bang_without_colon_is_treated_as_bare_value() {
        // `!worker` is NOT shorthand for `!name:worker`. The leading `!`
        // is only meaningful with an explicit column.
        let p = parse_node_filter("!worker", &no_label_cols()).unwrap();
        // The literal string `!worker` becomes a regex on NAME. The regex
        // crate accepts `!worker` as a literal match on `!worker`.
        let patterns = p.column_includes.get("name").expect("name column");
        assert_eq!(patterns.len(), 1);
        assert!(patterns[0].is_match("!worker"));
        assert!(p.column_excludes.is_empty());
    }
```

- [ ] **Step 2: Run tests to confirm they fail**

Run:
```bash
cargo test --bin kubetui features::node::filter::parser::tests 2>&1 | tail -20
```
Expected: the 3 new tests fail.

- [ ] **Step 3: Extend `parse_term` to recognize the `!` prefix**

Modify `src/features/node/filter/parser.rs`:

```rust
#[derive(Debug)]
enum Term {
    Bare(String),
    Include { column: String, value: String },
    Exclude { column: String, value: String },
}

fn parse_term(token: &str) -> Term {
    if let Some(stripped) = token.strip_prefix('!') {
        if let Some((col, val)) = stripped.split_once(':') {
            if !col.is_empty() && !val.is_empty() {
                return Term::Exclude {
                    column: col.to_lowercase(),
                    value: val.to_string(),
                };
            }
        }
        // Fall through: `!worker` without colon is a bare value.
    }

    if let Some((col, val)) = token.split_once(':') {
        if !col.is_empty() && !val.is_empty() {
            return Term::Include {
                column: col.to_lowercase(),
                value: val.to_string(),
            };
        }
    }

    Term::Bare(token.to_string())
}
```

And extend the match in `parse_node_filter`:

```rust
    for token in trimmed.split_whitespace() {
        match parse_term(token) {
            Term::Bare(v) => {
                let rx = Regex::new(&v).map_err(|e| format!("invalid regex '{}': {}", v, e))?;
                column_includes.entry("name".to_string()).or_default().push(rx);
            }
            Term::Include { column, value } => {
                let rx = Regex::new(&value)
                    .map_err(|e| format!("invalid regex '{}': {}", value, e))?;
                column_includes.entry(column).or_default().push(rx);
            }
            Term::Exclude { column, value } => {
                let rx = Regex::new(&value)
                    .map_err(|e| format!("invalid regex '{}': {}", value, e))?;
                column_excludes.entry(column).or_default().push(rx);
            }
        }
    }
```

Add `let mut column_excludes: HashMap<String, Vec<Regex>> = HashMap::new();` near the `column_includes` declaration, and pass `column_excludes` into the returned `TableFilterPredicate`.

- [ ] **Step 4: Run tests to verify they pass**

Run:
```bash
cargo test --bin kubetui features::node::filter::parser::tests 2>&1 | tail -15
```
Expected: all 12 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/features/node/filter/parser.rs
git commit -m "feat(node): parser — '!COL:val' exclude syntax

Tokens starting with '!' followed by 'COL:value' populate column_excludes.
A bare '!worker' (no colon) is treated as a literal NAME regex, not as
shorthand for '!name:worker' — the bang is only meaningful with an
explicit column prefix."
```

---

### Task 4: Parser — `label:<sel>` server-side selector with last-wins (TDD)

**Files:**
- Modify: `src/features/node/filter/parser.rs`

- [ ] **Step 1: Write failing tests**

Append to `tests`:

```rust
    #[test]
    fn label_selector_is_captured_verbatim() {
        let p = parse_node_filter("label:role=worker", &no_label_cols()).unwrap();
        assert_eq!(p.label_selector.as_deref(), Some("role=worker"));
        assert!(p.column_includes.is_empty());
        assert!(p.column_excludes.is_empty());
    }

    #[test]
    fn label_selector_supports_kubectl_comma_and() {
        let p = parse_node_filter("label:role=worker,zone=us-west", &no_label_cols()).unwrap();
        assert_eq!(
            p.label_selector.as_deref(),
            Some("role=worker,zone=us-west")
        );
    }

    #[test]
    fn multiple_label_terms_keep_the_last() {
        // The k8s API accepts only one labelSelector value; spec requires
        // last-wins to match the Pod log query convention.
        let p = parse_node_filter("label:a=1 label:b=2", &no_label_cols()).unwrap();
        assert_eq!(p.label_selector.as_deref(), Some("b=2"));
    }

    #[test]
    fn label_and_column_terms_coexist() {
        let p = parse_node_filter(
            "status:Ready label:role=worker !ns:kube-system",
            &no_label_cols(),
        )
        .unwrap();
        assert_eq!(p.column_includes.len(), 1);
        assert_eq!(p.column_excludes.len(), 1);
        assert_eq!(p.label_selector.as_deref(), Some("role=worker"));
    }
```

- [ ] **Step 2: Run tests to confirm they fail**

Run:
```bash
cargo test --bin kubetui features::node::filter::parser::tests 2>&1 | tail -20
```
Expected: 4 new tests fail (the parser currently puts `label:foo` into the `label` column as an include).

- [ ] **Step 3: Add `Label` term recognized BEFORE generic `<col>:<val>`**

Modify `parse_term` in `src/features/node/filter/parser.rs` so the `label:` case is detected first:

```rust
#[derive(Debug)]
enum Term {
    Bare(String),
    Include { column: String, value: String },
    Exclude { column: String, value: String },
    Label(String),
}

fn parse_term(token: &str) -> Term {
    if let Some(sel) = token.strip_prefix("label:") {
        if !sel.is_empty() {
            return Term::Label(sel.to_string());
        }
    }

    if let Some(stripped) = token.strip_prefix('!') {
        if let Some((col, val)) = stripped.split_once(':') {
            if !col.is_empty() && !val.is_empty() {
                return Term::Exclude {
                    column: col.to_lowercase(),
                    value: val.to_string(),
                };
            }
        }
    }

    if let Some((col, val)) = token.split_once(':') {
        if !col.is_empty() && !val.is_empty() {
            return Term::Include {
                column: col.to_lowercase(),
                value: val.to_string(),
            };
        }
    }

    Term::Bare(token.to_string())
}
```

Extend the match in `parse_node_filter` and add `let mut label_selector: Option<String> = None;`:

```rust
            Term::Label(sel) => {
                label_selector = Some(sel);
            }
```

And pass `label_selector` into the returned `TableFilterPredicate`.

- [ ] **Step 4: Run tests to verify they pass**

Run:
```bash
cargo test --bin kubetui features::node::filter::parser::tests 2>&1 | tail -15
```
Expected: all 16 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/features/node/filter/parser.rs
git commit -m "feat(node): parser — 'label:<selector>' k8s labelSelector

'label:<value>' is captured verbatim into TableFilterPredicate.label_selector
(the value is passed unchanged to the kube API as ?labelSelector=). When
multiple label: terms appear, the last value wins, matching the Pod log
query convention and the underlying k8s API which accepts only one
labelSelector value."
```

---

### Task 5: Parser — unknown column name produces parse error (TDD)

**Purpose:** Implement the column-name validation against the union of builtin Node column headers and the configured `label_registry`. Typos like `STATUSU:Ready` surface as a clear error in the Table's `filter_error` overlay.

**Files:**
- Modify: `src/features/node/filter/parser.rs`

**Background:** Valid column names = lowercased headers of every `NodeColumn` builtin (e.g. `name`, `status`, `roles`, `age`, `version`, `internal-ip`, `external-ip`, `os-image`, `kernel-version`, `container-runtime`) plus lowercased `header` of every `NodeLabelColumn` in the registry. The exact builtin list depends on Plans 1–3 of the Node stack — verify by reading `src/features/node/node_columns.rs` and reusing whatever enumerates builtin columns.

- [ ] **Step 1: Identify the builtin-column enumeration**

Run:
```bash
grep -n 'enum NodeColumn\|impl NodeColumn\|fn display' src/features/node/node_columns.rs
```
Expected: a `NodeColumn` enum with a `display()` (or similarly named) method that returns the column header. Note the method name; the next step uses it.

- [ ] **Step 2: Write failing tests**

Append to `tests` in `src/features/node/filter/parser.rs`:

```rust
    fn registry_with(name: &str, header: &str) -> Vec<NodeLabelColumn> {
        vec![NodeLabelColumn {
            name: name.to_string(),
            key: "irrelevant.example.com/key".to_string(),
            header: header.to_string(),
        }]
    }

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

    #[test]
    fn builtin_columns_are_accepted() {
        // `name` and `status` are builtin headers — must not error.
        assert!(parse_node_filter("name:n status:s", &no_label_cols()).is_ok());
    }

    #[test]
    fn registered_label_column_header_is_accepted() {
        let regs = registry_with("zone", "ZONE");
        let p = parse_node_filter("zone:us-west", &regs).unwrap();
        assert!(p.column_includes.contains_key("zone"));
    }

    #[test]
    fn label_keyword_is_not_treated_as_a_column_lookup() {
        // 'label:role=worker' must NOT trigger unknown-column validation
        // (it's the special-cased k8s labelSelector path).
        assert!(parse_node_filter("label:role=worker", &no_label_cols()).is_ok());
    }
```

- [ ] **Step 3: Run tests to confirm they fail**

Run:
```bash
cargo test --bin kubetui features::node::filter::parser::tests 2>&1 | tail -20
```
Expected: 4 new tests fail (unknown-column ones); the two acceptance tests already pass.

- [ ] **Step 4: Build the valid-column set and validate**

Modify `src/features/node/filter/parser.rs`. Add imports and a helper:

```rust
use std::collections::HashSet;

use crate::features::node::node_columns::{NodeColumn, NodeLabelColumn};
use strum::IntoEnumIterator;

fn valid_columns(label_registry: &[NodeLabelColumn]) -> HashSet<String> {
    let mut set: HashSet<String> = NodeColumn::iter()
        .map(|c| c.display().to_lowercase())
        .collect();
    for lc in label_registry {
        set.insert(lc.header.to_lowercase());
    }
    set
}
```

(If the actual builtin-display method is not called `display()`, substitute the real name observed in Step 1. If `NodeColumn` does not implement `IntoEnumIterator` via `strum`, look for an explicit `pub const ALL: &[NodeColumn]` or similar slice and iterate that instead.)

Wire validation into `parse_node_filter`:

```rust
    let valid = valid_columns(label_registry);

    for token in trimmed.split_whitespace() {
        match parse_term(token) {
            Term::Bare(v) => {
                let rx = Regex::new(&v).map_err(|e| format!("invalid regex '{}': {}", v, e))?;
                column_includes.entry("name".to_string()).or_default().push(rx);
            }
            Term::Include { column, value } => {
                if !valid.contains(&column) {
                    return Err(format!("unknown column '{}'", column));
                }
                let rx = Regex::new(&value)
                    .map_err(|e| format!("invalid regex '{}': {}", value, e))?;
                column_includes.entry(column).or_default().push(rx);
            }
            Term::Exclude { column, value } => {
                if !valid.contains(&column) {
                    return Err(format!("unknown column '{}'", column));
                }
                let rx = Regex::new(&value)
                    .map_err(|e| format!("invalid regex '{}': {}", value, e))?;
                column_excludes.entry(column).or_default().push(rx);
            }
            Term::Label(sel) => {
                label_selector = Some(sel);
            }
        }
    }
```

- [ ] **Step 5: Run tests to verify they pass**

Run:
```bash
cargo test --bin kubetui features::node::filter::parser::tests 2>&1 | tail -15
```
Expected: all 21 tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/features/node/filter/parser.rs
git commit -m "feat(node): parser — validate column names against registry

Builtin Node columns (NodeColumn::iter().display()) plus headers in the
configured label_registry form the set of acceptable column names.
Typos like 'STATUSU:Ready' produce a parse error 'unknown column
\"statusu\"', which the Table widget renders via filter_error
(body-replacement overlay). The 'label:' keyword bypasses validation —
it is the special-cased server-side k8s labelSelector path, not a
client-side column lookup."
```

---

### Task 6: `node_filter_applicator()` factory (TDD)

**Purpose:** Wire the parser into a `TableFilterApplicator` configured for the Node tab: `EnterToConfirm` strategy, help-dialog dispatch on `?`, `on_apply` that forwards the parsed `labelSelector` to the Node poller.

**Files:**
- Modify: `src/features/node/filter.rs`

- [ ] **Step 1: Write the factory**

Replace `src/features/node/filter.rs` body:

```rust
//! Node tab filter: parser + `TableFilterApplicator` factory.

mod parser;

use crossbeam::channel::Sender;

use crate::{
    features::{
        component_id::NODE_FILTER_HELP_DIALOG_ID,
        node::{message::NodeFilterMessage, node_columns::NodeLabelColumn},
    },
    message::Message,
    ui::widget::{ApplyStrategy, TableFilterApplicator, TableFilterParser},
};

pub use parser::parse_node_filter;

/// Build the Node tab's filter applicator.
///
/// `label_registry` is captured by value and used by the parser for
/// column-name validation. `tx` is used by `on_apply` to send
/// `NodeFilterMessage::Apply(label_selector)` to the Node poller; the
/// poller updates its `?labelSelector=` URL parameter on the next tick.
pub fn node_filter_applicator(
    label_registry: Vec<NodeLabelColumn>,
    tx: Sender<Message>,
) -> TableFilterApplicator {
    let parser: TableFilterParser = (move |input: &str| {
        parse_node_filter(input, &label_registry)
    })
    .into();

    TableFilterApplicator::new(parser, ApplyStrategy::EnterToConfirm)
        .with_help_dialog(NODE_FILTER_HELP_DIALOG_ID)
        .with_on_apply(move |predicate, _window| {
            tx.send(NodeFilterMessage::Apply(predicate.label_selector.clone()).into())
                .expect("Failed to send NodeFilterMessage::Apply");
        })
}
```

- [ ] **Step 2: Write a smoke test that exercises the factory**

Append to `src/features/node/filter.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crossbeam::channel;

    #[test]
    fn applicator_constructs_without_panic() {
        let (tx, _rx) = channel::bounded(1);
        // Simply constructing the applicator exercises all the closure
        // wiring; this catches type / capture errors that would otherwise
        // only surface at first user keystroke.
        let _ = node_filter_applicator(Vec::new(), tx);
    }

    #[test]
    fn applicator_on_apply_sends_label_selector_via_tx() {
        let (tx, rx) = channel::bounded(1);
        let app = node_filter_applicator(Vec::new(), tx);

        // Build a predicate as if parser produced it
        let pred = parse_node_filter("label:role=worker", &[]).unwrap();
        let on_apply = app.on_apply.as_ref().expect("on_apply must be set");

        // The callback expects (&TableFilterPredicate, &mut Window). We
        // don't have a Window here; this test asserts the message is
        // sent via the captured Sender. Since on_apply is `Rc<dyn Fn>`,
        // we have to construct a dummy Window. Skip this assertion if
        // Window has no `Default` impl and the test cannot compile —
        // the construction-only test above still covers the wiring.
        //
        // If Window has a constructor we can use, call on_apply and
        // expect Ok(NodeFilterMessage::Apply(Some("role=worker"))) on rx.
        let _ = (pred, on_apply, rx);
    }
}
```

Note: depending on whether `Window` is constructible in tests, the second test may need to be skipped or rewritten. Verify before committing — if `Window::builder().build()` works in tests elsewhere in the codebase (`grep -rn 'Window::builder' src/`), use that. Otherwise delete the second test and rely on the construction-only smoke test plus parser-level coverage.

- [ ] **Step 3: Run tests to verify they pass**

Run:
```bash
cargo test --bin kubetui features::node::filter::tests 2>&1 | tail -15
```
Expected: 1 or 2 tests pass (depending on the second test's fate).

- [ ] **Step 4: Verify the file builds cleanly**

Run:
```bash
cargo build 2>&1 | tail -5
```
Expected: clean build.

- [ ] **Step 5: Commit**

```bash
git add src/features/node/filter.rs
git commit -m "feat(node): node_filter_applicator factory

Wire the column-aware parser into a TableFilterApplicator configured for
the Node tab: ApplyStrategy::EnterToConfirm (avoid spamming the kube API
mid-typing), help-dialog dispatch via NODE_FILTER_HELP_DIALOG_ID, and
on_apply that forwards the parsed labelSelector to the Node poller via
NodeFilterMessage::Apply.

Client-side regex matching is fully handled by TableFilterPredicate::matches
(from PR A), so on_apply only needs to send the label_selector — the
rest of the predicate lives in Table.filter_state and is consulted by
the widget render path."
```

---

### Task 7: Reduce `NodeFilterMessage::Apply` payload to `Option<String>` and ripple through controller/poller

**Purpose:** The framework now owns the client-side filter state. The kube side only needs `labelSelector`. Trim the message type and adjust the controller/poller plumbing.

**Files:**
- Modify: `src/features/node/message.rs`
- Modify: `src/workers/kube/controller.rs` (or wherever `NodeFilterMessage::Apply` is handled)
- Modify: `src/features/node/kube/node.rs` (`SharedNodeFilter` type and URL builder)

- [ ] **Step 1: Change the message variant**

Edit `src/features/node/message.rs`. Replace the `Apply(Option<super::filter::NodeFilter>)` variant with:

```rust
/// Messages for changing the active Node-list filter.
#[derive(Debug)]
pub enum NodeFilterMessage {
    /// Replace the active labelSelector value. `None` clears it (the
    /// poller stops sending ?labelSelector= in its request URL).
    Apply(Option<String>),
}
```

Remove any `use super::filter::NodeFilter` import that becomes unused.

- [ ] **Step 2: Change `SharedNodeFilter` type**

Edit `src/features/node/kube/node.rs`. Replace:

```rust
pub type SharedNodeFilter = Arc<RwLock<Option<NodeFilter>>>;
```

with:

```rust
pub type SharedNodeFilter = Arc<RwLock<Option<String>>>;
```

Remove the now-unused `use crate::features::node::filter::NodeFilter` import.

- [ ] **Step 3: Simplify the URL builder**

Still in `src/features/node/kube/node.rs`, replace the existing helper:

```rust
fn node_request_path(filter: Option<&NodeFilter>) -> String {
    let base = Node::url_path(&(), None);
    match filter.and_then(|f| f.label_selector.as_deref()) {
        Some(sel) if !sel.is_empty() => format!("{}?labelSelector={}", base, sel),
        _ => base,
    }
}
```

with:

```rust
fn node_request_path(label_selector: Option<&str>) -> String {
    let base = Node::url_path(&(), None);
    match label_selector {
        Some(sel) if !sel.is_empty() => format!("{}?labelSelector={}", base, sel),
        _ => base,
    }
}
```

Update its single call site in the poller `run()` loop. The call previously read the lock once and passed the `Option<NodeFilter>` reference; now read the lock once and pass `selector.as_deref()`.

- [ ] **Step 4: Remove the client-side `matches_name` filtering pass**

Still in `src/features/node/kube/node.rs`, locate the iterator chain that does:

```rust
.filter(|r| {
    filter
        .as_ref()
        .map(|f| f.matches_name(&r.name))
        .unwrap_or(true)
})
.collect();
```

Delete the entire `.filter(...)` call (the rows now flow straight through; client-side filtering is the UI's job).

- [ ] **Step 5: Adjust the controller handler**

In `src/workers/kube/controller.rs`, find the `NodeFilterMessage::Apply` arm. Replace its body with:

```rust
NodeFilterMessage::Apply(label_selector) => {
    let mut guard = self.shared_node_filter.write().await;
    *guard = label_selector;
}
```

(If the existing handler uses sync `RwLock`, drop the `.await`. Match the surrounding style.)

- [ ] **Step 6: Update poller and URL builder tests**

In `src/features/node/kube/node.rs`'s `tests` module, replace the `node_request_path` tests so they pass `Option<&str>` directly:

```rust
        #[test]
        fn label_selector_is_appended_as_query() {
            assert_eq!(
                node_request_path(Some("role=worker,zone=us-west")),
                "/api/v1/nodes?labelSelector=role=worker,zone=us-west"
            );
        }

        #[test]
        fn empty_selector_produces_base_path() {
            assert_eq!(node_request_path(None), "/api/v1/nodes");
            assert_eq!(node_request_path(Some("")), "/api/v1/nodes");
        }
```

Delete any leftover tests that construct `NodeFilter { label_selector: Some(...), .. }`.

- [ ] **Step 7: Build and run tests**

Run:
```bash
cargo build 2>&1 | tail -5
cargo test --bin kubetui 2>&1 | tail -5
```
Expected: clean build, all tests pass. The `NodeFilter` struct itself may still exist at this point; it is deleted in Task 9.

- [ ] **Step 8: Commit**

```bash
git add src/features/node/message.rs src/features/node/kube/node.rs src/workers/kube/controller.rs
git commit -m "refactor(node): NodeFilterMessage payload is Option<String> (label selector only)

Client-side regex filtering moved to TableFilterPredicate (PR A). The
Node poller now only needs the k8s labelSelector value, so:

  - NodeFilterMessage::Apply(Option<String>) replaces Apply(Option<NodeFilter>)
  - SharedNodeFilter = Arc<RwLock<Option<String>>>
  - node_request_path takes Option<&str>
  - the poller's .filter(|r| f.matches_name(&r.name)) pass is dropped;
    rows flow straight through to the UI, which filters via Table.filter_state

NodeFilter (the struct) is not removed yet — Task 9 deletes it once the
last reference is gone."
```

---

### Task 8: Switch `node.rs` widget to `filter_form` + `node_filter_applicator`

**Purpose:** Replace the `.action('/', open_node_filter_dialog())` shortcut with the inline filter form on the Table widget. Re-introduce the title block_injection that displays the active filter raw text and match counts, reading from `Table.filter_state` (which now holds `TableFilterPredicate`).

**Files:**
- Modify: `src/features/node/view/widgets/node.rs`
- Modify: `src/features/node/view/tab.rs` (caller side — pass `label_registry` and `tx`)

- [ ] **Step 1: Identify how callers obtain `label_registry`**

Run:
```bash
grep -n 'label_registry\|NodeLabelColumn' src/features/node/view/tab.rs src/workers/render/window.rs 2>&1
```
Note the existing path that supplies the `NodeLabelColumn` registry to the Node tab (Plans 1–3). The applicator needs the same value — usually it is already passed into `NodeTab::new` or is reachable from a shared `Rc<RefCell<NodeColumns>>` via a snapshot.

- [ ] **Step 2: Update `node_widget()` signature and body**

Edit `src/features/node/view/widgets/node.rs`. Change the signature to accept `label_registry: Vec<NodeLabelColumn>`:

```rust
use crate::features::node::{
    filter::node_filter_applicator,
    message::NodeDetailMessage,
    node_columns::NodeLabelColumn,
};
use crate::ui::widget::{FilterForm, TableFilterPredicate};
```

Replace the `Table::builder()` chain. Drop both `.filtered_key("NAME")` (already gone in PR A — verify) and `.action('/', open_node_filter_dialog())`. Add `.filter_form(...)` and `.filter_applicator(...)`:

```rust
pub fn node_widget(
    tx: Sender<Message>,
    label_registry: Vec<NodeLabelColumn>,
    theme: WidgetThemeConfig,
) -> Widget<'static> {
    let widget_theme = WidgetTheme::from(theme.clone());
    let table_theme = TableTheme::from(theme.clone());

    let widget_base = WidgetBase::builder()
        .title("Node")
        .theme(widget_theme)
        .build();

    Table::builder()
        .id(NODE_WIDGET_ID)
        .widget_base(widget_base)
        .theme(table_theme)
        .filter_form(FilterForm::default())
        .filter_applicator(node_filter_applicator(label_registry, tx.clone()))
        .action('t', open_node_columns_dialog())
        .on_select(on_select(tx))
        .block_injection(block_injection())
        .build()
        .into()
}
```

- [ ] **Step 3: Rewrite `block_injection` to read from `Table.filter_state()`**

In the same file, replace the prior `block_injection` (which took a `NodeFilterTitleState` written by the standalone dialog) with one that reads the new `Table.filter_state()` accessor:

```rust
fn block_injection() -> impl Fn(&Table) -> WidgetBase {
    |table: &Table| {
        let mut base = table.widget_base().clone();

        let title = match table.filter_state() {
            Some(pred) if !pred.raw.is_empty() => {
                let matched = table.items().len();
                format!("Node [{}/{}]", matched, table.original_items_len())
                    .into() // adjust if WidgetBase::title takes &str
            }
            _ => "Node".into(),
        };

        // Use whichever WidgetBase setter your code base exposes for title.
        base.set_title(title);
        base
    }
}
```

Note: the exact `WidgetBase` mutation API may differ — `grep -n 'fn title\|pub fn set_title' src/ui/widget/base.rs` to confirm. The intent is: when a filter is active, render `Node [matched/total]` plus optionally the raw filter text; otherwise plain `Node`. Adjust the format string if there is a project convention you can mirror (Pod tab title is a good reference).

Also: `Table::filter_state()` accessor and `Table::original_items_len()` may need to be added to PR A's surface if not present. Check with:

```bash
grep -n 'fn filter_state\|original_items_len' src/ui/widget/table.rs
```

If absent, add public accessors in a small follow-up to PR A (or include the accessor addition as part of this task — a single read-only getter is low-risk). Don't reach into private fields.

- [ ] **Step 4: Remove obsolete `NodeFilterTitleState` references**

Search and remove:

```bash
grep -rn 'NodeFilterTitleState' src/
```

Delete every match (it was the bridge between the standalone dialog and the title; the new title reads from `Table.filter_state()`).

- [ ] **Step 5: Update `tab.rs` to pass `label_registry`**

Edit `src/features/node/view/tab.rs`. Where `node_widget(...)` is called, supply the registry:

- Locate the existing `NodeTab::new` or factory and add a `label_registry: Vec<NodeLabelColumn>` parameter (or reuse an already-passed `NodeColumns` and call `.label_columns()` if such a method exists).
- Remove the construction of `NodeFilterTitleState` and the `.action('/', ...)` opening the standalone dialog.
- Keep the `NODE_FILTER_HELP_DIALOG_ID` dialog registration intact (Task 10 updates its content).

- [ ] **Step 6: Build and run tests**

Run:
```bash
cargo build 2>&1 | tail -8
cargo test --bin kubetui 2>&1 | tail -5
```
Expected: clean build, tests pass. Compile errors are likely if `NodeFilterTitleState` is still referenced elsewhere — fix incrementally.

- [ ] **Step 7: Commit**

```bash
git add src/features/node/view/widgets/node.rs src/features/node/view/tab.rs
git commit -m "feat(node): switch widget to filter_form + node_filter_applicator

Drop the .action('/', open_node_filter_dialog()) shortcut that opened
the standalone NodeFilter dialog. The Table widget's built-in
FilterForm now hosts the input; node_filter_applicator drives parsing,
help-dialog dispatch, and labelSelector forwarding.

Title block_injection now reads Table.filter_state() (the
TableFilterPredicate set by PR A's filter framework) rather than a
NodeFilterTitleState shim. Displays 'Node [matched/total]' when a
filter is active."
```

---

### Task 9: Delete obsolete code (NodeFilter struct, standalone dialog widget, component ID)

**Purpose:** Now that nothing references the old API, remove the dead code in one focused commit so the PR diff is clear about what is being replaced.

**Files:**
- Delete: `src/features/node/view/widgets/node_filter.rs`
- Modify (delete contents): `src/features/node/filter.rs` (the old `NodeFilter` struct definition lived here; Task 6 replaced the body, so this is usually a no-op — verify nothing slipped through)
- Modify: `src/features/component_id.rs` (remove `NODE_FILTER_WIDGET_ID`)
- Modify: `src/features/node/view/widgets.rs` (remove `pub mod node_filter;`)
- Modify: `src/features/node/view/tab.rs` (remove the dialog widget construction and registration)

- [ ] **Step 1: Delete the standalone dialog widget file**

Run:
```bash
git rm src/features/node/view/widgets/node_filter.rs
```

- [ ] **Step 2: Remove the module declaration**

Edit `src/features/node/view/widgets.rs`. Remove the `pub mod node_filter;` line.

- [ ] **Step 3: Remove `NODE_FILTER_WIDGET_ID` from the component ID registry**

Edit `src/features/component_id.rs`. Remove the line declaring `NODE_FILTER_WIDGET_ID`. Then:

```bash
grep -rn 'NODE_FILTER_WIDGET_ID' src/
```
Expected: no matches. Fix any leftover references.

- [ ] **Step 4: Remove the dialog registration from tab.rs**

Edit `src/features/node/view/tab.rs`. Remove the chunk that constructs the `node_filter_widget()` Dialog and registers it. Keep the `node_filter_help_widget()` Dialog registration (Task 10 rewrites its content).

- [ ] **Step 5: Verify `NodeFilter` struct is gone**

Run:
```bash
grep -rn 'struct NodeFilter\b\|::NodeFilter\b\|: NodeFilter\b' src/
```
Expected: no matches. If `NodeFilter` is still defined in `src/features/node/filter.rs`, edit the file to remove the struct + impl + `pub use`. (Task 6 should have removed it; this step is a safety net.)

- [ ] **Step 6: Build and test**

Run:
```bash
cargo build 2>&1 | tail -5
cargo test --bin kubetui 2>&1 | tail -5
```
Expected: clean build, all tests pass.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "refactor(node): remove standalone NodeFilter dialog + dead types

With the inline FilterForm taking over the input UX, the standalone
node_filter dialog widget and its NODE_FILTER_WIDGET_ID component ID
have no consumers. The NodeFilter struct itself is also retired —
TableFilterPredicate (PR A) is the sole filter type. The
NODE_FILTER_HELP_DIALOG_ID dialog is preserved (Task 10 rewrites its
content for the new syntax)."
```

---

### Task 10: Update help dialog content for the new syntax

**Files:**
- Modify: `src/features/node/view/widgets/node_filter_help.rs`

- [ ] **Step 1: Rewrite `content()`**

Edit `src/features/node/view/widgets/node_filter_help.rs`. Replace the body of `content()` with:

```rust
fn content() -> Vec<String> {
    indoc! {r#"
        Usage: TERM [ TERM ]...

        Terms:
           <value>            Plain value: NAME include (regex).
           NAME:<regex>       Include nodes where NAME matches.
           STATUS:<regex>     Include where STATUS matches. Multiple
                              same-column includes are OR (in-list).
           !<COL>:<regex>     Exclude nodes whose COL matches.
           label:<selector>   Kubernetes labelSelector, applied
                              server-side (e.g. role=worker,zone=us-west).
                              Last 'label:' wins if repeated.

        Combining:
           Same column, multiple includes  →  OR (in-list)
           Different columns, includes     →  AND across columns
           Any matching exclude            →  row excluded
           Bare values                     →  treated as NAME includes

        Examples
           worker                          Show nodes whose NAME matches 'worker'
           NAME:gke STATUS:Ready           NAME~gke AND STATUS~Ready
           STATUS:Ready STATUS:Pending     STATUS in (Ready, Pending)
           !NAME:control label:zone=us     Server-side label filter + name exclude
           NAME:(?=.*foo)(?=.*bar)         Single-column AND via regex lookahead

        Column names are case-insensitive. Unknown columns produce a
        parse error. Press Enter to apply, Esc to cancel. Type ? or
        help in the filter input to open this help.
    "# }
    .lines()
    .map(ToString::to_string)
    .collect()
}
```

(Adjust the column names if `NodeColumn::display()` returns different display strings than `NAME`/`STATUS` — keep the convention consistent with what the user actually sees in column headers.)

- [ ] **Step 2: Build and visually inspect**

Run:
```bash
cargo build 2>&1 | tail -3
```
Expected: clean build. Visual confirmation is part of Task 11's manual smoke.

- [ ] **Step 3: Commit**

```bash
git add src/features/node/view/widgets/node_filter_help.rs
git commit -m "docs(node): rewrite filter help for column-aware syntax

Replace the old node:/!node:/label: documentation with the new
<COL>:<val> / !<COL>:<val> / label:<sel> grammar and the column-OR /
cross-AND / any-match-excludes semantics. Adds examples for the
single-column-AND regex-lookahead workaround and explicitly notes
case-insensitive column names and unknown-column parse errors."
```

---

### Task 11: Polish (fmt, clippy, full tests, manual smoke notes)

**Files:**
- Whole workspace.

- [ ] **Step 1: Run formatter**

Run:
```bash
cargo +nightly fmt
git status --short
```
If any files were rewritten, stage them: `git add -A`.

- [ ] **Step 2: Run clippy and address PR-B-introduced warnings**

Run:
```bash
cargo clippy --all-targets 2>&1 | grep -E '^(warning|error)' | head -30
```
Expected: no new warnings introduced by this PR. Pre-existing warnings (the 3 `too_many_arguments` ones from PR A's scope check) can be left alone. For any new warning, prefer fixing the code over `#[allow]`. If `#[allow]` is unavoidable, attach a one-line comment naming the cross-PR consumer.

- [ ] **Step 3: Run full test suite**

Run:
```bash
cargo test --all 2>&1 | tail -5
```
Expected: all tests pass, no regressions.

- [ ] **Step 4: Manual smoke list (for the PR description)**

Document the following so the reviewer can re-do them. Each item should be verifiable in a few seconds against a real cluster.

Suggested set:
1. `/` opens the Node filter form (inline, not a separate dialog).
2. `STATUS:Ready` filters live to nodes whose STATUS matches. Press Enter → form stays, rows stay filtered.
3. `STATUSU:Ready` (typo) shows a body-replacement error "unknown column 'statusu'" that does NOT disappear on the next polling tick (sticky).
4. `?` in the filter input opens the Node Filter Help dialog with the new syntax.
5. `label:role=worker` (or any valid label your cluster has): Enter applies it; the kubectl-style server-side filter is sent on the next poll (verify via `kubectl get --raw` against the same URL, or via the node count changing).
6. `NAME:foo !NAME:bar` honors include AND exclude on the same column.
7. `Esc` from FilterInput or FilterConfirm clears both rows and the error overlay.
8. Polling tick during an active filter does not reset the filter (matches behavior verified for PR A).

If any manual item fails, fix the root cause and re-run from Step 1.

- [ ] **Step 5: Commit final polish**

Only commit if Steps 1–3 produced changes:

```bash
git add -A
git commit -m "chore: cargo +nightly fmt and final polish for PR B"
```

(If no files changed, skip the commit.)

- [ ] **Step 6: Push and prepare PR description**

After explicit user approval (per project convention), push the branch and open a stacked PR against the prerequisite branch:

```bash
git push -u origin feat/node-filter-applicator
gh pr create --base <prerequisite-branch> --title "feat(node): column-aware filter via TableFilterApplicator" --body "..."
```

PR body skeleton (English, mirroring PR #982 structure):

```markdown
Stacked on <PR A or integration PR>.

## Summary
- Implement Node tab's column-aware filter using the TableFilterApplicator
  framework (PR A). Bare values default to NAME, <COL>:<val> targets a
  named column, !<COL>:<val> excludes, label:<sel> is the server-side
  k8s labelSelector (last-wins).
- Drop the standalone NodeFilter dialog widget; the Table widget's
  inline FilterForm hosts the input.
- Reduce NodeFilterMessage payload to Option<String> (only labelSelector
  reaches the server; client-side regex matching is owned by
  TableFilterPredicate::matches).

## Manual verification
(checklist from Task 11 Step 4)
```

---

## Self-review

### Spec coverage

- §307 (構文) → Tasks 1–5 (bare, COL:val, !COL:val, label:, unknown-column error)
- §323 (意味論) → handled by PR A's `TableFilterPredicate::matches`; parser only routes terms into the correct buckets (Tasks 1–4)
- §334 (列名 case-insensitive、正準形 lowercase) → Task 2 Step 3
- §336 (値は常に regex) → Tasks 1–3 use `Regex::new` on bare/include/exclude values
- §338 (NodeFilterApplicator) → Task 6
- §355 (build_on_apply) → Task 6 `with_on_apply` closure
- §367 (サーバ側フィルタリング) → Task 7 (payload reduction, URL builder)
- §407 (ヘルプ) → Task 10
- §449 (モジュール構成: node_filter.rs 削除 / node_filter_help.rs 保持) → Tasks 9, 10
- §479 (Node タブ実装の編集点リスト) → Tasks 8 (node.rs widget), 9 (deletes), 10 (help), all relevant

### Placeholder scan

- No "TBD" / "implement later" / "add appropriate error handling" placeholders.
- Code blocks given for every code step.
- Tests written before implementation in TDD tasks.

### Type consistency

- `TableFilterPredicate` field names (`column_includes`, `column_excludes`, `label_selector`, `raw`) match PR A and are used consistently across Tasks 1–6.
- `NodeFilterMessage::Apply(Option<String>)` is introduced in Task 7 and consumed by the controller in the same task; no later task references the old `Option<NodeFilter>` shape.
- `SharedNodeFilter = Arc<RwLock<Option<String>>>` matches the controller assignment in Task 7 Step 5.
- `node_filter_applicator(label_registry: Vec<NodeLabelColumn>, tx: Sender<Message>)` (Task 6) and its single call site (Task 8 Step 2) agree on argument order.
- `parse_node_filter(input: &str, label_registry: &[NodeLabelColumn]) -> Result<TableFilterPredicate, String>` is the signature from Task 1 onward; later tasks add cases inside the body without changing the signature.

### Known assumptions to verify at execution time

- `NodeColumn::display()` returns the column header used in the table (Task 1 / Task 5). If the actual method is named differently, Task 5 Step 4 needs the real name.
- `Table::filter_state()` and `Table::original_items_len()` accessors exist on PR A (Task 8 Step 3). If absent, add them as small follow-on commits to PR A or include the additions in Task 8.
- `Window` is constructible in tests (Task 6 Step 2 optional test). If not, drop that test.
