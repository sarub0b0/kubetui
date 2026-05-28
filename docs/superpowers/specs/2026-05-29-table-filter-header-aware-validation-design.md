# Table フィルタ: header ベースの列検証と列名正規化

- 日付: 2026-05-29
- ステータス: Proposed（提案中）
- 対象範囲: 共有 Table フィルタ framework（`src/ui/widget/table/`）＋ 検証台としての Node タブフィルタ（`src/features/node/filter/`）
- 対象外: Pod タブの column-aware フィルタへの移行（別途 Phase B spec として管理）

## 背景

共有 Table フィルタ framework（`TableFilterApplicator`、PR #980/#982 で導入、設計は
`docs/superpowers/specs/2026-05-27-table-filter-redesign.md`）は、各タブが生のフィルタ文字列を
`TableFilterPredicate` にパースし、テーブル行に対してマッチさせる仕組みである。Node タブは
column-aware なパーサ（`COL:val` / `!COL:val` / `label:sel` / bare→NAME）を使い、Pod / Config /
Network は今もよりシンプルな `substring_applicator` を使っている。

Pod 移行を計画する過程で、共有 framework に既存の欠陥が2つ見つかった。これらは Pod 固有では
なく、Node タブから今すでに踏める問題である。

### 課題 I — 空白を含む列名がフィルタ不能

- パーサは入力を空白で分割する（`separated_list0(multispace1, parse_token)`、
  `src/features/node/filter/parser.rs:264-269`）ため、列トークンに空白を含められない。
- マッチングは列名を**素の** lowercase 文字列の完全一致で引く。`cell_of` は
  `h.to_lowercase() == col_name_lower` で比較する
  （`src/ui/widget/table/filter_applicator.rs:80-89`）。
- Node の builtin 列はハイフン区切り（`INTERNAL-IP`、`OS-IMAGE` …）なので問題なくトークン化
  できるが、Node の **label** 列の header は `def.name.to_uppercase()` で空白除去をしない
  （`src/app.rs:272`）。name に空白を含む label 列（例: `"my label"` → header `"MY LABEL"`）は
  有効な列として登録される（`valid_columns` が `lc.header.to_lowercase()` を入れる、
  `parser.rs:217-225`）が、ユーザーは空白を打てず、かつ `cell_of` は空白込みの完全一致比較を
  するため、決してマッチしない。

要するに、コピーできる既存解は存在しない。Pod 移行ではこれが必須になる。Pod の default builtin
列に `NOMINATED NODE` と `READINESS GATES`（既定で空白入り）が含まれるためである。

### 課題 II — 有効だが非表示の列を指定すると全行が消える

- `valid_columns` は全 `NodeColumn::iter()` enum ＋ label registry から構築され
  （`parser.rs:217-225`）、現在どの列が表示されているかに依存しない。
- Node の列は列ダイアログで実行時に変更できる
  （`NodeMessage::Request` → `shared_node_columns`、`src/workers/kube/controller.rs:616-621`）。
  default 表示は builtin 10 列のうち 5 列（`DEFAULT_NODE_COLUMNS`）。
- マッチングは `cell_of(...).unwrap_or_default()` を使う。live header に存在しない列は `""` を
  返すため、`include` パターンが全行で失敗する（`filter_applicator.rs:53-71`）。結果、実在する
  が非表示の列でフィルタすると（例: default の Node 列で `internalip:10.`）、パースは成功した
  うえでテーブル全体が空に見える ── フィードバックも無くバグに見える。

## ゴール

1. 列名マッチングを空白 / `-` / `_` に寛容にし、複数語の列をフィルタ可能にする。
2. 非表示列に対する「全行が消える」挙動を、明示的で親切な parse error に置き換える。
3. 両修正を framework レベルで統一し、全タブ（今は Node、後で Pod / Config / Network）が恩恵を
   受けられるようにする（タブごとの場当たり的対処を不要にする）。
4. 既存の `substring_applicator` 系タブにとって挙動上安全な変更に留める。

## 非ゴール

- Pod（および Config / Network）の column-aware パーサへの移行 ── Phase B。
- Pod 向けのサーバーサイド `labelSelector` 配線 ── Phase B。
- フィルタで参照された非表示列を自動的に表示へ追加すること。

