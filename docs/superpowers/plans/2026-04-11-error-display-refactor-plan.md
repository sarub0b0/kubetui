# エラー表示リファクタリング実装計画

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 各ビューのデータ取得失敗時のエラー表示を、ウィジェット内部データを汚さずに Tab/Dialog 層で管理する方式にリファクタリングし、正常データ受信時に自動復帰する仕組みを実現する。

**Architecture:** Tab と Dialog にウィジェットごとのエラー状態（生のテキスト行）を保持するフィールドを追加。`Window::set_widget_error` / `clear_widget_error` メソッドで操作する。描画時、エラー状態があれば通常の widget.render() をスキップし、ウィジェットのブロック（タイトル・ボーダー）を維持しつつ中身をエラーテキスト描画で差し替える。スタイルは設定ファイル (`component.error`) から設定可能。ログは特殊で、ストリーム継続中の個別エラーは従来通りインライン追記するため `LogMessage::StreamError` バリアントを新設する。

**Tech Stack:** Rust, ratatui 0.30, anyhow, serde, figment

**Design Document:** `docs/superpowers/specs/2026-04-11-error-display-refactor-design.md`

---

### Task 1: `ErrorTheme` 型の定義（UI 層）

**Files:**
- Create: `src/ui/widget/error.rs`
- Modify: `src/ui/widget.rs` — `mod error;` と `pub use error::*;` を追加

**Step 1: 新規ファイルを作成**

`src/ui/widget/error.rs` を以下の内容で作成:

```rust
use ratatui::style::{Color, Style};

/// エラー表示用のテーマ
#[derive(Debug, Clone)]
pub struct ErrorTheme {
    pub style: Style,
}

impl Default for ErrorTheme {
    fn default() -> Self {
        Self {
            style: Style::default().fg(Color::Red),
        }
    }
}

impl ErrorTheme {
    pub fn style(mut self, style: impl Into<Style>) -> Self {
        self.style = style.into();
        self
    }
}
```

**Step 2: モジュール登録**

`src/ui/widget.rs` を開き、既存の `mod ...;` 宣言と `pub use ...;` 文が並んでいる箇所に以下を追加:

```rust
mod error;
```

`pub use` 文に以下を追加（アルファベット順を維持）:

```rust
pub use error::ErrorTheme;
```

**Step 3: ビルド確認**

Run: `cargo check 2>&1 | tail -20`
Expected: ビルド成功（既存の警告のみ、新しいエラーなし）

**Step 4: コミット**

```bash
git add src/ui/widget/error.rs src/ui/widget.rs
git commit -m "feat: add ErrorTheme for error display styling"
```

---

### Task 2: `render_widget_error` ヘルパー関数

**Files:**
- Modify: `src/ui/widget/error.rs` — `render_widget_error` 関数を追加

**Step 1: 関数シグネチャと実装を追加**

`src/ui/widget/error.rs` の末尾に以下を追加:

```rust
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Paragraph, Wrap},
    Frame,
};

/// ウィジェットのブロック（タイトル・ボーダー）内にエラーテキストを描画する。
///
/// - `chunk`: 描画領域
/// - `block`: ウィジェットのタイトル・ボーダーを含む Block
/// - `error_lines`: 表示するエラーテキストの行
/// - `theme`: エラー表示のテーマ
pub fn render_widget_error(
    f: &mut Frame,
    chunk: Rect,
    block: Block,
    error_lines: &[String],
    theme: &ErrorTheme,
) {
    let lines: Vec<Line> = error_lines
        .iter()
        .map(|line| Line::from(Span::styled(line.clone(), theme.style)))
        .collect();

    let paragraph = Paragraph::new(lines).block(block).wrap(Wrap { trim: false });

    f.render_widget(paragraph, chunk);
}
```

**Step 2: ビルド確認**

Run: `cargo check 2>&1 | tail -20`
Expected: ビルド成功

**Step 3: 単体テスト追加**

