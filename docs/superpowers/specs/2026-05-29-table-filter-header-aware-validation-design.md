# Table filter: header-aware column validation & name normalization

- Date: 2026-05-29
- Status: Proposed
- Scope: shared Table filter framework (`src/ui/widget/table/`) + Node tab filter (`src/features/node/filter/`) as the proving ground
- Out of scope: Pod tab migration to column-aware filtering (tracked as a separate Phase B spec)

## Background

The shared Table filter framework (`TableFilterApplicator`, introduced in PRs #980/#982,
design `docs/superpowers/specs/2026-05-27-table-filter-redesign.md`) lets a tab parse a
raw filter string into a `TableFilterPredicate` and match it against table rows. The Node
tab uses a column-aware parser (`COL:val` / `!COL:val` / `label:sel` / bare→NAME); Pod /
Config / Network still use the simpler `substring_applicator`.

While planning the Pod migration we found two pre-existing defects in the shared framework.
They are already reachable from the Node tab today; they are not Pod-specific.

### 課題 I — column names containing spaces are unfilterable

- The parser splits the input on whitespace (`separated_list0(multispace1, parse_token)`,
  `src/features/node/filter/parser.rs:264-269`), so a column token can never contain a space.
- Matching looks up the column by **plain** lowercase string equality:
  `cell_of` compares `h.to_lowercase() == col_name_lower`
  (`src/ui/widget/table/filter_applicator.rs:80-89`).
- Node builtin columns are hyphenated (`INTERNAL-IP`, `OS-IMAGE`, …) so they tokenize fine,
  but a Node **label** column header is `def.name.to_uppercase()` with no space stripping
  (`src/app.rs:272`). A label column whose name contains a space (e.g. `"my label"` →
  header `"MY LABEL"`) is registered as a valid column (`valid_columns` inserts
  `lc.header.to_lowercase()`, `parser.rs:217-225`) yet can never be matched, because the
  user cannot type the space and `cell_of` does exact-with-space comparison.

In short: there is no existing solution to copy. The Pod migration *needs* one because Pod's
default builtin columns include `NOMINATED NODE` and `READINESS GATES` (spaces by default).

### 課題 II — filtering a valid-but-hidden column silently hides every row

- `valid_columns` is built from the full `NodeColumn::iter()` enum plus the label registry
  (`parser.rs:217-225`), independent of which columns are currently displayed.
- Node columns are runtime-configurable via the column dialog
  (`NodeMessage::Request` → `shared_node_columns`, `src/workers/kube/controller.rs:616-621`);
  the default display is 5 of 10 builtin columns (`DEFAULT_NODE_COLUMNS`).
- Matching uses `cell_of(...).unwrap_or_default()`: a column not present in the live header
  yields `""`, so an `include` pattern fails for every row
  (`filter_applicator.rs:53-71`). Result: filtering on a real-but-hidden column (e.g.
  `internalip:10.` with the default Node columns) parses OK and then makes the whole table
  appear empty — looks like a bug, with no feedback.

## Goals

1. Make column-name matching tolerant of spaces / `-` / `_` so multi-word columns are filterable.
2. Replace the "all rows vanish" behavior for non-displayed columns with an explicit, helpful
   parse error.
3. Unify both fixes at the framework level so every tab (Node now; Pod / Config / Network
   later) benefits, with no per-tab workaround.
4. Keep the change behaviorally safe for the existing `substring_applicator` tabs.

## Non-goals

- Migrating Pod (or Config / Network) to the column-aware parser — Phase B.
- Server-side `labelSelector` plumbing for Pod — Phase B.
- Auto-adding a hidden column to the view when it is referenced in a filter.

## Mental model

The rule we are encoding: **you can filter exactly the columns you can see.** This matches how
TUI users (k9s, fzf, less) expect filtering to work — it narrows the visible representation.
A reference to a column that is not in the current view is reported as an error that tells the
user the column is not shown (so they can add it via the column dialog), rather than silently
returning nothing or silently ignoring the term.

## Design

### 1. The parser receives the live header

Change the `TableFilterParser` callback signature from

```
Fn(&str) -> Result<TableFilterPredicate, String>
```

to

```
Fn(&str, &[String]) -> Result<TableFilterPredicate, String>
```

where the second argument is the table's current display header
(`self.items.header().original()`). The Table widget already holds this and uses it for
matching; `run_parser_and_update_state` (`src/ui/widget/table.rs:831-839`) is a `&mut self`
method with access to it, so it passes `&header` into the parser closure.

The header is the single source of truth for "what is displayed," is already what matching
compares against, and is available synchronously on the render thread — which sidesteps the
async `RwLock` around the per-tab column config.

### 2. Shared column-name normalization

Introduce one normalization function in the filter framework:

```
normalize_column_name(s) = s.to_lowercase() with all ' ', '-', '_' removed
```

(`PodColumn::normalize_column` already implements exactly this and can be the reference; the
shared helper lives in the filter module so all tabs use the same rule.)

Apply it on **both** sides of every column-name comparison:

- In `cell_of`: normalize each header entry and the looked-up key before comparing, instead of
  plain `to_lowercase()`. Hyphenated names keep working (`internal-ip` → `internalip`);
  hyphens/underscores/spaces become insignificant.
