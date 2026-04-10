# エラー表示リファクタリング設計

## 概要

各ビューのデータ取得失敗時に、ウィジェットの内部データを汚すことなく、Tab/Dialog 層でエラー状態を管理してエラー表示を行う。正常データ受信時には自動的に元の表示に復帰する。

## 背景と課題

現在、ビューのデータ取得に失敗したとき、各ウィジェットタイプに応じて以下のような無理やりな表示を行っている：

- **Table ウィジェット** (Pod, Config, Network 一覧等): ヘッダーを `["ERROR"]` に差し替え、エラーメッセージを単一行のテーブル行として表示
- **Text/List ウィジェット** (Log, Event, YAML 等): エラーを ANSI 赤色コード (`\x1b[31m...\x1b[39m`) 付きの `LiteralItem` に変換して通常のテキスト行として表示
- **MultipleSelect/SingleSelect** (Namespace 等): 同様にエラー行を選択肢として表示

これらはすべて、そのウィジェットが本来想定していないデータ型をエラー表示のために流用している。加えて、`error_format!` / `error_lines!` マクロで ANSI エスケープコードを手動埋め込みしており、ratatui の Style システムを使った本来の描画方法から外れている。

## 方針

### 基本設計

ウィジェット自体にはエラーの知識を持たせず、**Tab/Dialog 層がウィジェットごとのエラー状態を管理**する。エラー時には通常のウィジェット描画をスキップし、同じ領域に専用のエラー表示を行う。正常データ受信時には自動的に元のウィジェット描画に戻る。

```
Err(e) → window.set_widget_error(id, &e)
         → Tab/Dialog が生のエラー行を保存
         → render() 時に該当ウィジェットの領域にエラーテキストを描画
         → 元のウィジェットのブロック（タイトル・ボーダー）は維持

Ok(data) → window.clear_widget_error(id)
           → Tab/Dialog からエラー状態削除
           → widget.update_widget_item(data) で通常更新
           → 通常描画に復帰
```

### 採用アプローチ

検討した候補：

- **A: WidgetBase エラー状態方式** — `WidgetBase` に `error_state` を追加
- **B: Error バリアント追加方式** — Widget enum に `Error(ErrorWidget)` を追加して Vec で差し替え
- **C: render 時オーバーレイ方式** — 既存ウィジェットの描画後にオーバーレイを重ねる

最終的に採用するのは、**C に近い「Tab/Dialog 層でエラー状態を管理し、描画時に生テキストへスタイルを適用する方式」**。理由：

- エラー表示の責務をウィジェットの深い層（`WidgetBase`）ではなく、ウィジェットを管理する Tab/Dialog に持たせるのが自然
- 各ウィジェットの実装を変更せずに済み、関心の分離が明確
- データ（生のエラーテキスト）とスタイリング（描画時の `Style` 適用）を分離でき、ANSI エスケープコードの埋め込みが不要になる
- ratatui の `Style` システムに統一でき、将来的なテーマ変更にも対応しやすい

## 詳細設計

### 1. Tab/Dialog のエラー状態管理

```rust
// src/ui/tab.rs
pub struct Tab<'a> {
    // 既存フィールド...
    error_states: HashMap<String, Vec<String>>,  // widget_id → 生のエラー行
    error_theme: ErrorTheme,
}

// src/ui/dialog.rs
pub struct Dialog<'a> {
    // 既存フィールド...
    error_state: Option<Vec<String>>,  // 単一ウィジェットなので Option
    error_theme: ErrorTheme,
}
```

エラー行は `String` の `Vec` として保持する（`LiteralItem` やスタイル付き型ではなく、純粋なテキストのみ）。スタイリングは描画時に適用する。

### 2. Window のインターフェース

```rust
// src/ui/window.rs
impl Window<'_> {
    /// 指定ウィジェットにエラー状態を設定する。
    /// anyhow::Error を debug format で行分割し、生テキストとして保存する。
    pub fn set_widget_error(&mut self, id: &str, error: &anyhow::Error);

    /// 指定ウィジェットのエラー状態をクリアする。
    pub fn clear_widget_error(&mut self, id: &str);
}
```