同じファイルの末尾に以下を追加:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, buffer::Buffer, widgets::Borders, Terminal};

    #[test]
    fn render_widget_error_draws_lines_with_style() {
        let backend = TestBackend::new(40, 6);
        let mut terminal = Terminal::new(backend).unwrap();

        let theme = ErrorTheme::default();
        let lines = vec!["error line 1".to_string(), "error line 2".to_string()];

        terminal
            .draw(|f| {
                let block = Block::default().borders(Borders::ALL).title("Title");
                render_widget_error(f, f.area(), block, &lines, &theme);
            })
            .unwrap();

        let buffer: &Buffer = terminal.backend().buffer();
        // Block タイトルが描画されていること
        let title_row: String = (0..40).map(|x| buffer[(x, 0)].symbol().to_string()).collect();
        assert!(title_row.contains("Title"), "title not rendered: {title_row}");
        // 最初の行にエラーが描画されていること
        let line1_row: String = (0..40).map(|x| buffer[(x, 1)].symbol().to_string()).collect();
        assert!(line1_row.contains("error line 1"), "error line 1 not rendered: {line1_row}");
    }
}
```

**Step 4: テスト実行**

Run: `cargo test --lib ui::widget::error 2>&1 | tail -30`
Expected: テストが PASS

**Step 5: コミット**

```bash
git add src/ui/widget/error.rs
git commit -m "feat: add render_widget_error helper"
```

---

### Task 3: `ErrorThemeConfig` の定義（設定層）

**Files:**
- Create: `src/config/theme/error.rs`
- Modify: `src/config/theme/widget.rs` — `WidgetThemeConfig` に `error` フィールド追加、`From<WidgetThemeConfig> for ErrorTheme` 実装追加
- Modify: `src/config/theme.rs` — `mod error;` と `pub use error::*;` を追加

**Step 1: `ErrorThemeConfig` の新規ファイルを作成**

`src/config/theme/error.rs` を以下の内容で作成:

```rust
use ratatui::style::Color;
use serde::{Deserialize, Serialize};

use crate::ui::widget::ErrorTheme;

use super::ThemeStyleConfig;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
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

impl From<ErrorThemeConfig> for ErrorTheme {
    fn from(config: ErrorThemeConfig) -> Self {
        ErrorTheme::default().style(config.style)
    }
}
```

**Step 2: モジュール登録**

`src/config/theme.rs` の `mod` 宣言ブロック（ファイル冒頭）に以下を追加:

```rust
mod error;
```

同じファイルの `pub use` ブロックに以下を追加:

```rust
pub use error::ErrorThemeConfig;
```

**Step 3: `WidgetThemeConfig` に `error` フィールドを追加**

`src/config/theme/widget.rs` の `WidgetThemeConfig` 構造体に `error` フィールドを追加:

```rust
#[derive(Default, Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct WidgetThemeConfig {
    #[serde(default)]
    pub base: ThemeStyleConfig,

    #[serde(default)]
    pub title: TitleThemeConfig,

    #[serde(default)]
    pub border: BorderThemeConfig,

    #[serde(default)]
    pub text: TextThemeConfig,

    #[serde(default)]
    pub table: TableThemeConfig,

    #[serde(default)]
    pub list: ListThemeConfig,

    #[serde(default)]
    pub check_list: CheckListThemeConfig,

    #[serde(default)]
    pub input: InputFormThemeConfig,

    #[serde(default)]
    pub dialog: DialogThemeConfig,

    #[serde(default)]
    pub error: ErrorThemeConfig,  // 追加
}
```

`super::` からのインポートに `ErrorThemeConfig` を追加:

```rust
use super::{
    BorderThemeConfig, CheckListThemeConfig, DialogThemeConfig, ErrorThemeConfig,
    FilterFormThemeConfig, InputFormThemeConfig, ListThemeConfig, TableThemeConfig,
    TextThemeConfig, ThemeStyleConfig,
};
```

同じファイルに `From<WidgetThemeConfig> for ErrorTheme` 実装を追加:

```rust
use crate::ui::widget::{
    multiple_select, single_select, table, CheckListTheme, ErrorTheme, InputFormTheme, ListTheme,
    SearchFormTheme, TableTheme, TextTheme, WidgetTheme,
};
```

`From` 実装ブロック群の末尾に追加:

```rust
impl From<WidgetThemeConfig> for ErrorTheme {
    fn from(theme: WidgetThemeConfig) -> Self {
        theme.error.into()
    }
}
```

**Step 4: 既存テストの更新**

`src/config/theme/widget.rs` の `test_serialize_widget_theme_config`, `test_deserialize_widget_theme_config`, `default_widget_theme_config` テストが `WidgetThemeConfig` の全フィールドを比較しているので、新しい `error` フィールドに対応する必要がある。

`default_widget_theme_config` テストの `expected` に追加:

```rust
let expected = WidgetThemeConfig {
    // 既存フィールド...
    dialog: DialogThemeConfig::default(),
    error: ErrorThemeConfig::default(),  // 追加
};
```

`test_serialize_widget_theme_config` の `theme` 構築に `error: ErrorThemeConfig::default()` を追加し、`expected` の YAML 文字列の末尾（`dialog:` ブロックの後）に以下を追加:

```yaml
error:
  style:
    fg_color: red
