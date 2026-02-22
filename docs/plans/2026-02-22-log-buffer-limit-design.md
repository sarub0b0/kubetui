# Log Buffer Limit Design

## Background

ログビューにログが追加され続けると、メモリ使用量が無制限に増加する問題がある。
報告では70万行のログで1.8GBほどのメモリを消費していた。

現在の実装では `TextItem` が全ログ行を `Vec<Line>` と `Vec<WrappedLine>` に無制限に蓄積しており、
各行は grapheme 単位で `StyledGrapheme` に展開されるため、元データの2〜3倍のメモリを消費する。

## Design

### Approach

TextItem レベルでローリングバッファを実装し、行数上限を超過した古いログを破棄する。

- デフォルトは制限なし（後方互換性維持）
- ユーザーが設定ファイルまたはログクエリで明示的に上限を設定した場合のみ有効

他ツールの調査結果:
- k9s: リングバッファ、最大5,000行、古いログは破棄（設定ファイルで変更可能）
- tui-logger: 循環バッファ、デフォルト10,000行
- lazydocker: 上限なし（性能劣化の既知問題あり）

### Data Structure Changes

#### TextItem

- `max_lines: Option<usize>` フィールドを追加
  - `None` = 制限なし（デフォルト）
  - `Some(n)` = 最大 n 行
- `Vec<Line>` → `VecDeque<Line>` に変更（先頭削除を O(1) にする）
- `Vec<WrappedLine>` → `VecDeque<WrappedLine>` に変更

#### push() メソッドの変更

行追加後に `max_lines` を超過していたら、先頭1行を削除する:

1. `lines` から先頭の行を `pop_front()` で削除
2. 対応する `wrapped_lines` を先頭から削除（削除する Line の wrapped_lines レンジ分）
3. スクロール位置（`scroll.y`）を削除された wrapped_lines 数分だけ減算（0未満にならないようクランプ）
4. 検索ハイライト（`highlights`）の `line_index` を再計算、削除された行のハイライトを除去

パフォーマンス最適化（バッチ削除等）は必要になってから行う。

### Configuration

#### Config File (YAML)

```yaml
logging:
  max_lines: 10000  # 省略時は制限なし
```

#### Log Query

既存のクエリ構文 `key:value` に合わせて `limit` 属性を追加:

```
limit:5000
lim:5000          # エイリアス
pod:api limit:5000  # 他のフィルタと組み合わせ可能
```

#### Priority

ログクエリの `limit` > 設定ファイルの `max_lines` > デフォルト（制限なし）

### Filter Changes

`FilterAttribute` enum に `Limit(usize)` バリアントを追加。
`Filter` 構造体に `limit: Option<usize>` フィールドを追加。
パーサーに `limit` / `lim` キーワードのパース処理を追加。

### Test Plan

1. **TextItem の行数制限テスト**
   - `max_lines` 設定時に上限を超えたら古い行が削除されること
   - `max_lines` 未設定時は従来通り無制限に蓄積すること
   - 削除後のスクロール位置が正しく調整されること
   - 検索ハイライトが正しく更新されること

2. **クエリパーサーのテスト**
   - `limit:5000` が正しくパースされること
   - `lim:5000` エイリアスが動作すること
   - 他のフィルタとの組み合わせが動作すること
   - 不正な値（`limit:abc`、`limit:-1`）のエラーハンドリング

3. **設定ファイルのテスト**
   - `logging.max_lines` が正しく読み込まれること
   - 省略時に `None`（制限なし）になること
