# Table フィルタ: 非表示列の項を inactive 化（サスペンド）

- 日付: 2026-05-29
- ステータス: Proposed（提案中）
- 対象範囲: 共有 Table フィルタ framework（`src/ui/widget/table/`）＋ Node タブフィルタ（`src/features/node/filter/`）
- 前提: PR #988（Phase A, header ベース列検証）がマージ済み。本 spec はその課題 II の解法を **「parse error」から「inactive（サスペンド）」へ作り替える**。
- 対象外: Pod の column-aware 移行（Phase B、別 spec）

## 背景

PR #988（Phase A）で、非表示列を指定したフィルタが「全行消失」する課題 II を **parse 時に `column 'x' is not in the current view` エラー**で解消した。しかし運用で2つの不満が残った。

1. **列ダイアログで「フィルタ中の列」を非表示にすると全行が消える**。検証は parse 時のみで、`update_header_and_rows`（`src/ui/widget/table.rs`）は既存 `filter_state` を新 header に再適用するだけ。非表示になった列の `cell_of` が `None`→`unwrap_or_default()`→`""` となり、include が全行を弾く。
2. **エラーはテーブル本体を置換する**ため、一時的に列を隠して他列を見たい場合でも、隠している間は行が一切見えない（`filter_error` は Normal モードでも本体置換、`src/ui/widget/table.rs` の render）。

kubetui は小ペイン前提（情報密度優先）で「スペースの都合で列を隠す」は主要ワークフロー。隠している間も**残りの可視列でフィルタした行は見えてほしい**。

## メンタルモデル

フィルタ項が参照する列を 3 状態で扱う:

| 列の状態 | 例 | 挙動 |
|---|---|---|
| 表示中 | `NAME`, `STATUS` | フィルタ有効 |
| 定義済みだが非表示 | 列ダイアログで `VERSION` を外す | **inactive**（項は保持し matching でスキップ。列を再表示すれば自動復活。バッジ表示） |
| 未定義（builtin でも label でもない） | `stauts:`（タイポ）, `foo:` | **parse error** |

要するに「**見えている列だけが効く。実在するが隠れている列の項は一時停止して取っておく。実在しない列はエラー**」。

## 設計

### 1. matching: header に無い列の制約をスキップ（inactive の本体）

`TableFilterPredicate::matches`（`src/ui/widget/table/filter_applicator.rs`）を変更し、`column_includes` / `column_excludes` の各列について、**その列名が現在の header に存在しなければその制約をスキップ**（include は行を弾かない、exclude は除外しない）する。スキップ条件は「**列が header に無い**」こと（正規化比較での header 探索が見つからない）であって、「セルが欠落している」ことではない点に注意（列は header にあるが行のセルが欠落している異常時は従来どおり空文字として扱う）。実装上は header からの列インデックス解決を制約評価の前に行い、見つからなければ `continue` する。

- これだけで両不満が直る:
  - 列削除時: `update_header_and_rows` が `filter_state` を新 header に再適用する際、消えた列の項が自然にスキップされ、残りの可視列で絞った行が見える。**`update_header_and_rows` 自体は変更不要**。
  - 列再表示時: 同じ再適用で項が再びマッチに参加＝自動復活。
- `normalize_column_name`（Phase A Task 1）と cell_of の正規化比較は**そのまま使う**。「列が header にあるか」も正規化して判定する。

### 2. parser: 既知列で検証（未定義はエラー、定義済みは header 非依存で受理）

Node parser（`src/features/node/filter/parser.rs`）の列検証を、**header ではなく「既知列の集合」**に対して行うよう戻す（Phase A Task 3 で header 検証に切り替えたのを既知列検証に作り替え）。

- 既知列 = builtin（`NodeColumn::iter()` の `display()` を正規化）＋ 定義済み label 列（`label_registry` の各 `header` を正規化）。＝ Phase A 以前の `valid_columns(label_registry)` と同等。
- `COL:val` / `!COL:val` の列が既知列に**無ければ** `unknown column '<COL>'` を返す（元のトークン表記で）。**header（表示中か否か）は parse 時に見ない** — 非表示は inactive として matching 側で表現するため。
- `label:`（サーバーサイド selector）と bare→NAME（NAME は builtin 既知）は従来どおり。regex 不正・クォート不整合等のエラーも従来どおり。
- 結果、**parser は header を必要としない**ので、Phase A Task 2 で導入した `TableFilterParser` への header 配線を撤去し `Fn(&str) -> Result<TableFilterPredicate, String>` に戻す。`label_registry` を `node_filter_applicator` / `parse_node_filter` に戻し、parser は構築時に捕捉した既知列集合で検証する。

### 3. inactive バッジ

filter_state の `column_includes` / `column_excludes` のキー（正規化済み）のうち、**現在の正規化 header に無いもの**を集め、空でなければ count インジケータの後ろに `(inactive: <列名>)` を表示する。

- 列名は**正規化形**（filter_state のキーそのまま。例 `internalip`）、**ソートして決定的順序**（`HashMap` 順のちらつき防止）、`,` 区切り、**上限なし**（長ければタイトルが自然に切り詰め）。1ワード `inactive`。
- 計算・表示は共有 Table widget 側（`count_indicator()` に統合し、count を出す全タブで自動表示）。filter_state に既知列以外は入らない（未定義はエラーで弾かれる）ので、「filter_state にあって header に無い列」＝「定義済みだが非表示」＝ inactive 対象で過不足ない。substring タブ（NAME フィルタ）は NAME 常在のため発火しない。