```

`test_deserialize_widget_theme_config` の `data` YAML にも同じ `error:` ブロックを追加し、`expected` 構築に `error: ErrorThemeConfig::default()` を追加。

**Step 5: ビルドとテスト確認**

Run: `cargo test --lib config::theme::widget 2>&1 | tail -30`
Expected: すべてのテストが PASS

**Step 6: コミット**

```bash
git add src/config/theme/error.rs src/config/theme.rs src/config/theme/widget.rs
git commit -m "feat: add ErrorThemeConfig for configurable error style"
```

---

### Task 4: `Tab` にエラー状態フィールドを追加

**Files:**
- Modify: `src/ui/tab.rs` — `Tab` 構造体にフィールド追加、`new` 変更、メソッド追加

**Step 1: インポート追加**

`src/ui/tab.rs` のインポート部（冒頭の `use super::...` 近辺）に追加:

```rust
use std::collections::HashMap;
```

`use super::widget::*;` は既にあるので、`ErrorTheme` は `widget::*` 経由で使える（Task 1 で pub use 済み）。

**Step 2: `Tab` 構造体にフィールドを追加**

```rust
pub struct Tab<'a> {
    id: String,
    title: String,
    chunk: Rect,
    layout: TabLayout,
    widgets: Vec<Widget<'a>>,
    active_widget_index: usize,
    activatable_widget_indices: Vec<usize>,
    mouse_over_widget_index: Option<usize>,
    dragging_widget_index: Option<usize>,
    error_states: HashMap<String, Vec<String>>,  // 追加
    error_theme: ErrorTheme,                      // 追加
}
```

**Step 3: `new` メソッドで初期化**

既存の `new` メソッドを以下のように変更:

```rust
pub fn new(
    id: impl Into<String>,
    title: impl Into<String>,
    widgets: impl Into<Vec<Widget<'a>>>,
    layout: TabLayout,
) -> Self {
    let widgets: Vec<_> = widgets.into();

    let activatable_widget_indices = widgets
        .iter()
        .enumerate()
        .filter(|(_, w)| w.can_activate())
        .map(|(i, _)| i)
        .collect();

    Self {
        id: id.into(),
        title: title.into(),
        chunk: Rect::default(),
        layout,
        widgets,
        activatable_widget_indices,
        active_widget_index: 0,
        mouse_over_widget_index: None,
        dragging_widget_index: None,
        error_states: HashMap::new(),        // 追加
        error_theme: ErrorTheme::default(),  // 追加
    }
}
```

**Step 4: エラー状態管理メソッドを追加**

`Tab<'a>` の `impl` ブロック内に以下のメソッドを追加:

```rust
/// 指定ウィジェットのエラー状態を設定する。
pub fn set_widget_error(&mut self, widget_id: &str, lines: Vec<String>) {
    self.error_states.insert(widget_id.to_string(), lines);
}

/// 指定ウィジェットのエラー状態をクリアする。
pub fn clear_widget_error(&mut self, widget_id: &str) {
    self.error_states.remove(widget_id);
}

/// 指定ウィジェットがこのタブに含まれているかを返す。
pub fn contains_widget(&self, widget_id: &str) -> bool {
    self.widgets.iter().any(|w| w.id() == widget_id)
}

/// エラーテーマを設定する（ビルダーパターン用）。
pub fn error_theme(mut self, theme: ErrorTheme) -> Self {
    self.error_theme = theme;
    self
}
```