## メンタルモデル

ここで表現するルール: **見えている列だけをフィルタできる**。これは TUI ユーザー（k9s, fzf,
less）が期待するフィルタの挙動 ── 見えている表現を絞り込む ── に一致する。現在の表示に無い列を
参照した場合は、（列ダイアログで追加できるよう）その列が表示されていない旨を伝えるエラーとして
報告する。サイレントに何も返さない／サイレントに項を無視する、のいずれでもない。

## 設計

### 1. パーサが live header を受け取る

`TableFilterParser` コールバックのシグネチャを

```
Fn(&str) -> Result<TableFilterPredicate, String>
```

から

```
Fn(&str, &[String]) -> Result<TableFilterPredicate, String>
```

へ変更する。第2引数はテーブルの現在の表示 header（`self.items.header().original()`）である。
Table ウィジェットはこれを既に保持しマッチングに使っている。`run_parser_and_update_state`
（`src/ui/widget/table.rs:831-839`）は `&mut self` メソッドで header にアクセスできるため、
`&header` をパーサクロージャに渡す。

header は「何が表示されているか」の唯一の真実の源であり、すでにマッチングが比較対象とするもので
あり、かつ render スレッド上で同期的に取得できる ── これによりタブごとの列設定を包む async
`RwLock` を回避できる。

### 2. 共有の列名正規化

フィルタ framework に正規化関数を1つ導入する:

```
normalize_column_name(s) = s を小文字化し、' ' / '-' / '_' をすべて除去
```

（`PodColumn::normalize_column` がまさにこれを実装済みで参照にできる。共有ヘルパーは全タブが同じ
ルールを使えるようフィルタモジュールに置く。）

これを列名比較の**両側**に適用する:

- `cell_of` 内: 素の `to_lowercase()` の代わりに、各 header エントリと引くキーを正規化してから
  比較する。ハイフン入りの名前は引き続き動作し（`internal-ip` → `internalip`）、
  ハイフン / アンダースコア / 空白は無視されるようになる。
- パーサ内: ユーザーの列トークンと各 header エントリを正規化して検証し、predicate のキーには
  **正規化済み**の列名を格納する。`cell_of` も正規化するため、格納したキーは正しい header 列へ
  解決される。

これにより `nominatednode` / `nominated-node` / `Nominated_Node` がすべて `NOMINATED NODE`
header にマッチし、空白入りの Node label 列もフィルタ可能になる。

### 3. live header に対して列参照を検証する（課題 II）

パーサ内で、有効な列集合は**正規化された header エントリの集合**とする（builtin enum でも、
label registry のスナップショットでもない）。各 `COL:val` / `!COL:val` 項について:

- `normalize_column_name(COL)` が正規化済み header エントリに含まれない場合 → parse error を返す:
  `column '<COL>' is not in the current view`（既存の `filter_error` チャネルがこれをテーブル本体
  の代わりに描画する ── 修正されるまで sticky）。
- `label:sel` は常に受理する（これは表示列ではなくサーバーサイド selector であるため）。
- bare 値は NAME 列の include に対応する。NAME は常に存在する（`ensure_name_column`）ので、
  実際にはこれがエラーになることはない。

帰結: Node パーサは検証のために `NodeColumn::iter()` や label registry スナップショットを必要と
しなくなる。`node_filter_applicator` の `label_registry` 引数は検証に使われなくなり削除する。
パーサは header に対してのみ検証するよう簡素化される。

エッジケース ── 空の header（最初のポーリングがテーブルを埋める前）: header が空の場合は列検証を
スキップ（項を受理）し、データ到着前にユーザーが誤って「not in the current view」エラーを受け
ないようにする。（Node では header は設定済みの列 spec から構築され、テーブルが一度構築されれば
すぐ埋まるため、これが影響するのは最初の描画前の短い区間のみ。）

### 4. エラー文言

「unknown column（未知の列）」ではなく「not in the current view（現在の表示に無い）」を使う。
フィルタの視点で知り得る列は表示中のものだけであり、実在するが非表示の k8s 列と単なるタイプミスは
区別できず、両者とも正しく「その列は表示されていない」に解決されるためである。

