# Node タブ — Plan 2: ランタイム列ダイアログ実装プラン

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Node 一覧で `t` キーを押すと列選択ダイアログ（CheckList）が開き、チェックした列が即座に一覧へ反映されるようにする（Pod タブの列ダイアログと同等）。

**Architecture:** Pod の列ダイアログを踏襲。ダイアログの `on_change` が `NodeMessage::Request(NodeColumns)` を送信 → kube ワーカーの `EventController` が `shared_node_columns` を更新 → `NodePoller` が次回ポーリングで新しい列を反映。ダイアログの初期チェック状態は、設定から解決した `default_node_columns` を `Render → WindowInit → NodeTab → node_columns_dialog` と渡して反映する。

**Tech Stack:** Rust 2021, ratatui (CheckList widget), crossbeam channel, tokio, strum, pretty_assertions。

**Scope（このプラン）:** `t` キーの列ダイアログ、`NodeMessage::Request` 処理、`default_node_columns` のダイアログ初期値反映。**含まない**: ラベル列（Plan 3）、詳細ペイン（Plan 4）、フィルタ（Plan 5）。

**前提:** Plan 1 完了済み（`NodeColumns`、`NodePoller`、`shared_node_columns`、Node タブ一覧、`theme.node` プリセット、CLI フラグ）。

**設計スペック:** `docs/superpowers/specs/2026-05-22-node-tab-design.md`（「列設定」節）。

---

## ファイル構成

新規:
- `src/features/node/view/widgets/node_columns_dialog.rs` — 列選択 CheckList ダイアログ

変更:
- `src/features/node/message.rs` — `NodeMessage` に `Request(NodeColumns)` 追加
- `src/features/node/view/widgets.rs` — `node_columns_dialog` を公開
- `src/features/node/view/widgets/node.rs` — 一覧 Table に `t` アクション追加
- `src/features/node/view/tab.rs` — `NodeTab::new` が `default_columns` を受け取り、ダイアログを構築して返す
- `src/features/component_id.rs` — `node_columns_dialog`
- `src/workers/kube/controller.rs` — `EventController` に `shared_node_columns` と `NodeMessage::Request` 処理を追加
- `src/workers/render.rs` — `Render` に `default_node_columns` を追加し `WindowInit` へ
- `src/workers/render/window.rs` — `WindowInit` に `default_node_columns`、`NodeTab::new` 呼び出し更新、ダイアログ登録
- `src/app.rs` — `default_node_columns` を `Render::new` へ渡す

---

## Task 1: NodeMessage に Request を追加

**Files:** Modify `src/features/node/message.rs`

- [ ] **Step 1: 実装**

`src/features/node/message.rs` を次のようにする（`NodeColumns` を import し `Request` を追加）:

```rust
use anyhow::Result;

use crate::{kube::table::KubeTable, message::Message, workers::kube::message::Kube};

use super::NodeColumns;

#[derive(Debug)]
pub enum NodeMessage {
    Request(NodeColumns),
    Poll(Result<KubeTable>),
}

impl From<NodeMessage> for Message {
    fn from(m: NodeMessage) -> Message {
        Message::Kube(Kube::Node(m))
    }
}
```

- [ ] **Step 2: ビルド確認**

Run: `cargo build`
Expected: 成功（`Request` はまだ送信・処理されないが、`EventController` には後続タスクで catch されるまで送信側が無いのでパニックしない）。

- [ ] **Step 3: Commit**

```bash
git add src/features/node/message.rs
git commit -m "feat(node): add NodeMessage::Request variant"
```

---

## Task 2: EventController で NodeMessage::Request を処理

**Files:** Modify `src/workers/kube/controller.rs`

`PodMessage::Request` が `shared_pod_columns` を更新するのと同じ仕組みを Node に追加する。`SharedNodeColumns` は `crate::features::node::kube::SharedNodeColumns`（Plan 1 で定義済み）。

- [ ] **Step 1: import 追加**

`controller.rs` の features import で、Plan 1 で追加した `node::kube::{NodeConfig, NodePoller}` を `node::kube::{NodeConfig, NodePoller, SharedNodeColumns}` に拡張。また `node::message::NodeMessage` を import 群に追加（`pod::message::{LogMessage, PodMessage}` の近く）。