**Step 5: ビルド確認**

Run: `cargo check 2>&1 | tail -20`
Expected: ビルド成功

**Step 6: コミット**

```bash
git add src/ui/tab.rs
git commit -m "feat: add error state management to Tab"
```

---

### Task 5: `Tab::render` でエラー状態を考慮した描画

**Files:**
- Modify: `src/ui/tab.rs` — `render` メソッド変更

**Step 1: `render` メソッドを変更**

既存の `render` を以下に置き換え:

```rust
impl Tab<'_> {
    pub fn render(&mut self, f: &mut Frame) {
        let active_index = self.active_widget_index;
        let mouse_over_index = self.mouse_over_widget_index;
        let error_theme = &self.error_theme;
        let error_states = &self.error_states;

        self.widgets.iter_mut().enumerate().for_each(|(i, w)| {
            let is_active = i == active_index;
            let is_mouse_over = mouse_over_index.is_some_and(|idx| idx == i);

            if let Some(error_lines) = error_states.get(w.id()) {
                // エラー状態: widget のブロックを取得してエラーテキストで中身を差し替え
                let block = w
                    .widget_base()
                    .render_block(w.can_activate() && is_active, is_mouse_over);
                super::widget::render_widget_error(
                    f,
                    w.chunk(),
                    block,
                    error_lines,
                    error_theme,
                );
            } else {
                w.render(f, is_active, is_mouse_over);
            }
        });
    }
}
```

**Step 2: `render_widget_error` を `pub use` で公開**

`src/ui/widget.rs` の `pub use` ブロックで `render_widget_error` も export:

```rust
pub use error::{render_widget_error, ErrorTheme};
```

**Step 3: ビルド確認**

Run: `cargo check 2>&1 | tail -30`
Expected: ビルド成功

**Step 4: コミット**

```bash
git add src/ui/tab.rs src/ui/widget.rs
git commit -m "feat: render error state in Tab::render"
```

---

### Task 6: `Dialog` にエラー状態フィールドを追加

**Files:**
- Modify: `src/ui/dialog.rs` — `Dialog`, `DialogTheme`, `DialogBuilder` に変更

**Step 1: インポート追加**

`src/ui/dialog.rs` の先頭 `use super::...` に `ErrorTheme` を追加:

```rust
use super::{
    event::EventResult,
    widget::{render_widget_error, ErrorTheme, RenderTrait, StyledClear, Text, Widget, WidgetTrait},
};
```

**Step 2: `DialogTheme` に `error_theme` フィールドを追加**

```rust
#[derive(Debug, Default, Clone)]
pub struct DialogTheme {
    pub base_style: Style,
    pub size: DialogSize,
    pub error_theme: ErrorTheme,  // 追加
}

impl DialogTheme {
    pub fn base_style(mut self, style: impl Into<Style>) -> Self {
        self.base_style = style.into();
        self
    }

    pub fn size(mut self, size: impl Into<DialogSize>) -> Self {
        self.size = size.into();
        self
    }

    // 追加
    pub fn error_theme(mut self, theme: ErrorTheme) -> Self {
        self.error_theme = theme;
        self
    }
}
```

**Step 3: `Dialog` 構造体にフィールド追加**

```rust
pub struct Dialog<'a> {
    widget: Widget<'a>,
    chunk: Rect,
    chunk_size: DialogSize,
    base_style: Style,
    error_state: Option<Vec<String>>,  // 追加
    error_theme: ErrorTheme,           // 追加
}
```

**Step 4: `DialogBuilder::build` を変更**

```rust
pub fn build(self) -> Dialog<'a> {
    Dialog {
        widget: self.widget,
        chunk: Default::default(),
        chunk_size: self.theme.size,
        base_style: self.theme.base_style,
        error_state: None,
        error_theme: self.theme.error_theme,
    }
}
```

**Step 5: `Dialog::new` も同様に変更**

```rust
#[allow(dead_code)]
pub fn new(widget: Widget<'a>) -> Self {
    Self {
        widget,
        chunk: Default::default(),
        chunk_size: Default::default(),
        base_style: Style::default(),
        error_state: None,
        error_theme: ErrorTheme::default(),
    }
}
```

