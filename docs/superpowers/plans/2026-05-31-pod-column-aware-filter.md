# Pod column-aware フィルタ移行 実装計画 (PR B)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Pod テーブルのフィルタを `substring_applicator("NAME")` から column-aware な `pod_filter_applicator` に置き換え、`label:` サーバーサイド selector も含めて Node と同等にする（namespace は除く・後述）。

**Architecture:** PR #991 で抽出した共有 `parse_table_filter(input, validate_column)` の上に Pod 用の薄い `parse_pod_filter` を載せる。既知列は `PodColumn::iter()`（9種）のみで label 列は無い。`namespace:` は「選択軸であってフィルタ列ではない」(Z) として親切メッセージを返し、その他の未定義列は `unknown column` エラー。NAMESPACE 列は複数 ns 表示時の文脈ラベルのまま不変。`label:` は Node の `SharedNodeFilter` 配線をミラーした `SharedPodFilter` で per-namespace の URL に `?labelSelector=` を付与。`EnterToConfirm`＋help dialog。inactive/バッジ/正規化は framework から自動。

**Tech Stack:** Rust 2021、nom（共有パーサ）、regex、tokio（async RwLock）、strum、`#[cfg(test)]` インラインテスト、pretty_assertions。

参照設計:
- 共有 framework: [[table-filter-framework memory]] と #988/#989/#991 の spec/plan。
- PR B 設計は brainstorming 済み（pod-filter-migration memory に集約）。Node の `src/features/node/filter/`・`src/features/node/kube/node.rs`・`src/features/node/view/widgets/node_filter_help.rs` が雛形。

注: kubetui は binary crate。テストは `cargo test <path>` / `cargo test --all`（`--lib` 不可）。

---

## ファイル構成

- Modify: `src/features/pod/message.rs` — `PodMessage::Filter(Option<String>)` 追加。
- Modify: `src/features/pod/kube/pod.rs` — `SharedPodFilter` 型・`PodPoller` フィールド・`get_pods_per_namespace` の per-ns URL に `?labelSelector=` 付与。
- Modify: `src/workers/kube/controller.rs` — `SharedPodFilter` 構築・`PodPoller` へ受け渡し・`Kube::Pod(PodMessage::Filter(sel))` ハンドラ追加（Node の SharedNodeFilter ミラー）。
- Create: `src/features/pod/filter.rs` — `pod_filter_applicator(tx)` ファクトリ。
- Create: `src/features/pod/filter/parser.rs` — `parse_pod_filter(input)` = `parse_table_filter` ＋ Pod 列バリデータ（namespace 案内＋PodColumn 検証）。
- Modify: `src/features/pod.rs` — `mod filter;` 追加。
- Create: `src/features/pod/view/widgets/pod_filter_help.rs` — help dialog widget（`node_filter_help.rs` ミラー）。
- Modify: `src/features/pod/view/widgets.rs` — `mod pod_filter_help;` ＋ `pub(super) use ...`。
- Modify: `src/features/component_id.rs` — `POD_FILTER_HELP_DIALOG_ID` 定数追加。
- Modify: `src/features/pod/view/tab.rs` — `PodTab` に `pod_filter_help_dialog` フィールド追加・コンストラクタで作成。
- Modify: `src/workers/render/window.rs` — Pod の filter help dialog を window へ登録（既存のダイアログ登録経路に追加）。
- Modify: `src/features/pod/view/widgets/pod.rs` — `substring_applicator("NAME")` → `pod_filter_applicator(tx)`。

---

## Task 1: `PodMessage::Filter` ＋ `SharedPodFilter` ＋ poller の labelSelector 配線

**Files:**
- Modify: `src/features/pod/message.rs`
- Modify: `src/features/pod/kube/pod.rs`
- Modify: `src/workers/kube/controller.rs`

このタスクは atomic（enum variant 追加＋controller の Kube マッチ網羅＋poller のフィールド・URL 加工が同時に必要）。テンプレートは Node の `SharedNodeFilter` 配線（`src/features/node/kube/node.rs` と controller の Node 周辺）。

- [ ] **Step 1: `PodMessage::Filter` 追加（失敗するビルドで挙動確認）**