- [ ] **Step 2: EventControllerArgs と EventController に field 追加**

`struct EventControllerArgs` の `shared_pod_columns: SharedPodColumns,` の直後に:

```rust
    shared_node_columns: SharedNodeColumns,
```

`struct EventController` の同じ位置（`shared_pod_columns: SharedPodColumns,` の直後）にも同様に追加。

`impl EventController { fn new(args) }` の `shared_pod_columns: args.shared_pod_columns,` の直後に:

```rust
            shared_node_columns: args.shared_node_columns,
```

`run()` 冒頭の `let EventController { ... shared_pod_columns, ... } = self;` 分解に `shared_node_columns,` を追加（`shared_pod_columns,` の直後）。

- [ ] **Step 3: args 構築箇所に shared_node_columns を渡す**

`run()`（`KubeController::run`）内、`EventControllerArgs { ... shared_pod_columns: shared_pod_columns.clone(), ... }` の `shared_pod_columns` の直後に:

```rust
                shared_node_columns: shared_node_columns.clone(),
```

（`shared_node_columns` は Plan 1 で `run()` 内に作成済み。）

- [ ] **Step 4: メッセージ処理の match arm を追加**

`EventController::run()` の match で `Kube::Pod(PodMessage::Request(req)) => { ... }` ブロックの直後に追加:

```rust
                        Kube::Node(NodeMessage::Request(req)) => {
                            let mut node_columns = shared_node_columns.write().await;
                            *node_columns = req;

                            logger!(info, "Node columns updated: {:#?}", node_columns);
                        }
```

- [ ] **Step 5: ビルド確認**

Run: `cargo build`
Expected: 成功。

- [ ] **Step 6: Commit**

```bash
git add src/workers/kube/controller.rs
git commit -m "feat(node): handle NodeMessage::Request in EventController"
```

---

## Task 3: component_id に node_columns_dialog を追加

**Files:** Modify `src/features/component_id.rs`

- [ ] **Step 1: 実装**

`component_id!( ... )` の dialogs グループ（`pod_columns_dialog,` がある箇所）に `node_columns_dialog,` を追加。

- [ ] **Step 2: ビルド確認**

Run: `cargo build`
Expected: 成功（`NODE_COLUMNS_DIALOG_ID` 生成）。

- [ ] **Step 3: Commit**

```bash
git add src/features/component_id.rs
git commit -m "feat(node): add NODE_COLUMNS_DIALOG_ID"
```

---

## Task 4: 列選択ダイアログウィジェット

**Files:** Create `src/features/node/view/widgets/node_columns_dialog.rs`; Modify `src/features/node/view/widgets.rs`

`src/features/pod/view/widgets/pod_columns_dialog.rs` を踏襲（Pod→Node 置換）。TDD。

- [ ] **Step 1: 失敗するテストを書く**

`src/features/node/view/widgets/node_columns_dialog.rs` の末尾に:

```rust
#[cfg(test)]
mod tests {
    #![allow(non_snake_case)]
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn 既存カラムをチェック済みで先頭に_残りを未チェックで並べる() {
        let node_columns = NodeColumns::new([NodeColumn::Name, NodeColumn::Status]);
        let items = build_check_list_items_from_existing(node_columns);

        // 先頭2つは指定カラム（チェック済み）
        assert_eq!(items[0].label, "NAME");
        assert!(items[0].checked && items[0].required);
        assert_eq!(items[1].label, "STATUS");
        assert!(items[1].checked && !items[1].required);
        // 残りは未チェックで全 NodeColumn 数に一致
        assert_eq!(items.len(), NodeColumn::iter().count());
        assert!(items[2..].iter().all(|i| !i.checked));
    }

    #[test]
    fn デフォルトカラムがチェック済みで構築される() {
        let items = build_default_check_list_items();
        let checked: Vec<&str> = items
            .iter()
            .filter(|i| i.checked)
            .map(|i| i.label.as_str())
            .collect();
        assert_eq!(checked, vec!["NAME", "STATUS", "ROLES", "AGE", "VERSION"]);
        assert!(items.iter().find(|i| i.label == "NAME").unwrap().required);
    }
}
```