例:
```
 Node  [1/3 (4)]  (inactive: version)
```

### 4. ヘルプ文言

`src/features/node/view/widgets/node_filter_help.rs` の説明を 3 状態に合わせて更新（Phase A の "not in the current view" 文言を置換）。趣旨: 「列は builtin か定義済み label 列であること（未定義はエラー）。現在表示されていない列の項は、その列を表示するまで inactive（一時停止）になる」。

### 5. Phase A との関係（差分の明示）

- **残す**: Task 1（`normalize_column_name` ＋ `cell_of` 正規化、課題 I）。
- **作り替え/撤去**: Task 2（`TableFilterParser` への header 配線 → `Fn(&str)` に戻す）、Task 3（header 検証 → 既知列検証に戻し、未定義はエラー。`label_registry` を parser に復帰）。
- **新規**: matching の inactive スキップ、inactive バッジ。
- 課題 II の最終的な解は「非表示列＝parse error」から「**非表示の既知列＝inactive（行は見える）／未定義列＝error**」へ。

### エッジケース

- **初回ポーリング前（header 空）**: matching は全制約をスキップ（行は 0 件なので無影響）。parser は既知列集合（header 非依存）で検証するため正常動作。Phase A で必要だった「空 header で検証スキップ」特例は不要になる。
- **exclude on 非表示列**: include と同様にスキップ（除外しない）。inactive バッジには include/exclude 双方の対象列を載せる。
- **同名列の include/exclude 重複**: バッジは列名で重複排除して 1 回だけ表示。

## 影響を受けるファイル

- `src/ui/widget/table/filter_applicator.rs` — `matches` を「header に無い列はスキップ」に変更。`TableFilterParser` を `Fn(&str) -> Result<…>` に戻す。`substring_applicator` のクロージャを `Fn(&str)` に戻す。`normalize_column_name` / `cell_of` 正規化は維持。
- `src/ui/widget/table.rs` — `run_parser_and_update_state` のパーサ呼び出しを header 無しに戻す。`count_indicator()`（または同等の共有ヘルパー）に inactive バッジ計算・連結を追加。テスト用パーサのシグネチャを戻す。
- `src/features/node/filter.rs` — `node_filter_applicator(label_registry, tx)` に戻し、クロージャは `move |input| parse_node_filter(input, &label_registry)`。`NodeLabelColumn` import 復帰。
- `src/features/node/filter/parser.rs` — `parse_node_filter(input, label_registry)` に戻す。`valid_columns(label_registry)` = builtin（`NodeColumn::iter()`）＋ registry の正規化集合。未定義列は `unknown column '<x>'`。`NodeColumn` / `NodeLabelColumn` / `strum::IntoEnumIterator` import 復帰。テスト更新。
- `src/features/node/view/widgets/node.rs` — `node_widget(tx, label_registry, theme)` に戻し、`node_filter_applicator(label_registry, tx.clone())`。`NodeLabelColumn` import 復帰。
- `src/features/node/view/tab.rs` — `node_widget(tx.clone(), label_registry.clone(), theme.clone())` に戻す（`label_registry` は引き続き dialog でも使用）。
- `src/features/node/view/widgets/node_filter_help.rs` — 文言更新。

## テスト

- **matching**: 非 header 列の include をスキップして行が残る／非 header 列の exclude をスキップ（除外しない）／header にある列は従来どおり機能。
- **parser**: 既知列（builtin・登録 label）は受理。未定義列は `unknown column` エラー。非表示の既知列も parse は通る（matching 側で inactive）。regex/クォートのエラーは維持。`label:`・bare→NAME は従来どおり。
- **バッジ**: filter_state に header 外の既知列があるとき `(inactive: …)` をソート・正規化・重複排除して整形。inactive 無しのとき非表示。substring タブで非発火。
- **結合**: フィルタ適用→列ダイアログで該当列を非表示→行が見える＋バッジ→再表示で復活。タイポ→parse error。
- `cargo test --all` / `cargo clippy` / `cargo +nightly fmt --check`。

## リスク / 後方非互換

- PR #988（Phase A Task 2/3）の header 配線・header 検証を作り替えるため churn がある。`normalize_column_name`（Task 1）は維持。
- 挙動変更: 非表示の既知列を指定したフィルタは、エラー（Phase A）→ **inactive（行は見える＋バッジ）** に変わる。未定義列は引き続きエラー（文言は "not in the current view" → "unknown column" に戻る）。
- inactive バッジの列名は正規化形（例 `internalip`）。表示用の正準名（`INTERNAL-IP`）にするには widget へ既知列の display 情報を渡す配線が要るため、本 spec では正規化形に留める（将来の磨き込み余地）。

## Pod（Phase B）への波及

matching の inactive スキップとバッジは共有 widget なので、Pod が column-aware に移行すれば自動的に効く。Pod の parser は既知列＝`PodColumn::iter()`（label 列なし）で検証すればよい。