`src/features/pod/message.rs` の `enum PodMessage` を次に置き換える:

```rust
#[derive(Debug)]
pub enum PodMessage {
    Request(PodColumns),
    Poll(Result<KubeTable>),
    Filter(Option<String>),
}
```

- [ ] **Step 2: ビルドを実行（controller の Kube 網羅マッチが未対応で失敗を確認）**

Run: `cargo build 2>&1 | tail -20`
Expected: controller.rs で `non-exhaustive patterns: ... Filter(_) not covered` 等のコンパイルエラー（網羅マッチに新バリアントが無い）。

- [ ] **Step 3: `SharedPodFilter` 型と `PodPoller` フィールドを追加**

`src/features/pod/kube/pod.rs` の先頭付近の型エイリアス行（`SharedPodColumns = Arc<RwLock<PodColumns>>;` 付近）に追加:

```rust
pub type SharedPodFilter = Arc<RwLock<Option<String>>>;
```

`PodPoller` 構造体定義に `shared_pod_filter: SharedPodFilter` フィールドを追加（既存の `shared_target_namespaces`/`shared_pod_columns` の隣）。コンストラクタ `PodPoller::new(...)` のシグネチャと本体にも `shared_pod_filter: SharedPodFilter` を追加（Node の `NodePoller::new` のパターンを参考に、`SharedNodeFilter` の追加と同じ形）。

`async fn get_pod_info(&self)` の冒頭（現在 `let namespaces = ...; let pod_columns = ...;` の直後）に次を追加して、後続処理に渡すために label_selector を取得:

```rust
        let label_selector = self.shared_pod_filter.read().await.clone();
```

そして `self.get_pods_per_namespace(&namespaces, &pod_columns).await` を `self.get_pods_per_namespace(&namespaces, &pod_columns, label_selector.as_deref()).await` に変更。

- [ ] **Step 4: `get_pods_per_namespace` で per-namespace URL に `?labelSelector=` を付与**

`get_pods_per_namespace` のシグネチャに `label_selector: Option<&str>` を追加（既存の `namespaces` / `pod_columns` の後ろ）。`try_join_all(namespaces.iter().map(|ns| { … get_resource_per_namespace(&self.kube_client, Pod::url_path(&Default::default(), Some(ns)), &columns, …) … }))` の中で、URL を次のように組み立てて渡す（Node `get_node_table` 同型）:

```rust
        let label_selector = label_selector.map(|s| s.to_string());
        try_join_all(namespaces.iter().map(|ns| {
            let base_path = Pod::url_path(&Default::default(), Some(ns));
            let path = match label_selector.as_deref().filter(|s| !s.is_empty()) {
                Some(sel) => format!("{}?labelSelector={}", base_path, sel),
                None => base_path,
            };
            get_resource_per_namespace(
                &self.kube_client,
                path,
                &columns,
                move |row: &TableRow, indexes: &[usize]| {
                    // … 既存のクロージャ本体はそのまま …
                },
            )
        }))
        .await
```

（既存の `move |row, indexes|` クロージャ本体・`KubeTableRow` 構築は不変。`path` を作って `get_resource_per_namespace` に渡すだけが新規。`label_selector` は `try_join_all` クロージャ内で参照するため、ローカルに `.to_string()` で `Option<String>` を作って各イテレーションで `.as_deref()` するのが借用上安全。）

- [ ] **Step 5: controller で `SharedPodFilter` を構築・`PodPoller` に渡す・`Filter` ハンドラ追加**

`src/workers/kube/controller.rs` で:
- import に `SharedPodFilter` を追加（既存の `SharedPodColumns` の隣、`use ...workers::kube::{..., SharedPodFilter}` 等の経路で）。
- `shared_pod_columns` を作っている付近に `let shared_pod_filter: SharedPodFilter = Arc::new(RwLock::new(None));` を追加（Node の `shared_node_filter` 構築と同じパターン）。
- `PodPoller` を構築している箇所に `shared_pod_filter: shared_pod_filter.clone()` を渡す。さらに `PodPoller::new(...)` 呼び出しに `shared_pod_filter.clone()` を引数として追加（Node 側 `NodePoller::new` への `shared_node_filter.clone()` 渡しと同形）。
- 既存の `Kube::Pod(PodMessage::Request(req)) => { ... }` の直後に、Node の `Kube::Node(NodeMessage::Filter(sel)) => { ... }` をミラーして追加:

```rust
                        Kube::Pod(PodMessage::Filter(sel)) => {
                            *shared_pod_filter.write().await = sel;
                        }
```

- [ ] **Step 6: ビルドとテストを実行**

Run: `cargo build 2>&1 | tail -10`
Expected: コンパイル成功（網羅マッチ解消）。

Run: `cargo test --all 2>&1 | rg "test result:"`
Expected: 全テスト PASS（既存挙動を壊していないこと）。

- [ ] **Step 7: コミット**

```bash
git add src/features/pod/message.rs src/features/pod/kube/pod.rs src/workers/kube/controller.rs
git commit -m "feat(pod): add PodMessage::Filter + SharedPodFilter + per-ns labelSelector"
```

---

## Task 2: Pod フィルタモジュール（parser ＋ applicator）を新設

**Files:**
- Create: `src/features/pod/filter.rs`
- Create: `src/features/pod/filter/parser.rs`
- Modify: `src/features/pod.rs`（`mod filter;` 追加・必要なら `pub use filter::pod_filter_applicator;`）

- [ ] **Step 1: parser.rs を作成（テストを含む）**

`src/features/pod/filter/parser.rs` を新規作成:

```rust
//! Pod filter parser.
//!
//! Delegates tokenization/quoting/predicate-building to the shared
//! `parse_table_filter`. The Pod-specific part is the column validator:
//! `namespace:` returns a guidance message (namespace is a scope, not a
//! column-level filter — use the namespace selector); other unknown columns
//! return `unknown column '<x>'`; built-in `PodColumn`s are accepted.

use std::collections::HashSet;

use strum::IntoEnumIterator;

use crate::{
    features::pod::pod_columns::PodColumn,
    ui::widget::{normalize_column_name, parse_table_filter, TableFilterPredicate},
};

/// Parse a Pod-filter input string into a `TableFilterPredicate`.
///
/// `namespace:` is rejected with a guidance message that points users to the
/// namespace selector (namespace is a scope, not a row attribute). Other
/// columns are validated against the builtin `PodColumn` set; a column not in
/// that set produces `unknown column '<x>'`. Label columns are not supported
/// for Pod yet (future work).
pub fn parse_pod_filter(input: &str) -> Result<TableFilterPredicate, String> {
    let valid: HashSet<String> = PodColumn::iter()
        .map(|c| normalize_column_name(c.display()))
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

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn empty_input_yields_empty_predicate() {
        let p = parse_pod_filter("").unwrap();
        assert!(p.column_includes.is_empty());
        assert!(p.column_excludes.is_empty());
        assert_eq!(p.label_selector, None);
    }

    #[test]
    fn bare_value_becomes_name_include() {
        let p = parse_pod_filter("nginx").unwrap();
        let patterns = p.column_includes.get("name").expect("name column");
        assert!(patterns[0].is_match("nginx-abc"));
    }

    #[test]
    fn builtin_columns_are_accepted() {
        let p = parse_pod_filter("status:Running !ready:0/1").unwrap();
        assert!(p.column_includes.contains_key("status"));
        assert!(p.column_excludes.contains_key("ready"));
    }

    #[test]
    fn multiword_builtin_via_normalization() {
        // NOMINATED NODE は builtin。nominatednode / nominated-node どちらも受理。
        assert!(parse_pod_filter("nominatednode:foo").is_ok());
        assert!(parse_pod_filter("nominated-node:foo").is_ok());
    }

    #[test]
    fn label_selector_is_captured() {
        let p = parse_pod_filter("label:app=nginx").unwrap();
        assert_eq!(p.label_selector.as_deref(), Some("app=nginx"));
    }

    #[test]
    fn unknown_column_produces_parse_error() {
        let err = parse_pod_filter("staus:Running").unwrap_err();
        assert!(
            err.contains("unknown column") && err.contains("staus"),
            "got: {}",
            err
        );
    }

    #[test]
    fn namespace_returns_guidance_message_not_unknown_column() {
        let err = parse_pod_filter("namespace:default").unwrap_err();
        assert_eq!(
            err,
            "namespace is selected via the namespace selector, not the filter"
        );
        // Case / format-insensitive variants also hit the guidance.
        let err2 = parse_pod_filter("NAMESPACE:default").unwrap_err();
        assert_eq!(
            err2,
            "namespace is selected via the namespace selector, not the filter"
        );
    }

    #[test]
    fn quoted_value_with_whitespace() {
        let p = parse_pod_filter(r#"status:"CreateContainerConfigError""#).unwrap();
        let patterns = p.column_includes.get("status").unwrap();
        assert!(patterns[0].is_match("CreateContainerConfigError"));
    }
}
```