- [ ] **Step 2: テストが失敗することを確認**

Run: `cargo test features::node::view`
Expected: コンパイルエラー（未定義）。

- [ ] **Step 3: 実装**

`src/features/node/view/widgets/node_columns_dialog.rs` の先頭:

```rust
use std::str::FromStr as _;

use crossbeam::channel::Sender;
use strum::IntoEnumIterator;

use crate::{
    config::theme::WidgetThemeConfig,
    features::{
        component_id::NODE_COLUMNS_DIALOG_ID,
        node::{message::NodeMessage, NodeColumn, NodeColumns, DEFAULT_NODE_COLUMNS},
    },
    message::Message,
    ui::{
        event::EventResult,
        widget::{CheckList, CheckListItem, CheckListTheme, Widget, WidgetBase, WidgetTheme},
        Window,
    },
};

pub fn node_columns_dialog(
    tx: &Sender<Message>,
    default_columns: Option<NodeColumns>,
    theme: WidgetThemeConfig,
) -> Widget<'static> {
    let check_list_theme = CheckListTheme::from(theme.clone());
    let widget_theme = WidgetTheme::from(theme.clone());
    let widget_base = WidgetBase::builder()
        .title("Node Columns")
        .theme(widget_theme)
        .build();

    let check_list_items = build_check_list_items(default_columns);

    CheckList::builder()
        .id(NODE_COLUMNS_DIALOG_ID)
        .widget_base(widget_base)
        .theme(check_list_theme)
        .items(check_list_items)
        .on_change(on_change(tx.clone()))
        .build()
        .into()
}

fn on_change(tx: Sender<Message>) -> impl Fn(&mut Window, &CheckListItem) -> EventResult {
    move |w: &mut Window, _v| {
        let widget = w.find_widget_mut(NODE_COLUMNS_DIALOG_ID).as_mut_check_list();

        let items = widget
            .items()
            .iter()
            .filter(|item| item.required || item.checked)
            .filter_map(|i| NodeColumn::from_str(&i.label).ok())
            .collect::<Vec<_>>();

        tx.send(NodeMessage::Request(NodeColumns::new(items)).into())
            .expect("Failed to send NodeMessage::Request");

        EventResult::Nop
    }
}

fn build_check_list_items(default_columns: Option<NodeColumns>) -> Vec<CheckListItem> {
    match default_columns {
        Some(columns) => {
            build_check_list_items_from_existing(columns.ensure_name_column().dedup_columns())
        }
        None => build_default_check_list_items(),
    }
}

fn build_check_list_items_from_existing(node_columns: NodeColumns) -> Vec<CheckListItem> {
    node_columns
        .columns()
        .iter()
        .map(|column| make_item(*column, true))
        .chain(
            NodeColumn::iter()
                .filter(|c| !node_columns.columns().contains(c))
                .map(|column| make_item(column, false)),
        )
        .collect()
}

fn build_default_check_list_items() -> Vec<CheckListItem> {
    NodeColumn::iter()
        .map(|column| {
            let checked = DEFAULT_NODE_COLUMNS.contains(&column);
            make_item(column, checked)
        })
        .collect()
}

fn make_item(column: NodeColumn, checked: bool) -> CheckListItem {
    CheckListItem {
        label: column.display().to_string(),
        checked,
        required: column == NodeColumn::Name,
        metadata: None,
    }
}
```

注: `NodeColumns` の `ensure_name_column` / `dedup_columns` / `columns` は Plan 1 で実装済み。`DEFAULT_NODE_COLUMNS` も公開済み。`NodeColumn` は `display()` と `from_str` を持つ。`IntoEnumIterator`（`NodeColumn::iter()`）の import を忘れない。もし `CheckList` builder の API がここで示したものと異なる場合は `src/features/pod/view/widgets/pod_columns_dialog.rs` の実コードに合わせること。

- [ ] **Step 4: widgets.rs に登録**

`src/features/node/view/widgets.rs` を次のようにする:

```rust
mod node;
mod node_columns_dialog;

pub use node::*;
pub use node_columns_dialog::*;
```

- [ ] **Step 5: テスト実行**

Run: `cargo test features::node::view`
Expected: PASS（2 テスト）。`cargo build` も成功（ダイアログはまだ未使用だが pub なので警告は出にくい。出る場合は後続タスクで解消）。

- [ ] **Step 6: Commit**

```bash
git add src/features/node/view/widgets/node_columns_dialog.rs src/features/node/view/widgets.rs
git commit -m "feat(node): add node columns selection dialog widget"
```

---

## Task 5: 一覧 Table に `t` アクションを追加

**Files:** Modify `src/features/node/view/widgets/node.rs`

- [ ] **Step 1: 実装**

`src/features/node/view/widgets/node.rs` を更新。import に `EventResult`, `Window`, `NODE_COLUMNS_DIALOG_ID` を追加し、`Table::builder()` チェーンに `.action('t', open_node_columns_dialog())` を追加する。

import 部:

```rust
use crate::{
    config::theme::WidgetThemeConfig,
    features::component_id::{NODE_COLUMNS_DIALOG_ID, NODE_WIDGET_ID},
    ui::{
        event::EventResult,
        widget::{
            FilterForm, FilterFormTheme, Table, TableTheme, Widget, WidgetBase, WidgetTheme,
        },
        Window,
    },
};
```

`Table::builder()` チェーンの `.filtered_key("NAME")` の直後に:

```rust
        .action('t', open_node_columns_dialog())
```

ファイル末尾（`node_widget` 関数の外）に:

```rust
fn open_node_columns_dialog() -> impl Fn(&mut Window) -> EventResult {
    move |w: &mut Window| {
        w.open_dialog(NODE_COLUMNS_DIALOG_ID);
        EventResult::Nop
    }
}
```

- [ ] **Step 2: ビルド確認**

Run: `cargo build`
Expected: 成功。（ダイアログはまだ Window に登録されていないので `t` は実際には開かないが、コンパイルは通る。登録は Task 6。）

- [ ] **Step 3: Commit**

```bash
git add src/features/node/view/widgets/node.rs
git commit -m "feat(node): open columns dialog with 't' key on node list"
```

---

## Task 6: default_node_columns をダイアログまで通し、Window に登録

**Files:** Modify `src/features/node/view/tab.rs`, `src/workers/render/window.rs`, `src/workers/render.rs`, `src/app.rs`

Pod の `default_pod_columns` の流れ（`app.rs → Render → WindowInit → PodTab → pod_columns_dialog`、ダイアログは window.rs の dialogs に登録）を Node にも作る。

- [ ] **Step 1: NodeTab を更新**

`src/features/node/view/tab.rs` を次のようにする（`default_columns` を受け取り、ダイアログを構築して返す）:

```rust
use ratatui::layout::{Constraint, Direction};

use crossbeam::channel::Sender;

use crate::{
    config::theme::WidgetThemeConfig,
    features::{
        component_id::NODE_TAB_ID,
        node::NodeColumns,
    },
    message::Message,
    ui::{
        tab::{LayoutElement, NestedLayoutElement, NestedWidgetLayout, TabLayout},
        widget::Widget,
        Tab,
    },
};

use super::widgets::{node_columns_dialog, node_widget};

pub struct NodeTab {
    pub tab: Tab<'static>,
    pub node_columns_dialog: Widget<'static>,
}

impl NodeTab {
    pub fn new(
        title: &'static str,
        tx: &Sender<Message>,
        default_columns: Option<NodeColumns>,
        theme: WidgetThemeConfig,
    ) -> Self {
        let node_widget = node_widget(theme.clone());
        let node_columns_dialog = node_columns_dialog(tx, default_columns, theme);

        let tab = Tab::new(
            NODE_TAB_ID,
            title,
            [node_widget],
            TabLayout::new(layout, Direction::Vertical),
        );

        Self {
            tab,
            node_columns_dialog,
        }
    }
}

fn layout(_split_direction: Direction) -> NestedWidgetLayout {
    NestedWidgetLayout::default().nested_widget_layout([NestedLayoutElement(
        Constraint::Percentage(100),
        LayoutElement::WidgetIndex(0),
    )])
}
```