**Step 6: エラー管理メソッドを追加**

`impl<'a> Dialog<'a>` ブロックに追加:

```rust
/// ダイアログ内ウィジェットのエラー状態を設定する。
pub fn set_widget_error(&mut self, lines: Vec<String>) {
    self.error_state = Some(lines);
}

/// ダイアログ内ウィジェットのエラー状態をクリアする。
pub fn clear_widget_error(&mut self) {
    self.error_state = None;
}
```

**Step 7: `render` メソッドをエラー状態対応に変更**

```rust
pub fn render(&mut self, f: &mut Frame) {
    f.render_widget(StyledClear::new(self.base_style), self.chunk);

    if let Some(error_lines) = &self.error_state {
        let block = self.widget.widget_base().render_block(true, false);
        render_widget_error(
            f,
            self.widget.chunk(),
            block,
            error_lines,
            &self.error_theme,
        );
    } else {
        self.widget.render(f, true, false)
    }
}
```

**Step 8: ビルド確認**

Run: `cargo check 2>&1 | tail -30`
Expected: ビルド成功

**Step 9: コミット**

```bash
git add src/ui/dialog.rs
git commit -m "feat: add error state management to Dialog"
```

---

### Task 7: `Window` に `set_widget_error` / `clear_widget_error` メソッドを追加

**Files:**
- Modify: `src/ui/window.rs` — メソッド追加

**Step 1: メソッドを追加**

`impl Window<'_>` の適切な場所（他のヘルパーメソッドの近く）に追加:

```rust
/// 指定ウィジェットにエラー状態を設定する。
/// anyhow::Error を debug format で行分割し、生テキストとして保存する。
pub fn set_widget_error(&mut self, id: &str, error: &anyhow::Error) {
    let lines: Vec<String> = format!("{:?}", error)
        .lines()
        .map(String::from)
        .collect();

    // Dialog → Tab の順で検索
    if let Some(dialog) = self.dialogs.iter_mut().find(|d| d.id() == id) {
        dialog.set_widget_error(lines);
        return;
    }

    if let Some(tab) = self.tabs.iter_mut().find(|t| t.contains_widget(id)) {
        tab.set_widget_error(id, lines);
    }
}

/// 指定ウィジェットのエラー状態をクリアする。
pub fn clear_widget_error(&mut self, id: &str) {
    if let Some(dialog) = self.dialogs.iter_mut().find(|d| d.id() == id) {
        dialog.clear_widget_error();
        return;
    }

    if let Some(tab) = self.tabs.iter_mut().find(|t| t.contains_widget(id)) {
        tab.clear_widget_error(id);
    }
}
```

**Step 2: ビルド確認**

Run: `cargo check 2>&1 | tail -20`
Expected: ビルド成功

**Step 3: コミット**

```bash
git add src/ui/window.rs
git commit -m "feat: add set_widget_error/clear_widget_error to Window"
```

---

### Task 8: 設定ファイルから `ErrorTheme` を Tab/Dialog に受け渡す

**Files:**
- Modify: `src/config/theme.rs` — `From<ThemeConfig> for DialogTheme` を更新
- Modify: Tab 構築箇所 — 各 feature の tab.rs で `error_theme` を設定

**Step 1: `DialogTheme` 変換を更新**

`src/config/theme.rs` の `impl From<ThemeConfig> for DialogTheme` を以下に変更:

```rust
impl From<ThemeConfig> for DialogTheme {
    fn from(config: ThemeConfig) -> Self {
        let base_style = config.component.dialog.base.unwrap_or_else(|| *config.base);
        let error_theme = config.component.error.clone().into();

        DialogTheme::default()
            .base_style(base_style)
            .size(config.component.dialog.size)
            .error_theme(error_theme)
    }
}
```

**Step 2: Tab 構築箇所を調査**

Run: `rg 'Tab::new\(' src/features --type rust -l`
Expected: 複数の tab.rs ファイルのリスト

**Step 3: Tab 構築箇所を更新**