- [ ] **Step 2: filter.rs（applicator factory）を作成**

`src/features/pod/filter.rs` を新規作成:

```rust
//! Pod tab filter: parser + `TableFilterApplicator` factory.
//!
//! The applicator wires `parse_pod_filter` (which builds on the shared
//! `parse_table_filter`) into the Table widget with `EnterToConfirm` strategy.
//! Server-side `labelSelector` is forwarded to the Pod poller via
//! `PodMessage::Filter` from `on_apply`/`on_cancel`. Typing `?` or `help` in
//! the filter input opens the `POD_FILTER_HELP_DIALOG_ID` dialog.

mod parser;

use crossbeam::channel::Sender;

use crate::{
    features::{component_id::POD_FILTER_HELP_DIALOG_ID, pod::message::PodMessage},
    message::Message,
    ui::{
        widget::{
            ApplyStrategy,
            OnFilterApply,
            OnFilterCancel,
            TableFilterApplicator,
            TableFilterParser,
            TableFilterPredicate,
        },
        Window,
    },
};

pub use parser::parse_pod_filter;

/// Build the Pod tab's filter applicator.
///
/// `tx` is captured by `on_apply`/`on_cancel` to forward the parsed
/// `label_selector` to the Pod poller via `PodMessage::Filter`.
///
/// The applicator uses `EnterToConfirm` so the parser only runs on Enter
/// (avoids server-side roundtrips mid-typing).
pub fn pod_filter_applicator(tx: Sender<Message>) -> TableFilterApplicator {
    let parser: TableFilterParser = (move |input: &str| parse_pod_filter(input)).into();

    let tx_apply = tx.clone();
    let tx_cancel = tx;

    let on_apply: OnFilterApply = (move |predicate: &TableFilterPredicate, _window: &mut Window| {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applicator_constructs_without_panic() {
        let (tx, _rx) = crossbeam::channel::bounded(1);
        let _ = pod_filter_applicator(tx);
    }
}
```

- [ ] **Step 3: `mod filter;` を Pod のモジュールに追加し公開**

`src/features/pod.rs` を開き、既存の `pub mod kube;` `pub mod message;` `pub mod view;` 等のモジュール宣言の隣に追加する（具体行は隣接の既存宣言に合わせる）:

```rust
mod filter;

pub use filter::pod_filter_applicator;
```

`POD_FILTER_HELP_DIALOG_ID` は Task 3 で `component_id.rs` に追加されるが、Task 2 内では参照しているのでビルド時に未定義エラーが出る。Task 3 で `POD_FILTER_HELP_DIALOG_ID` を足してから Task 2 のテストを回す方が順序的に楽 — ただし Task 2 のテストは `parse_pod_filter` 単体テストと `pod_filter_applicator` 構築テストで、後者は `POD_FILTER_HELP_DIALOG_ID` を参照する。順序として **Task 2 と Task 3 を同じコミット範囲で達成**するため、本 Step では Task 3 を先に済ませてから戻ってもよい。実装時は Task 3 の Step 1（component_id 追加）だけ先に済ませると Task 2 のビルドが通る。

- [ ] **Step 4: テスト実行**

Run: `cargo test features::pod::filter 2>&1 | tail -30`
Expected: parser テスト（8件）＋applicator 構築テストが PASS。Task 3 Step 1 を先に行っていれば pod_filter_applicator もコンパイル可。

