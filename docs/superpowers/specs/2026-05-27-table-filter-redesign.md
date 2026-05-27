# Table 高度フィルタ機構 設計

- 関連 Issue: [#920 feat: add Node tab for viewing node information](https://github.com/sarub0b0/kubetui/issues/920)（Node タブ Plan 5 の差し戻しを含む）
- ステータス: ドラフト
- 作成日: 2026-05-27

## 概要

`Table` ウィジェットに**プラガブルなフィルタ機構**を導入する。共通フレームワークの上で、既存タブ（Pod / Config / Network / Event / API / Yaml / Context / Namespace）は今と同じ「単一列への部分一致 live フィルタ」を実現し、Node タブは列ベースの高度な構文（`<col>:<val>` / `!<col>:<val>` / `label:<sel>`）をプラグインとして実現する。Plan 5 のダイアログ方式は破棄し、Table 既存の `filter_form` を全タブで再活用する。

## 背景

Plan 5 で「Node タブ用のフィルタ」として独立ダイアログ + 独立メッセージを実装したが、3 つの問題が明らかになった:

1. **ダイアログが大きすぎる**: 1 行入力に対してダイアログサイズが過剰、kubetui の小ペイン設計と合わない。
2. **エラーがポーリングで消える**: フィルタ parse エラーを `set_widget_error(NODE_WIDGET_ID, ...)` で表示していたが、次ポーリングが `clear_widget_error` を呼ぶため消失。
3. **既存フィルタとの一貫性欠如**: 既存タブの `/` 部分一致フィルタと Node の `/` ダイアログが**別の UX**になっている。将来「任意列でフィルタしたい」要求が他タブにも出たとき、機構が 2 系統あることで設計が混乱する。

## ゴール / 非ゴール

### ゴール

- Table ウィジェットの `filter_form` UI を全タブで活用（Node 用に独立ダイアログを作らない）
- 既存タブの UX を保ったまま、内部実装を統一パスへ移行
- Node タブで以下の構文をサポート:
  - `<value>` → NAME 列に regex マッチ
  - `<column>:<value>` → 任意列に regex マッチ
  - `!<column>:<value>` → 任意列で除外
  - `label:<selector>` → k8s labelSelector（サーバ側絞り込み）
  - スペース区切り AND、複数項目可
- フィルタ parse エラーは**ポーリングで消えない**粘着状態として管理
- API は kubetui 既存パターン（`define_callback!` / enum dispatch）と整合

### 非ゴール

- 既存タブの構文を Node 同等の高度フィルタに自動拡張する（YAGNI、別タブで要求が出てから）
- ライブ filter と Enter filter を混在させる UI（タブごとに 1 つ選ぶ）
- フィルタ条件の永続化（in-memory のみ）

## UI / レイアウト

Table ウィジェットの既存 `filter_form`（1 行）を全タブで使う。

```
+-- Node [3/8]  filter: STATUS:Ready ----------+
| STATUS:Ready                                 |  <- filter_form (FilterConfirm モードで常駐)
+----------------------------------------------+
| NAME              STATUS  ROLES  AGE  VER    |
| gke-prod-001      Ready   worker 10d v1.30   |
| gke-prod-002      Ready   worker 10d v1.30   |
| ...                                          |
+----------------------------------------------+
```

`/` で入力モードへ、Enter で確定。確定後は filter_form が 1 行常駐し、現在の条件が見える。

## アーキテクチャ

### コア型

```rust
// src/ui/widget/table/filter_applicator.rs (新規)

/// パーサ・戦略・副作用・ヘルプ ID を 1 つに束ねたファクトリ。
/// Table::builder().filter_applicator(...) で渡す。
pub struct TableFilterApplicator {
    pub(crate) parser: TableFilterParser,            // define_callback!
    pub(crate) strategy: ApplyStrategy,              // Live | EnterToConfirm
    pub(crate) help_dialog_id: Option<&'static str>, // Some なら入力中 `?` で開く
    pub(crate) on_apply: Option<OnFilterApply>,      // define_callback!, 副作用フック
}

pub enum ApplyStrategy {
    /// 毎キーで parser を呼ぶ。空入力は「フィルタなし」相当。Substring 系で使う。
    Live,
    /// Enter のみで parser を呼ぶ。Node 系の高度構文で使う。
    EnterToConfirm,
}

define_callback!(
    pub TableFilterParser,
    Fn(&str) -> Result<TableFilterPredicate, String>
);

define_callback!(
    pub OnFilterApply,
    Fn(&TableFilterPredicate, &mut Window)
);

/// 全タブで共通のフィルタ判定。意味論は「列内 OR、列間 AND、exclude は any match
/// excludes」で統一されているため、既存タブ用と Node 用で構造を分ける必要はなく
/// 単一 struct で表現できる。
pub struct TableFilterPredicate {
    /// 列ごとの include patterns（同一列内は OR、列間は AND）。
    /// キーは小文字に正規化した列名。
    pub column_includes: HashMap<String, Vec<Regex>>,
    /// 列ごとの exclude patterns（いずれかにマッチすれば除外）。
    pub column_excludes: HashMap<String, Vec<Regex>>,
    /// k8s labelSelector（Node 系のみ、サーバ側で適用される。
    /// matches() では参照されず、on_apply 経由で poller に渡る）。
    pub label_selector: Option<String>,
    /// 入力文字列（タイトル表示・デバッグ用）。
    pub raw: String,
}

impl TableFilterPredicate {
    pub fn matches(&self, item: &TableItem, header: &[String]) -> bool {
        // include: 各列で「いずれか」マッチ（列内 OR）、すべての列を通過（列間 AND）。
        for (col, regexes) in &self.column_includes {
            let cell = cell_of(item, header, col).unwrap_or("");
            if !regexes.iter().any(|r| r.is_match(cell)) {
                return false;
            }
        }
        // exclude: いずれかにマッチしたら除外。
        for (col, regexes) in &self.column_excludes {
            let cell = cell_of(item, header, col).unwrap_or("");
            if regexes.iter().any(|r| r.is_match(cell)) {
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

/// 列名（小文字、正準形）に対応するセル値を返すヘルパ。
fn cell_of<'a>(item: &'a TableItem, header: &[String], col: &str) -> Option<&'a str> {
    let idx = header.iter().position(|h| h.eq_ignore_ascii_case(col))?;
    item.item.get(idx).map(|s| s.as_str())
}
```

### Table widget の拡張

Task 0 #980 で `filter_form: Option<FilterForm>` 化済み。さらに以下を追加:

```rust
pub struct Table<'a> {
    // 既存:
    filter_form: Option<FilterForm>,

    // 新規:
    filter_applicator: Option<TableFilterApplicator>,
    filter_state: Option<TableFilterPredicate>,   // 最後に成功した parse 結果
    filter_error: Option<String>,                 // 粘着エラー（Tab 階層の widget_error とは別管理）
    ...
}
```

**`filtered_key` と `filtered_word` は完全削除。** 全タブが SubstringFilterApplicator に乗り換える設計なので、移行期の if/else 分岐は持たない。一気に新パスへ統一する。

Tab 階層の `widget_error`（既存）は touch しない。Table widget の render は filter_error だけ気にする。

### Table widget 内部のフロー

#### キー入力時

```rust
// Mode::FilterInput で何かキー入力 → filter_form.on_key_event 後:
match applicator.strategy {
    ApplyStrategy::Live => {
        // 毎キーで parse → state 更新
        let input = filter_form.content();
        match (applicator.parser)(&input) {
            Ok(p) => { filter_state = Some(p); filter_error = None; }
            Err(e) => { filter_error = Some(e); }
        }
        // Live は副作用なし（on_apply は呼ばない）
    }
    ApplyStrategy::EnterToConfirm => {
        // タイプ中は何もしない。入力バッファに溜まるだけ。
    }
}
```

#### Enter 押下時

```rust
let input = filter_form.content();
let parsed = (applicator.parser)(&input);

match parsed {
    Ok(p) => {
        filter_state = Some(p);
        filter_error = None;
        // 副作用フック（例: NodeFilter なら label_selector を SharedNodeFilter へ書き込み）
        if let Some(cb) = &applicator.on_apply {
            // schedule cb(&p, &mut window) via Window callback queue
        }
        mode = FilterConfirm;
    }
    Err(e) => {
        filter_error = Some(e);
        // filter_state は変更しない（旧成功状態が残ったまま、ただし render はエラー優先で行を見せない）
        mode = FilterInput;  // 入力モード継続
    }
}
```

#### 入力中の `?` または `help`

```rust
if input == "?" || input == "help" {
    if let Some(id) = applicator.help_dialog_id {
        filter_form.clear();
        window.open_dialog(id);
    }
    // help_dialog_id が None なら通常の文字入力扱い
}
```

#### 行フィルタリング

```rust
fn item_passes_filter(&self, item: &TableItem) -> bool {
    if let Some(p) = &self.filter_state {
        p.matches(item, &self.header.original)
    } else {
        true
    }
}
```

`filtered_key` / `filtered_word` ベースの旧分岐は完全削除。すべての行絞り込みは `filter_state` を経由する。

#### Render

```rust
if let Some(e) = self.filter_error.as_ref() {
    // テーブル本体置換でエラー表示（行は描画しない）
    render_widget_error(f, chunk, block, e, theme);
} else {
    // 通常 render: filter_state があれば item_passes_filter で絞り込まれた行を描画
}
```

注: Tab 階層の `widget_error` は Tab.render が widget を呼び出す前にチェックする（既存）。つまり widget_error が立っているときはこの Table::render すら呼ばれない。よって優先順位は技術的に `widget_error > filter_error`。

## 既存タブの移行

各タブの Table 生成箇所で:

```rust
// Before:
Table::builder()
    .filter_form(filter_form)
    .filtered_key("NAME")
    .build()

// After:
Table::builder()
    .filter_form(filter_form)
    .filter_applicator(substring_applicator("NAME"))
    .build()
```

`substring_applicator` ファクトリ:

```rust
pub fn substring_applicator(column: &str) -> TableFilterApplicator {
    let col = column.to_string().to_lowercase();
    TableFilterApplicator {
        parser: (move |input: &str| {
            let patterns: Vec<Regex> = input
                .split_whitespace()
                .map(|p| Regex::new(&regex::escape(p)))
                .collect::<Result<_, _>>()
                .map_err(|e| e.to_string())?;

            let mut column_includes = HashMap::new();
            if !patterns.is_empty() {
                column_includes.insert(col.clone(), patterns);
            }

            Ok(TableFilterPredicate {
                column_includes,
                column_excludes: HashMap::new(),
                label_selector: None,
                raw: input.to_string(),
            })
        }).into(),
        strategy: ApplyStrategy::Live,
        help_dialog_id: None,
        on_apply: None,
    }
}
```

挙動は既存と等価:
- live で毎キー絞り込み（ApplyStrategy::Live）
- 単一列（NAME）に対し、スペース区切り **OR** で部分一致
  - 既存実装 `split(' ').any(|p| contains(p))` と同じ意味論（OR）。
  - 新統一意味論（列内 OR、列間 AND）と整合: 列が NAME 1 つだけなので「列内 OR」がそのまま既存挙動になる。
- 空入力で「フィルタなし」（column_includes が空 → 全行 pass）
- patterns は `regex::escape` でリテラル化（既存 substring と等価動作。ユーザーは regex を意識しなくていい）

対象タブ: Pod / Config / Network / Event / API / Yaml / Context / Namespace の各 Table 生成箇所。

## Node タブのフィルタ実装

### 構文

```
foo                              → NAME に foo を含む（regex）
foo bar                          → NAME に foo OR bar（列内 OR）
NAME:gke.*worker                 → NAME に regex
STATUS:Ready                     → STATUS に Ready
STATUS:Ready STATUS:Pending      → STATUS が Ready OR Pending（列内 OR、in 句相当）
STATUS:Ready NAME:nginx          → STATUS が Ready AND NAME が nginx（列間 AND）
foo bar STATUS:a STATUS:b        → (NAME に foo OR bar) AND (STATUS に a OR b)
!NS:kube-system                  → NAMESPACE に kube-system を含む行を除外
!STATUS:Pending !STATUS:Failed   → STATUS が Pending OR Failed を含む行を除外（not in 相当）
label:role=worker                → k8s labelSelector (server-side)
label:role=worker,zone=us-west   → カンマ AND の k8s labelSelector（k8s 構文そのまま）
```

**意味論:**

| 区分 | 結合 | 備考 |
|---|---|---|
| 同一列の include 複数 | **OR** | `in (a, b)` 相当 |
| 異なる列の include 混在 | **AND** | 「列ごとに条件を重ねる」 |
| 同一列の exclude 複数 | **いずれかにマッチすれば除外** | `not in (a, b)` 相当 |
| 異なる列の exclude 混在 | 同上（いずれかマッチすれば除外） | 全 exclude を pass する必要 |
| ベア値 | NAME 列 include の暗黙形 | `foo bar` = `NAME:foo NAME:bar` = NAME に foo OR bar |
| `label:` | 最後勝ち | k8s API は labelSelector 1 つしか取らない |

- 列名は大小区別しない、正準形は小文字（CLI / Pod log query と同型）
- 単一列内の AND が欲しい場合（稀）は regex lookahead で `NAME:(?=.*foo)(?=.*bar)`
- 値は常に regex として解釈（plain text は実質的に部分一致として動作）

### NodeFilterApplicator

Node 用の TableFilterApplicator ファクトリ。前述の `TableFilterPredicate` をそのまま返す（Node 専用の predicate type は持たない）。

```rust
pub fn node_filter_applicator(
    label_registry: Vec<NodeLabelColumn>,
    tx: Sender<Message>,
) -> TableFilterApplicator {
    TableFilterApplicator {
        parser: build_node_filter_parser(label_registry),
        strategy: ApplyStrategy::EnterToConfirm,
        help_dialog_id: Some(NODE_FILTER_HELP_DIALOG_ID),
        on_apply: Some(build_on_apply(tx)),
    }
}

fn build_on_apply(tx: Sender<Message>) -> OnFilterApply {
    (move |predicate: &TableFilterPredicate, _w: &mut Window| {
        // labelSelector を SharedNodeFilter 経由で poller に
        // （labelSelector が None でも Apply で「クリア」として送る）
        tx.send(NodeFilterMessage::Apply(predicate.label_selector.clone()).into())
            .expect("Failed to send NodeFilterMessage::Apply");
    }).into()
}
```

`NodeFilterMessage` のペイロードは `Option<String>`（label_selector のみ）に縮められる。クライアント側 filter（regex マッチ）は `TableFilterPredicate.matches` が担うので、controller / poller は labelSelector だけ知っていれば十分。

### サーバ側フィルタリング

`NodeFilterMessage::Apply(Option<NodeFilterPredicate>)` を controller が受けて `SharedNodeFilter` に書き込む。`NodePoller` が次ポーリングで `predicate.label_selector` を URL `?labelSelector=...` に反映してリクエスト。

クライアント側 regex は Table widget の `filter_state.matches()` 経由で全行に適用される。

## エラーハンドリング

エラー状態は 2 つあり、**管理階層も優先順位も異なる**ことに注意:

| 状態 | 管理階層 | セット | クリア | レンダリング箇所 |
|---|---|---|---|---|
| `widget_error` | **Tab**（`Tab.error_states: HashMap<widget_id, Vec<String>>`、既存）| action.rs の `update_widget_item_for_table` が Err 受信時 | 同関数が Ok 受信時に自動 | `Tab.render()` が widget の代わりに `render_widget_error` を呼ぶ（widget 自身は呼ばれない）|
| `filter_error` | **Table widget**（新規フィールド）| Table 内部で `parser(input)` が Err のとき | `parser` 成功時 / フィルタクリア時 / filter_form が空に戻ったとき | Table の render 内で行の代わりに描画 |

### 優先順位: `widget_error > filter_error`

これは「優先したい」というより**技術的にそうなる**。Tab.render が widget_error を見て表示するときは、Table widget の render 自体が呼ばれない（Tab が代替描画する）→ filter_error はチェックされない。

UX 的にも妥当:
- `widget_error` = API そのものが失敗（基礎データなし）→ 最優先メッセージ
- `filter_error` = データはあるがフィルタが壊れている → API 復旧次第顔を出す

### parse エラー（クライアント側）

- `filter_form` の Enter（または live モードの毎キー）で `parser(input)` が `Err(msg)` を返す
- Table の `filter_error = Some(msg)` をセット
- render 時、`filter_error` が Some なら**既存 `render_widget_error` と同じ関数**でテーブル本体を置換表示（行は描画されない）
- ポーリングは触らない（粘着）

### サーバ側エラー（labelSelector が無効など）

- 既存通り `NodeMessage::Poll(Err(e))` → action.rs の `update_widget_item_for_table` で `set_widget_error(NODE_WIDGET_ID, &e)`
- Tab レベルの widget_error 経路で表示、ポーリングが失敗し続ければ表示維持、成功すれば自動的に消える（既存挙動）
- このとき Table widget の render は呼ばれないので、`filter_error` が立っていても見えない（widget_error が優先）

### 両方立つケース

例: フィルタ parse エラー中にサーバ側もエラーになった → Tab レベルの widget_error が表示される。サーバ復旧したら widget_error が clear され、filter_error が見える。自然な振る舞い。

## ヘルプ

`Applicator::help_dialog_id` が Some のとき、入力中に `?` または `help` を打つと filter_form をクリアして該当ダイアログを開く。Pod log query の慣習を踏襲。

Substring applicator は `help_dialog_id = None`（既存タブはヘルプなし、現状維持）。
Node applicator は `help_dialog_id = Some(NODE_FILTER_HELP_DIALOG_ID)`。

## データフロー

```
[ユーザー入力 `/`]
        ↓
[filter_form 入力モード]
        ↓
タイプ:
  - Live applicator → 毎キーで parser → state 更新（成功時）or error（失敗時）
  - EnterToConfirm applicator → 何もしない
        ↓
[Enter]
        ↓
parser(input)
  ├── Ok(predicate) ──→ Table.filter_state = Some(predicate)
  │                     Table.filter_error = None
  │                     on_apply(&predicate, &mut Window)
  │                       └─ Node の場合: NodeFilterMessage::Apply で SharedNodeFilter 更新
  │                          → 次ポーリングで URL に labelSelector 反映
  │                     mode = FilterConfirm
  │                     render: filter_state で行をフィルタして表示
  │
  └── Err(msg) ─────────→ Table.filter_error = Some(msg)
                           mode = FilterInput 継続
                           render: テーブル本体置換でエラー表示（行は描画されない）
```

## モジュール構成

```
src/ui/widget/table/
├── filter.rs                  # FilterForm (既存)
├── filter_applicator.rs       # 新規: TableFilterApplicator, ApplyStrategy, TableFilterPredicate
└── item.rs                    # 既存

src/features/node/
├── filter.rs                  # 既存（Plan 5）: NodeFilterPredicate を再定義（列ベースに拡張）
├── filter/parser.rs           # 既存（Plan 5）: nom パーサ、構文を拡張
└── view/widgets/
    ├── node.rs                # 修正: filter_form 復活、node_filter_applicator を設定
    ├── node_filter.rs         # 削除: ダイアログとしての node_filter_widget
    ├── node_filter_help.rs    # 保持: ヘルプダイアログ
    └── ...
```

## 既存ファイルの編集点

### Table widget 拡張（新規共有ウィジェット改修）
- `src/ui/widget/table.rs` — TableBuilder に `filter_applicator` フィールド + setter、Table struct に 3 フィールド追加（applicator, filter_state, filter_error）。on_key_event の Enter / 通常キーで applicator を呼ぶ。render で filter_error 優先。
- `src/ui/widget/table/filter_applicator.rs` — 新規。型定義一式。
- `src/ui/widget/table/item.rs` — item_passes_filter を新パスに切り替え（filter_state 経由）。filtered_key / filtered_word の参照を削除。
- `src/ui/widget.rs` — `pub use table::filter_applicator::...` re-export

### 既存タブ移行（SubstringFilterApplicator へ）
- `src/features/pod/view/widgets/pod.rs`
- `src/features/config/view/widgets/config.rs`
- `src/features/network/view/widgets/network.rs`
- `src/features/event/view/widgets/event.rs`（あれば）
- `src/features/api_resources/view/dialog.rs`
- `src/features/yaml/view/dialogs/{name,kind}.rs`
- `src/features/context/view/dialog.rs`
- `src/features/namespace/view/{single,multiple}_namespaces_dialog.rs`

各箇所で `.filtered_key("NAME")` を `.filter_applicator(substring_applicator("NAME"))` に置換。

### Node タブ実装
- `src/features/node/filter.rs` — NodeFilterPredicate を列ベースに再定義
- `src/features/node/filter/parser.rs` — `<col>:<val>` 構文に拡張、`TableFilterPredicate` を返す（ベア値は NAME 列 OR、列明示は AND across columns）
- `src/features/node/view/widgets/node.rs` — filter_form 復活、node_filter_applicator() 設定
- `src/features/node/view/widgets/node_filter.rs` — 削除
- `src/features/node/view/widgets/node_filter_help.rs` — 保持、起動経路は applicator の help_dialog_id 経由
- `src/features/component_id.rs` — `NODE_FILTER_WIDGET_ID` 削除（ダイアログとしてのウィジェットがなくなる）

### 既存タブの構造的変更（filter_state ベースへの完全移行）
移行が完了したら、Table widget から `filtered_key` / `filtered_word` 関連コードを削除。`InnerItem::update_filter` も削除し、絞り込みは外部から `filter_state` 経由で行うようにする。

## テスト

- `TableFilterPredicate::matches`: 列内 OR・列間 AND・exclude (any-match-excludes) の各組み合わせを単体テスト
- `substring_applicator`: parser → Predicate のラウンドトリップ、空入力で Empty
- `node_filter_applicator` parser: `<col>:<val>` / `!<col>:<val>` / `label:` の各パターン、エラー、列名大小区別なし
- Table widget の filter_state ベース行絞り込み（既存の Pod 行テストが等価結果になるか）
- filter_error がポーリング更新で消えないことの単体テスト
- 既存タブの substring filter ライブ挙動の単体テスト（substring_applicator 経由で）

## 段階的実装計画（plan で詳細化）

実装規模が大きいので 2 つの PR に分割:

### PR A: Table widget の filter_applicator 化＋全既存タブの SubstringFilterApplicator 移行
- 共有ウィジェット改修（`fmt` ベース起点）
- 既存タブの UX を保ったまま内部実装を統一
- マージされると、既存タブの filtered_key 構成は API 上消える

### PR B: Node タブの NodeFilterApplicator 実装（Plan 5 ブランチを再構築）
- PR A の上にスタック
- Plan 5 既存実装の流用パーツ（NodeFilterPredicate, parser, SharedNodeFilter, controller の Apply ハンドラ, poller の URL labelSelector）はそのまま
- 独立ダイアログ `node_filter_widget` は削除
- node.rs で `filter_applicator(node_filter_applicator(...))` を設定

## 主要な設計判断の記録

- **既存タブも parser ベースに統一する**: Table widget 内部に `if let Some(predicate)` と `if let Some(filtered_word)` の 2 系統を残すと負債になるので、一気に統一する。既存タブは SubstringFilterApplicator（live + 単一列 substring、OR 維持）で挙動を完全に保つ。
- **適用タイミングは applicator ごとに宣言**: Live と EnterToConfirm を実装に依存させず、parser とセットで applicator が自分の適切な戦略を持つ。
- **意味論は「列内 OR、列間 AND、exclude は any-match excludes」**: SQL / kubectl labelSelector / spreadsheet など主要な「列ベースフィルタ」慣習と一致。既存 Pod の `/foo bar` (OR) も同じ枠組みに自然に収まる（NAME 単一列の OR 形）。単一列 AND が必要なレアケースは regex lookahead `NAME:(?=.*foo)(?=.*bar)` で対応。
- **エラーは管理階層が違う**: `widget_error` は Tab レベル（既存・既存タブの API エラー UI を変えない）、`filter_error` は Table widget レベル（新規・粘着）。優先順位は技術的に `widget_error > filter_error`（widget_error 表示時は Table.render が呼ばれない）。UX 的にも妥当: API 失敗は最優先メッセージ、parse エラーは API 復旧次第顔を出す。
- **TableFilterPredicate は単一 struct**: 新意味論では Substring と Node が同じ「列内 OR、列間 AND」で表現できるため、enum 分岐は不要。将来別種マッチが必要になった時点で enum 化すれば良い（YAGNI）。
- **API 形は applicator 構造体で束ねる**: builder のメソッドを増やすより、parser/strategy/help/on_apply を 1 ショットの applicator として渡す方が、タブ側コードがシンプルで設定漏れも起きない。
- **dyn は既存パターン (`define_callback!` = `Rc<dyn Fn>`) を踏襲**: コールバック層では既に dyn を多用しているので一貫性。ホットパス（matches）は単一 struct のメソッドなので static dispatch。