各 feature の tab.rs で `Tab::new(...)` を呼んでいる箇所の末尾に `.error_theme(...)` を追加する必要がある。ただし、`Tab` の構築は各 feature で theme を受け取っているため、`ThemeConfig` もしくは `WidgetThemeConfig` から `ErrorTheme` を取り出して渡す形にする。

まず、各 tab.rs が `WidgetThemeConfig` を受け取っているかを確認:

Run: `rg 'WidgetThemeConfig|ThemeConfig' src/features --type rust -l | head -20`

**Step 4: 各 Tab 構築で error_theme を設定**

見つかった各 feature の tab.rs で、`Tab::new(...)` の呼び出し後に `.error_theme(widget_theme.error.clone().into())` を追加する。

例:
```rust
// Before
Tab::new(POD_TAB_ID, title, widgets, layout)

// After
Tab::new(POD_TAB_ID, title, widgets, layout)
    .error_theme(widget_theme.error.clone().into())
```

※具体的な受け渡し方法は既存のテーマ受け渡しパターンに合わせる。

**Step 5: ビルド確認**

Run: `cargo check 2>&1 | tail -30`
Expected: ビルド成功

**Step 6: コミット**

```bash
git add -A
git commit -m "feat: wire ErrorTheme from config to Tab/Dialog"
```

---

### Task 9: `LogMessage::StreamError` バリアントを追加

**Files:**
- Modify: `src/features/pod/message.rs` — `LogMessage` に `StreamError` バリアント追加

**Step 1: メッセージ定義を探す**

Run: `rg 'enum LogMessage' src/features/pod --type rust`
Expected: `src/features/pod/message.rs` またはその近く

**Step 2: `StreamError` バリアントを追加**

該当ファイルの `LogMessage` enum に追加:

```rust
pub enum LogMessage {
    Request(...),
    Response(Result<Vec<String>, anyhow::Error>),
    SetMaxLines(...),
    StreamError(String),  // 新規追加
}
```

**Step 3: ビルド確認**

Run: `cargo check 2>&1 | tail -20`
Expected: match 網羅性エラーが出るファイルを確認（後のタスクで処理）

**Step 4: コミット**

```bash
git add src/features/pod/message.rs
git commit -m "feat: add LogMessage::StreamError variant"
```

---

### Task 10: `pod_watcher` の Error イベントを `StreamError` に変更

**Files:**
- Modify: `src/features/pod/kube/log/pod_watcher.rs:141-146` — `Error(err)` の送信を変更

**Step 1: 該当箇所を変更**

```rust
// Before
Error(err) => {
    if let Err(e) = self.tx.send(LogMessage::Response(Err(anyhow!(err))).into()) {
        logger!(error, "Failed to send LogMessage::Response: {}", e);
        return;
    }
}

// After
Error(err) => {
    if let Err(e) = self
        .tx
        .send(LogMessage::StreamError(err.to_string()).into())
    {
        logger!(error, "Failed to send LogMessage::StreamError: {}", e);
        return;
    }
}
```

**Step 2: ビルド確認**

Run: `cargo check 2>&1 | tail -20`
Expected: ビルド成功（`anyhow!` の import が不要になっていれば削除）

**Step 3: コミット**

```bash
git add src/features/pod/kube/log/pod_watcher.rs
git commit -m "refactor: send StreamError from pod_watcher on watch errors"
```

---

### Task 11: `action.rs` で `update_widget_item_for_table` を変更

**Files:**
- Modify: `src/workers/render/action.rs:100-171` — Err 分岐を `set_widget_error` 呼び出しに変更、Ok 分岐で `clear_widget_error` 呼び出し

**Step 1: `update_widget_item_for_table` を変更**