- [ ] **Step 5: コミット**

```bash
git add src/features/pod/filter.rs src/features/pod/filter/parser.rs src/features/pod.rs
git commit -m "feat(pod): add pod_filter_applicator + parse_pod_filter (Z for namespace)"
```

---

## Task 3: help dialog widget ＋ component_id ＋ PodTab 配線

**Files:**
- Modify: `src/features/component_id.rs`
- Create: `src/features/pod/view/widgets/pod_filter_help.rs`
- Modify: `src/features/pod/view/widgets.rs`
- Modify: `src/features/pod/view/tab.rs`
- Modify: `src/workers/render/window.rs`

- [ ] **Step 1: `POD_FILTER_HELP_DIALOG_ID` 定数を追加**

`src/features/component_id.rs` の既存 `NODE_FILTER_HELP_DIALOG_ID` 定数の隣に追加:

```rust
pub const POD_FILTER_HELP_DIALOG_ID: &str = "pod-filter-help";
```

（既存の Pod 関連 ID `POD_WIDGET_ID` / `POD_COLUMNS_DIALOG_ID` 等のグループに置いてもよい。プロジェクトの並びに合わせる。）

- [ ] **Step 2: `pod_filter_help.rs` widget を作成**

`src/features/pod/view/widgets/pod_filter_help.rs` を新規作成（`src/features/node/view/widgets/node_filter_help.rs` をミラー、文言は Pod 文脈に調整）:

```rust
use indoc::indoc;
use ratatui::crossterm::event::KeyCode;

use crate::{
    config::theme::WidgetThemeConfig,
    features::component_id::POD_FILTER_HELP_DIALOG_ID,
    message::UserEvent,
    ui::{
        event::EventResult,
        widget::{SearchForm, SearchFormTheme, Text, TextTheme, Widget, WidgetBase, WidgetTheme},
        Window,
    },
};

pub fn pod_filter_help_widget(theme: WidgetThemeConfig) -> Widget<'static> {
    let widget_theme = WidgetTheme::from(theme.clone());
    let text_theme = TextTheme::from(theme.clone());
    let search_theme = SearchFormTheme::from(theme);

    let widget_base = WidgetBase::builder()
        .title("Pod Filter Help")
        .theme(widget_theme)
        .build();

    let search_form = SearchForm::builder().theme(search_theme).build();

    Text::builder()
        .id(POD_FILTER_HELP_DIALOG_ID)
        .widget_base(widget_base)
        .search_form(search_form)
        .theme(text_theme)
        .items(content())
        .action(UserEvent::from(KeyCode::Enter), close_dialog())
        .build()
        .into()
}

fn content() -> Vec<String> {
    indoc! {r#"
        Usage: TERM [ TERM ]...

        Terms:
           <value>            Plain value: NAME include (regex).
           NAME:<regex>       Include pods where NAME matches.
           STATUS:<regex>     Include where STATUS matches. Multiple
                              same-column includes are OR (in-list).
           !<COL>:<regex>     Exclude pods whose COL matches.
           label:<selector>   Kubernetes labelSelector, applied
                              server-side (e.g. app=nginx,env=prod).
                              Last 'label:' wins if repeated.

        Quoting (values with spaces):
           "value with spaces"           Double-quoted value
           'value with spaces'           Single-quoted value
           \" \' \\                      Literal " ' \ inside quotes
           \<other>                      Backslash preserved (regex \s etc.)

        Combining:
           Same column, multiple includes  ->  OR (in-list)
           Different columns, includes     ->  AND across columns
           Any matching exclude            ->  row excluded
           Bare values                     ->  treated as NAME includes

        Examples
           nginx                           Show pods whose NAME matches 'nginx'
           NAME:web STATUS:Running         NAME~web AND STATUS~Running
           STATUS:Running STATUS:Pending   STATUS in (Running, Pending)
           !NAME:test label:app=nginx      Server-side label filter + name exclude
           STATUS:"CreateContainerConfigError"
                                           Quoted value with whitespace

        Columns must be builtin pod columns; unknown columns produce
        an error. A term on a column that is not currently shown
        becomes inactive (kept, but not applied) until that column
        is shown again; the title shows (inactive: ...). Column names
        ignore case, spaces, '-' and '_'. The 'namespace' column is
        not filterable — use the namespace selector. Press Enter to
        apply, Esc to cancel. Type ? or help in the filter input to
        open this help.
    "# }
    .lines()
    .map(ToString::to_string)
    .collect()
}

fn close_dialog() -> impl Fn(&mut Window) -> EventResult {
    move |w: &mut Window| {
        w.close_dialog();
        EventResult::Nop
    }
}
```

