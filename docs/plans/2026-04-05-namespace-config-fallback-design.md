# Namespace Config Fallback Design

## Background

namespace 一覧の取得権限がない環境では、`n`/`N` キーでnamespace選択ダイアログを開くとエラーが表示され、UIからnamespaceを切り替えられない。
CLIの `-n` フラグでnamespaceを指定するワークアラウンドはあるが、切り替えのたびにプログラムの再起動が必要になる。

参照: https://github.com/sarub0b0/kubetui/issues/927

## Design

### Approach

設定ファイル (`config.yaml`) に `namespaces` フィールドを追加する。
Kubernetes APIでnamespace一覧の取得に失敗した場合、設定ファイルの一覧をフォールバックとして表示する。

- API取得成功時: API結果を表示（従来通り）
- API取得失敗時 + 設定あり: 設定の一覧を表示
- API取得失敗時 + 設定なし: エラーメッセージを表示（従来通り）

エラー種別（403 Forbidden, ネットワークエラー等）による分岐は行わない。
設定のnamespaceが実際にアクセス可能かどうかは、選択後の操作時に判明するため、
フォールバック時点での細かいエラー判定は不要。

### Config File

```yaml
# ~/.config/kubetui/config.yaml
namespaces:
  - production
  - staging
  - dev
```

`Config` 構造体に `namespaces: Option<Vec<String>>` を追加。

### UI

フォールバック発動時、選択リストのタイトルを `Items (from config)` に変更してソースを明示する。

```
┌ select namespace ─────────────┐
│ Filter: [         ]           │
│┌ Items (from config) ────────┐│
││ production                  ││
││ staging                     ││
││ dev                         ││
│└─────────────────────────────┘│
└───────────────────────────────┘
```

### 対象

単一namespace選択（`n` キー）と複数namespace選択（`N` キー）の両方。

### 変更箇所

1. **`src/config.rs`**: `Config` に `namespaces: Option<Vec<String>>` 追加
2. **`src/workers/kube/controller.rs`**: `fetch_all_namespaces` 失敗時のフォールバックロジック追加
3. **`src/workers/render/action.rs`**: フォールバック時のリストタイトル変更対応
4. **`example/config.yaml`**: namespaces設定の例を追加