内部実装:

1. Dialog → Tab の順で対象ウィジェットを検索
2. 該当する Tab/Dialog の `error_states` に `format!("{:?}", error).lines().map(String::from).collect()` の結果を保存
3. `clear_widget_error` は該当エントリを削除

### 3. 描画

Tab/Dialog の `render()` 内で、各ウィジェット描画時に error_states を確認：

```rust
// Tab::render() 内（概念）
for widget in &mut self.widgets {
    if let Some(error_lines) = self.error_states.get(widget.id()) {
        // ウィジェットのブロック（タイトル・ボーダー）は維持し、
        // 中身をエラー表示で差し替える
        render_widget_error(
            f,
            widget,
            error_lines,
            &self.error_theme,
            is_active,
            is_mouse_over,
        );
    } else {
        widget.render(f, is_active, is_mouse_over);
    }
}
```

`render_widget_error` ヘルパーの挙動:

- ウィジェットの `chunk` 領域を取得
- ウィジェットの `widget_base` からブロック（タイトル・ボーダー）を取得して維持
- ratatui の `Paragraph` にエラー行をセット、`error_theme` のスタイルを適用
- ブロック内部にエラーテキストを描画
- プレフィックス（従来の `[kubetui]`）は付与しない

### 4. 設定ファイルからのスタイル設定

エラー表示用のスタイルを `WidgetThemeConfig` に追加する：

```rust
// src/config/theme/widget.rs
pub struct WidgetThemeConfig {
    // 既存フィールド...
    #[serde(default)]
    pub error: ErrorThemeConfig,
}

// src/config/theme/error.rs (新規)
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ErrorThemeConfig {
    #[serde(default = "default_error_style")]
    pub style: ThemeStyleConfig,
}

impl Default for ErrorThemeConfig {
    fn default() -> Self {
        Self {
            style: default_error_style(),
        }
    }
}

fn default_error_style() -> ThemeStyleConfig {
    ThemeStyleConfig {
        fg_color: Some(Color::Red),
        ..Default::default()
    }
}
```

YAML 設定例:

```yaml
component:
  error:
    style:
      fg_color: red
      modifier: bold
```

UI 層には `ErrorTheme` 型を定義し、`ErrorThemeConfig` から `From` 変換。Tab/Dialog 構築時に受け渡す。

### 5. action.rs の変更

`src/workers/render/action.rs` の `update_widget_item_for_table`, `update_widget_item_for_vec` と、個別に展開されているエラー処理を変更する：

```rust
// 変更前（Table）
Err(e) => {
    let rows: Vec<TableItem> = vec![vec![error_format!("{:?}", e)].into()];
    w.update_header_and_rows(&["ERROR".to_string()], &rows);
}

// 変更後（Table）
Err(e) => {
    window.set_widget_error(id, &e);
}

// 変更前（Vec）
Err(e) => {
    widget.update_widget_item(Item::Array(error_lines!(e)));
}

// 変更後（Vec）
Err(e) => {
    window.set_widget_error(id, &e);
}

// Ok 分岐ではクリアしてから更新
Ok(data) => {
    window.clear_widget_error(id);
    // 既存の update_widget_item 処理...
}
```

`error_format!` / `error_lines!` マクロは廃止する。

### 6. ログエラーの特別扱い

ログウィジェットは唯一「append 更新」を行っているウィジェットで、複数の並行タスク（複数 Pod/コンテナの log stream）が動作するという特性を持つ。このため、ストリーム継続中の個別エラーでウィジェット全体をエラー表示に差し替えると、正常にストリーミングできている Pod のログが隠れてしまう。

そこで、`LogMessage` に新しいバリアントを追加する：

```rust
// src/features/pod/message.rs
pub enum LogMessage {
    Request(...),
    Response(Result<Vec<String>>),  // 既存: 取得前エラー or 通常データ
    StreamError(String),             // 新規: ストリーム継続中の個別エラー
    SetMaxLines(...),
}
```