- [ ] **Step 3: widgets モジュールに登録**

`src/features/pod/view/widgets.rs` に追加（既存の `mod pod;` / `mod pod_columns_dialog;` 等の隣）:

```rust
mod pod_filter_help;

pub(super) use pod_filter_help::pod_filter_help_widget;
```

- [ ] **Step 4: `PodTab` に help dialog フィールドを追加**

`src/features/pod/view/tab.rs` の `PodTab` 構造体と `PodTab::new` を更新（既存の `pod_columns_dialog` フィールドと同じパターン）:

- `PodTab` struct に `pub pod_filter_help_dialog: Widget<'static>` を追加。
- `PodTab::new` の本体で `let pod_filter_help_dialog = pod_filter_help_widget(theme.clone());` を作り、構造体リテラルの返却部分に `pod_filter_help_dialog,` を追加。
- `use super::widgets::{ … pod_filter_help_widget, … };` も追加。

- [ ] **Step 5: window に Pod の filter help dialog を登録**

`src/workers/render/window.rs` を開き、既存の `node_filter_help_dialog` が window に登録されている箇所を探す（`tab.node_filter_help_dialog` / `add_dialog`/`open_dialog` 等の経路）。同じ場所で Pod の `pod_filter_help_dialog` も登録する。具体的にはコード上の `node_filter_help_dialog` 行をモデルに、`pod_filter_help_dialog` を同様に追加（Pod の他のダイアログ `pod_columns_dialog` / `log_query_help_dialog` の登録も近くにあるはずなので、それと並べる）。

- [ ] **Step 6: ビルドとテスト**

Run: `cargo test --all 2>&1 | rg "test result:"`
Expected: 全テスト PASS。Pod の applicator 構築テスト（Task 2）も `POD_FILTER_HELP_DIALOG_ID` が解決して通る。

Run: `cargo build 2>&1 | tail -10`
Expected: コンパイル成功。

- [ ] **Step 7: コミット**

```bash
git add src/features/component_id.rs src/features/pod/view/widgets/pod_filter_help.rs src/features/pod/view/widgets.rs src/features/pod/view/tab.rs src/workers/render/window.rs
git commit -m "feat(pod): pod_filter_help widget + POD_FILTER_HELP_DIALOG_ID + tab wiring"
```

---

## Task 4: Pod widget を `pod_filter_applicator` に載せ替え

**Files:**
- Modify: `src/features/pod/view/widgets/pod.rs`

- [ ] **Step 1: filter_applicator を差し替え**

`src/features/pod/view/widgets/pod.rs` の import を更新（`substring_applicator` を取り除き、`pod_filter_applicator` を入れる）。具体的には:

- 既存の `use crate::ui::widget::{ ... substring_applicator ... };` から `substring_applicator,` を削除。
- 既存の Pod モジュール経由の import に `pod_filter_applicator` を追加（`crate::features::pod::pod_filter_applicator` の経路、Task 2 Step 3 で `pub use filter::pod_filter_applicator;` を加えた）。

`pod_widget` 関数内の `.filter_applicator(substring_applicator("NAME"))` 行（現状 pod.rs:59 付近）を次に置き換える:

```rust
        .filter_applicator(pod_filter_applicator(tx.clone()))
```

（`tx` は同関数で既に `let tx = tx.clone();` でローカル化されている。クローン共有しないなら `tx.clone()` で渡す。）

- [ ] **Step 2: ビルドとテスト**

Run: `cargo test --all 2>&1 | rg "test result:"`
Expected: 全テスト PASS。Pod 一覧の挙動はキー仕様が変わる（Live substring → EnterToConfirm column-aware）が、ビルド・自動テストには影響しない（既存テストは TUI ロジックを直接触らない）。