```rust
fn update_widget_item_for_table(window: &mut Window, id: &str, table: Result<KubeTable>) {
    match table {
        Ok(table) => {
            window.clear_widget_error(id);
            let widget = window.find_widget_mut(id);
            let w = widget.as_mut_table();

            if w.equal_header(table.header()) {
                w.update_widget_item(Item::Table(
                    table
                        .rows
                        .into_iter()
                        .map(
                            |KubeTableRow {
                                 namespace,
                                 name,
                                 metadata,
                                 row,
                             }| {
                                let mut item_metadata = BTreeMap::from([
                                    ("namespace".to_string(), namespace),
                                    ("name".to_string(), name),
                                ]);

                                if let Some(metadata) = metadata {
                                    item_metadata.extend(metadata);
                                }

                                TableItem {
                                    metadata: Some(item_metadata),
                                    item: row,
                                }
                            },
                        )
                        .collect(),
                ));
            } else {
                let rows: Vec<TableItem> = table
                    .rows
                    .into_iter()
                    .map(
                        |KubeTableRow {
                             namespace,
                             name,
                             metadata,
                             row,
                         }| {
                            let mut item_metadata = BTreeMap::from([
                                ("namespace".to_string(), namespace),
                                ("name".to_string(), name),
                            ]);

                            if let Some(metadata) = metadata {
                                item_metadata.extend(metadata);
                            }

                            TableItem {
                                metadata: Some(item_metadata),
                                item: row,
                            }
                        },
                    )
                    .collect();

                w.update_header_and_rows(&table.header, &rows);
            }
        }
        Err(e) => {
            window.set_widget_error(id, &e);
        }
    }
}
```

**Step 2: ビルド確認**

Run: `cargo check 2>&1 | tail -20`
Expected: ビルド成功

**Step 3: コミット**

```bash
git add src/workers/render/action.rs
git commit -m "refactor: use set_widget_error for table error display"
```

---

### Task 12: `action.rs` で `update_widget_item_for_vec` を変更

**Files:**
- Modify: `src/workers/render/action.rs:173-183` — Err 分岐変更、Ok 分岐で clear

**Step 1: `update_widget_item_for_vec` を変更**

```rust
fn update_widget_item_for_vec(window: &mut Window, id: &str, vec: Result<Vec<String>>) {
    match vec {
        Ok(i) => {
            window.clear_widget_error(id);
            let widget = window.find_widget_mut(id);
            widget.update_widget_item(Item::Array(
                i.into_iter().map(LiteralItem::from).collect(),
            ));
        }
        Err(e) => {
            window.set_widget_error(id, &e);
        }
    }
}
```

**Step 2: ビルド確認**

Run: `cargo check 2>&1 | tail -20`
Expected: ビルド成功

**Step 3: コミット**

```bash
git add src/workers/render/action.rs
git commit -m "refactor: use set_widget_error for vec error display"
```

---

### Task 13: `action.rs` のログメッセージ処理を変更

**Files:**
- Modify: `src/workers/render/action.rs:196-219` — `LogMessage::Response` 処理を変更、`StreamError` 追加

**Step 1: ログメッセージ処理を変更**

```rust
Kube::Log(LogMessage::Response(res)) => {
    let widget = window.find_widget_mut(POD_LOG_WIDGET_ID);

    match res {
        Ok(i) => {
            window.clear_widget_error(POD_LOG_WIDGET_ID);
            let widget = window.find_widget_mut(POD_LOG_WIDGET_ID);
            let array = i
                .into_iter()
                .map(|i| LiteralItem {
                    metadata: None,
                    item: convert_tabs_to_spaces(i),
                })
                .collect();
            widget.append_widget_item(Item::Array(array));
        }
        Err(e) => {
            window.set_widget_error(POD_LOG_WIDGET_ID, &e);
        }
    }
}

Kube::Log(LogMessage::StreamError(msg)) => {
    // ストリーム継続中のエラー: ログにインライン追記（エラー状態はクリアしない）
    let widget = window.find_widget_mut(POD_LOG_WIDGET_ID);
    let item = LiteralItem {
        metadata: None,
        item: msg,
    };
    widget.append_widget_item(Item::Array(vec![item]));
}
```

※ 最初の `let widget = ...` は削除するか、`Ok` ブランチで再取得する形に整える。

**Step 2: ビルド確認**

Run: `cargo check 2>&1 | tail -30`
Expected: ビルド成功

**Step 3: コミット**

```bash
git add src/workers/render/action.rs
git commit -m "refactor: handle Log Response and StreamError with error state"
```

---

### Task 14: `action.rs` の残りのエラー処理を変更

**Files:**
- Modify: `src/workers/render/action.rs:239-291` — Namespace Err 処理
- Modify: `src/workers/render/action.rs:335-366` — API Dialog Err 処理
- Modify: `src/workers/render/action.rs:368-446` — YAML Err 処理