### 5. ヘルプダイアログ

Node フィルタのヘルプ（`src/features/node/view/widgets/node_filter_help.rs`）を更新し、「有効な
列」の案内を固定列挙ではなく「現在テーブルに表示されている列」と読めるようにする。現在の header を
ダイアログに動的に列挙するのは任意の磨き込みであり、後回しにしてよい。

### 6. `substring_applicator` と他タブ

`substring_applicator(column)`（`src/ui/widget/table/filter_applicator.rs`）は
`TableFilterParser` を生成する。そのクロージャのシグネチャを更新して header 引数を受け取りつつ
無視する（固定の単一列 ── NAME ── をフィルタし、NAME は常に存在するため挙動は不変）。共有
`cell_of` の正規化は単一語の `NAME` に対しては no-op なので、Pod / Config / Network のマッチング
は各自の Phase B 移行まで影響を受けない。

## 影響を受けるファイル

- `src/ui/widget/table/filter_applicator.rs` ── `TableFilterParser` シグネチャ、`cell_of` の
  正規化、新規 `normalize_column_name` ヘルパー、`substring_applicator` のクロージャシグネチャ。
- `src/ui/widget/table.rs` ── `run_parser_and_update_state` でパーサに `&header` を渡す、
  モジュール内テスト用パーサ（`table.rs:~1120`）の更新。
- `src/features/node/filter.rs` ── `node_filter_applicator` から `label_registry` を削除、
  パーサ配線の更新、registry を渡している呼び出し元の更新。
- `src/features/node/filter/parser.rs` ── `parse_node_filter` が label registry ではなく header
  を受け取る、正規化済み header に対する検証、正規化済みキーの格納、テスト更新。
- `src/features/node/view/widgets/node_filter_help.rs` ── ヘルプ文言。
- `node_filter_applicator` の呼び出し元（例: `src/features/node/view/widgets/node.rs`、
  `tab.rs`、render 配線）── 不要になった registry 引数の削除。

## テスト

- `normalize_column_name` の単体テスト（空白 / `-` / `_` / 大小文字）。
- `cell_of` のテスト: 複数語 header が正規化キーにマッチする、ハイフン入り header も引き続き
  マッチする。
- `parse_node_filter` のテスト（header 駆動）: 有効な表示列、正規化トークン経由の複数語列、
  `!COL` exclude、`label:`、bare→NAME、**非表示列 → parse error**、空 header → 検証エラー無し。
- `substring_applicator` の挙動が不変であることの確認（既存の Pod/Config/Network フィルタ
  テストが通り続ける）。
- `cargo test --all`、`cargo clippy`、`cargo +nightly fmt --check`。

## リスク / 後方非互換

- Node のマッチング意味論がわずかに変わる: 列名中の `-` / `_` / 空白が無意味になる
  （例: `internalip` が `INTERNAL-IP` にマッチするようになる）。これは現在のマッチの厳密な
  上位集合であり、既存の `internal-ip` クエリは引き続き動作する。Node パーサのテストは正規化
  比較を反映するよう更新が必要。
- `node_filter_applicator` から `label_registry` を削除すると呼び出し箇所に波及する（機械的）。

## フォローアップ（Phase B ── 別 spec）

Pod タブを `substring_applicator("NAME")` から column-aware な `pod_filter_applicator` へ移行し、
この修正済み framework に乗せる:
- `parse_pod_filter` は live header（表示時は `NAMESPACE` も含む）に対して検証するので、Pod の
  複数語 builtin 列が Pod-local な正規化ハックなしで動作する。
- `label:` サーバーサイド selector: `PodMessage::Filter(Option<String>)` ＋ `SharedPodFilter` を
  追加し、namespace ごとの pod 取得（`get_pods_per_namespace`）に `?labelSelector=` を配線、
  controller でメッセージを処理。Node に倣い `EnterToConfirm` strategy ＋ ヘルプダイアログ。
- Pod のログクエリパーサ（`src/features/pod/kube/filter.rs`）は無関係（ログ行のフィルタ・別ペイン）
  であり、手を加えない。