- [ ] **Step 2: Render に default_node_columns を追加**

`src/workers/render.rs`:
- `Render` 構造体の `default_pod_columns: Option<PodColumns>,` の直後に `default_node_columns: Option<NodeColumns>,` を追加（`use crate::features::node::NodeColumns;` を import）。
- `Render::new` の引数に `default_node_columns: Option<NodeColumns>,` を `default_pod_columns` の直後に追加し、構造体初期化にも追加。
- `WindowInit::new(...)` 呼び出し（`self.default_pod_columns.clone()` の箇所）に `self.default_node_columns.clone(),` を直後に追加。

- [ ] **Step 3: WindowInit に default_node_columns を追加し、NodeTab を更新・ダイアログ登録**

`src/workers/render/window.rs`:
- import に `node::{view::NodeTab, NodeColumns}` のように `NodeColumns` を追加（既存の `node::view::NodeTab` を拡張）。
- `WindowInit` 構造体の `default_pod_columns: Option<PodColumns>,` の直後に `default_node_columns: Option<NodeColumns>,` を追加。
- `WindowInit::new` の引数（`default_pod_columns` の直後）に `default_node_columns: Option<NodeColumns>,` を追加し、構造体初期化にも追加。
- `tabs_dialogs()` の `NodeTab::new("Node", self.theme.component.clone())` を以下に変更:

```rust
        let NodeTab {
            tab: node_tab,
            node_columns_dialog,
        } = NodeTab::new(
            "Node",
            &self.tx,
            self.default_node_columns.clone(),
            self.theme.component.clone(),
        );
```

- `dialog_widgets` ベクタ（`pod_columns_dialog,` がある）に `node_columns_dialog,` を追加。

- [ ] **Step 4: app.rs から default_node_columns を渡す**

`src/app.rs`:
- `let default_pod_columns = kube_worker_config.pod_config.default_columns.clone();` の直後に:

```rust
        let default_node_columns = kube_worker_config.node_config.default_columns.clone();
```

- `Render::new(...)` 呼び出しの `default_pod_columns,` の直後に `default_node_columns,` を追加。

- [ ] **Step 5: ビルド・テスト・実機確認**

Run: `cargo build`
Expected: 成功。

Run: `cargo run -- --config-file example/config.yaml`（クラスタ接続）
Expected: Node タブ（キー5）で `t` を押すと「Node Columns」ダイアログが開き、チェックを変更すると一覧の列が変わる。

- [ ] **Step 6: Commit**

```bash
git add src/features/node/view/tab.rs src/workers/render.rs src/workers/render/window.rs src/app.rs
git commit -m "feat(node): wire columns dialog with default_node_columns"
```

---

## Task 7: 仕上げ（fmt / clippy / 全テスト）

- [ ] **Step 1: フォーマット**

Run: `cargo +nightly fmt`
（注: main は整形済みなので、変更ファイルのみが対象になるはず。万一無関係ファイルが変わる場合はコミットしない。）

- [ ] **Step 2: Lint**

Run: `cargo clippy --all-targets`
Expected: 新規コードに警告なし。

- [ ] **Step 3: 全テスト**

Run: `cargo test`
Expected: 全 PASS。

- [ ] **Step 4: Commit（fmt 差分があれば）**

```bash
git add -A
git commit -m "chore(node): fmt and clippy for columns dialog"
```

---

## 後続プラン

- **Plan 3**: ラベル列（`label_columns` レジストリ＋ `NodeColumn::Label{key,name}`、`includeObject=Metadata` でラベル抽出、設定衝突/未定義参照の読込時バリデーション）。
- **Plan 4**: 詳細ペイン（2 ペイン化、`NodeDetailWorker`：Node YAML〔managedFields 除去〕＋関連 Pod、3 秒更新、`on_select`）。
- **Plan 5**: フィルタ（`node:`/`!node:`/`label:`、nom パーサ、`shared_node_filter`、`labelSelector`、フィルタ入力ウィジェット＋ヘルプダイアログ）。