**Step 1: Namespace エラーを変更**

```rust
Kube::Namespace(NamespaceMessage::Response(res)) => match res {
    NamespaceResponse::Get(res) => match res {
        Ok(namespaces) => {
            window.clear_widget_error(MULTIPLE_NAMESPACES_DIALOG_ID);
            window.clear_widget_error(SINGLE_NAMESPACE_DIALOG_ID);
            // 既存の Ok 処理...
        }
        Err(err) => {
            window.set_widget_error(MULTIPLE_NAMESPACES_DIALOG_ID, &err);
            window.set_widget_error(SINGLE_NAMESPACE_DIALOG_ID, &err);
        }
    },
    // 既存の他のバリアント...
}
```

**Step 2: API Dialog エラーを変更**

`Kube::Api(ApiMessage::Response(res))` の `Get(apis)` 内:

```rust
Get(apis) => {
    match apis {
        Ok(i) => {
            window.clear_widget_error(API_DIALOG_ID);
            let widget = window.find_widget_mut(API_DIALOG_ID);
            let items = i
                .into_iter()
                .map(|api_resource| { /* 既存のまま */ })
                .collect();
            widget.update_widget_item(Item::Array(items));
        }
        Err(e) => {
            window.set_widget_error(API_DIALOG_ID, &e);
        }
    }
}
```

**Step 3: YAML エラーを変更**

`Kube::Yaml(YamlMessage::Response(ev))` 配下の各 Err 分岐も同様に `set_widget_error` に変更:

- `APIs(res)` の Err → `window.set_widget_error(YAML_KIND_DIALOG_ID, &e)`
- `Resource(res)` の Err → `window.set_widget_error(YAML_NAME_DIALOG_ID, &e)`
- 対応する Ok 分岐の先頭に `window.clear_widget_error(...)` を追加

**Step 4: `error_format!` / `error_lines!` マクロを削除**

すべての呼び出しを置き換えたら、ファイル冒頭のマクロ定義を削除:

```rust
// この2つを削除
macro_rules! error_format { ... }
macro_rules! error_lines { ... }
```

**Step 5: ビルド確認**

Run: `cargo check 2>&1 | tail -30`
Expected: ビルド成功、未使用のインポートがあれば削除

**Step 6: コミット**

```bash
git add src/workers/render/action.rs
git commit -m "refactor: migrate all error displays to set_widget_error"
```

---

### Task 15: テスト - 全体のビルドとクリッピー

**Files:**
- 変更なし

**Step 1: 全テスト実行**

Run: `cargo test --all 2>&1 | tail -40`
Expected: すべてのテストが PASS

**Step 2: clippy チェック**

Run: `cargo clippy 2>&1 | tail -30`
Expected: 警告が既存のもののみ（新しい警告なし）

**Step 3: フォーマット確認**

Run: `cargo +nightly fmt --check 2>&1 | tail -10`
Expected: 出力なし（フォーマット済み）

必要に応じて:

Run: `cargo +nightly fmt`

**Step 4: コミット（フォーマット変更があれば）**

```bash
git add -A
git commit -m "style: apply rustfmt"
```

---

### Task 16: 動作確認（手動）

**Files:**
- 変更なし

**Step 1: ビルド**

Run: `cargo build 2>&1 | tail -10`
Expected: ビルド成功

**Step 2: 手動動作確認（ユーザーへ依頼）**

ユーザーに以下を依頼:

1. 無効な kubeconfig または未接続のクラスタで kubetui を起動し、各タブで赤色のエラー表示が出ることを確認
2. エラー中に接続が復旧すると、各タブが通常表示に自動復帰することを確認
3. ログタブで不正な query を入力し、エラー表示に切り替わることを確認
4. `component.error` を設定ファイルで変更（例: `fg_color: yellow`）し、色が変わることを確認

**Step 3: 確認完了後、最終コミット不要**

（動作確認のみ、コード変更なし）

---

## スコープ外（別タスク）

- `Message::Error(NotifyError)` の UI 表示
- 複数 Namespace 指定時の部分失敗対応
- LogWorker のラベルセレクター取得の部分失敗対応