**送信元の振り分け:**

| 箇所 | ファイル | 扱い |
|---|---|---|
| `spawn_tasks` 失敗（ラベルセレクタ取得エラー等） | `src/features/pod/kube/log.rs` | `Response(Err(_))` のまま → エラー表示差し替え |
| `Filter::parse` 失敗（クエリ構文エラー） | `src/features/pod/kube/log.rs` | `Response(Err(_))` のまま → エラー表示差し替え |
| `pod_watcher` の Error イベント（watch 中の個別エラー） | `src/features/pod/kube/log/pod_watcher.rs` | `StreamError(msg)` へ変更 → ログにインライン追記 |

上記の振り分けの根拠:

- `spawn_tasks` と `Filter::parse` は、いずれもログストリーミング開始前に失敗する。失敗時点で並行タスクは存在しないため、ウィジェット全体の差し替えで問題ない
- `pod_watcher` の Error イベントは、複数 Pod を watch 中の個別エラーであり、他の Pod のストリームが正常に動作している可能性がある。ストリームに混ぜて表示することで、正常なログを隠さずにエラー情報も伝えられる

**action.rs の LogMessage 処理:**

- `Response(Ok(_))` → `window.clear_widget_error(POD_LOG_WIDGET_ID)` → 通常 append
- `Response(Err(e))` → `window.set_widget_error(POD_LOG_WIDGET_ID, &e)`
- `StreamError(msg)` → エラー状態はクリアせず、ログウィジェットに append

## 影響範囲

| 変更種別 | ファイル | 内容 |
|---|---|---|
| 変更 | `src/ui/tab.rs` | `error_states` と `error_theme` フィールド追加、render でエラー分岐 |
| 変更 | `src/ui/dialog.rs` | `error_state` と `error_theme` フィールド追加、render でエラー分岐 |
| 変更 | `src/ui/window.rs` | `set_widget_error` / `clear_widget_error` メソッド追加 |
| 新規 | `src/ui/widget/error.rs` | `ErrorTheme` 型と `render_widget_error` ヘルパー |
| 新規 | `src/config/theme/error.rs` | `ErrorThemeConfig` 定義 |
| 変更 | `src/config/theme/widget.rs` | `WidgetThemeConfig` に `error` フィールド追加 |
| 変更 | `src/workers/render/action.rs` | Err 分岐で `set_widget_error` 呼び出し、Ok 分岐で `clear_widget_error` 呼び出し、`error_format!` / `error_lines!` マクロ削除 |
| 変更 | `src/features/pod/message.rs` | `LogMessage::StreamError` 追加 |
| 変更 | `src/features/pod/kube/log/pod_watcher.rs` | Error イベント送信を `StreamError` に変更 |
| 変更 | Tab/Dialog の呼び出し元 | `error_theme` を渡すためコンストラクタ変更 |

## テスト方針

- 各ウィジェット種別（Table, Text, List, SingleSelect, MultipleSelect）でエラー表示 → 正常データ受信 → 元の表示に自動復帰することを確認
- エラー表示中にウィジェットのタイトル・ボーダーが維持されることを確認
- 設定ファイルから `component.error.style` を変更して描画スタイルが反映されることを確認
- ログウィジェットで `StreamError` がインライン追記され、他の正常ログが隠れないことを確認
- ログウィジェットで `Response(Err(_))` がエラー表示差し替えになることを確認

## スコープ外

- `Message::Error(NotifyError)` の UI 表示
  - 現状通りログ出力のみ
  - クラスタ切り替え時の Namespace フォールバックや Worker プロセスのクラッシュ時にのみ発生し、通常のポーリング中には起きないため今回は対象外
- 複数 Namespace 指定時の部分失敗対応
  - 現状は `try_join_all` のため 1 つでも失敗すると全体が Err になる
  - Poller 層の大きな変更が必要になるため別タスクで扱う
- LogWorker のラベルセレクター取得の部分失敗対応
  - こちらも改善余地はあるが、今回はスコープ外