- In the parser: normalize the user's column token and each header entry to validate, and store
  the **normalized** column name as the predicate key. Because `cell_of` also normalizes, the
  stored key resolves back to the right header column.

This makes `nominatednode`, `nominated-node`, `Nominated_Node` all match the `NOMINATED NODE`
header, and makes spaced Node label columns filterable.

### 3. Validate column references against the live header (課題 II)

In the parser, the valid column set is the set of **normalized header entries** (not the
builtin enum, not the label-registry snapshot). For each `COL:val` / `!COL:val` term:

- If `normalize_column_name(COL)` is not among the normalized header entries → return a parse
  error: `column '<COL>' is not in the current view` (the existing `filter_error` channel
  renders it in place of the table body — sticky until corrected).
- `label:sel` is always accepted (it is a server-side selector, not a display column).
- A bare value maps to the NAME column include; NAME is guaranteed present
  (`ensure_name_column`), so this never errors in practice.

Consequence: the Node parser no longer needs `NodeColumn::iter()` or the `label_registry`
argument for validation. `node_filter_applicator`'s `label_registry` parameter becomes unused
for validation and is removed; the parser is simplified to validate purely against the header.

Edge case — empty header (before the first poll populates the table): if the header is empty,
skip column validation (accept the term) so the user does not get spurious "not in the current
view" errors before any data has arrived. (In Node the header is built from the configured
column specs and is populated as soon as the table is built once, so this only affects the
brief pre-first-render window.)

### 4. Error-message wording

Use "not in the current view" rather than "unknown column", because from the filter's
perspective the only knowable columns are the displayed ones; a real-but-hidden k8s column and
a genuine typo are indistinguishable and both correctly resolve to "that column is not shown".

### 5. Help dialog

Update the Node filter help (`src/features/node/view/widgets/node_filter_help.rs`) so the
"valid columns" guidance reads as "the columns currently shown in the table" instead of a fixed
enumeration. Dynamically listing the current header in the dialog is optional polish and may be
deferred.

### 6. `substring_applicator` and other tabs

`substring_applicator(column)` (`src/ui/widget/table/filter_applicator.rs`) produces a
`TableFilterParser`; its closure signature is updated to accept and ignore the header argument
(it filters a single fixed column — NAME — which is always present, so its behavior is
unchanged). The shared `cell_of` normalization is a no-op for single-word `NAME`, so Pod /
Config / Network matching is unaffected until their own Phase B migration.

## Affected files

- `src/ui/widget/table/filter_applicator.rs` — `TableFilterParser` signature; `cell_of`
  normalization; new `normalize_column_name` helper; `substring_applicator` closure signature.
- `src/ui/widget/table.rs` — pass `&header` into the parser in `run_parser_and_update_state`;
  update the in-module test parser (`table.rs:~1120`).
- `src/features/node/filter.rs` — drop `label_registry` from `node_filter_applicator`; update
  the parser wiring; update callers that pass the registry.
- `src/features/node/filter/parser.rs` — `parse_node_filter` takes the header instead of the
  label registry; validate against normalized header; store normalized keys; update tests.
- `src/features/node/view/widgets/node_filter_help.rs` — help wording.
- Callers of `node_filter_applicator` (e.g. `src/features/node/view/widgets/node.rs`,
  `tab.rs`, render wiring) — drop the now-unused registry argument.

## Testing

- Unit tests for `normalize_column_name` (spaces / `-` / `_` / case).
- `cell_of` tests: multi-word header matches normalized key; hyphenated header still matches.
- `parse_node_filter` tests (header-driven): valid displayed column; multi-word column via
  normalized token; `!COL` exclude; `label:`; bare→NAME; **hidden column → parse error**;
  empty-header → no validation error.
- Confirm `substring_applicator` behavior is unchanged (existing Pod/Config/Network filter
  tests still pass).
- `cargo test --all`, `cargo clippy`, `cargo +nightly fmt --check`.

## Risks / backward incompatibility

- Node matching semantics change slightly: `-` / `_` / spaces in column names become
  insignificant (e.g. `internalip` now matches `INTERNAL-IP`). This is a strict superset of
  current matches; existing `internal-ip` queries keep working. Node parser tests must be
  updated to reflect normalized comparison.
- Removing `label_registry` from `node_filter_applicator` touches its call sites; mechanical.

## Follow-up (Phase B — separate spec)

Migrate the Pod tab from `substring_applicator("NAME")` to a column-aware
`pod_filter_applicator`, riding on this fixed framework:
- `parse_pod_filter` validates against the live header (incl. `NAMESPACE` when shown), so Pod's
  multi-word builtin columns work with no Pod-local normalization hack.
- `label:` server-side selector: add `PodMessage::Filter(Option<String>)` + `SharedPodFilter`,
  wire `?labelSelector=` into the per-namespace pod fetch (`get_pods_per_namespace`), handle the
  message in the controller; `EnterToConfirm` strategy + help dialog, mirroring Node.
- The Pod log-query parser (`src/features/pod/kube/filter.rs`) is unrelated (filters log lines,
  different pane) and is left untouched.