- [ ] **Step 3: コミット**

```bash
git add src/features/pod/view/widgets/pod.rs
git commit -m "feat(pod): wire pod_filter_applicator into the Pod widget"
```

---

## Task 5: 全体検証

**Files:** なし（検証のみ）

- [ ] **Step 1: 全テスト**

Run: `cargo test --all 2>&1 | rg "test result:"`
Expected: 全テスト PASS。新規 Pod parser テスト（8件）と applicator 構築テスト（1件）含む。

- [ ] **Step 2: clippy**

Run: `cargo clippy --all-targets 2>&1 | rg "src/features/pod|workers/kube/controller|workers/kube/pod"`
Expected: 変更ファイルに新規警告なし。

- [ ] **Step 3: format**

Run: `cargo +nightly fmt --check 2>&1 | rg "Diff in" | rg -v "store.rs"`
Expected: 変更ファイルに差分なし（出力空。store.rs は main 由来の既存 drift で対象外）。

- [ ] **Step 4: 手動スモーク（実クラスタ／KIND、不可なら省略明記）**

`cargo run` で Pod タブを開いて確認:
1. `/` → `status:Running` → Enter → STATUS が Running の Pod だけ。
2. `/` → `staus:Running`（タイポ）→ Enter → `unknown column 'staus'` エラー。
3. `/` → `namespace:default` → Enter → `namespace is selected via the namespace selector, not the filter` 案内（generic な unknown column ではない）。
4. `/` → `label:app=nginx`（実在ラベル）→ Enter → サーバーサイドで pods が絞られる（複数 ns 表示時は per-ns 全てに `?labelSelector=` 付与）。
5. 列ダイアログ（`c` 等、Pod の列ダイアログのキー）で STATUS を非表示 → `status:Running` フィルタは inactive、`(inactive: status)` バッジ、行は残る（残りの可視列で絞られる）→ 再表示で自動復活。
6. `/` → `?` → Pod Filter Help が更新文言で開く。
7. Esc → label: サーバーサイドフィルタがクリアされ全 pod に戻る。
8. 複数 ns 選択時に NAMESPACE 列が表示される（変更前と同様）。

- [ ] **Step 5: （必要なら）fmt 修正をコミット**

```bash
git add -A
git commit -m "style: cargo fmt"
```

---

## Self-Review

- **設計カバレッジ:**
  - `PodMessage::Filter` ＋ `SharedPodFilter` ＋ per-ns labelSelector → Task 1
  - `parse_pod_filter`（PodColumn 検証＋namespace 案内）→ Task 2 Step 1
  - `pod_filter_applicator`（EnterToConfirm／help／on_apply/on_cancel → PodMessage::Filter）→ Task 2 Step 2
  - help dialog ＋ `POD_FILTER_HELP_DIALOG_ID` ＋ PodTab ＋ window 登録 → Task 3
  - widget 載せ替え → Task 4
  - 検証 → Task 5
  - inactive/バッジ/正規化は framework 由来で自動（追加実装不要）
  - Pod ログクエリは別概念で不変（触らない）
- **プレースホルダ:** TBD/TODO なし。controller / window.rs の追加位置は「Node の SharedNodeFilter / node_filter_help_dialog をミラー」という具体的かつ参照可能な指示。Task 3 Step 3-5 のモジュール宣言・タブ更新はパターンが既存にあり機械的。
- **型整合:** `SharedPodFilter = Arc<RwLock<Option<String>>>`（Task 1）と `*shared_pod_filter.write().await = sel;`（controller、`sel: Option<String>`）一致。`PodMessage::Filter(Option<String>)`（Task 1）と `on_apply`/`on_cancel` の `PodMessage::Filter(predicate.label_selector.clone() / None)`（Task 2）一致。`pod_filter_applicator(tx: Sender<Message>)`（Task 2）と `.filter_applicator(pod_filter_applicator(tx.clone()))`（Task 4）一致。`parse_pod_filter(&str) -> Result<TableFilterPredicate, String>`（Task 2 parser）と applicator のクロージャ `move |input: &str| parse_pod_filter(input)` 一致。
