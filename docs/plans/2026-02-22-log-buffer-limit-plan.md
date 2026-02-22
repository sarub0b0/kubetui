# Log Buffer Limit Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** ログビューに行数ベースのローリングバッファを実装し、メモリ使用量の無制限増加を防止する。

**Architecture:** TextItem の `Vec<Line>` / `Vec<WrappedLine>` を `VecDeque` に変更し、`max_lines` 設定で上限超過時に先頭行を破棄する。設定は Config（YAML）とログクエリの `limit` 属性の2箇所から指定可能。デフォルトは制限なし（後方互換性維持）。

**Tech Stack:** Rust, nom (parser), figment (config), ratatui, VecDeque

**Design Doc:** `docs/plans/2026-02-22-log-buffer-limit-design.md`

---

### Task 1: FilterAttribute に Limit バリアントを追加

**Files:**
- Modify: `src/features/pod/kube/filter.rs:466-479` (FilterAttribute enum)

**Step 1: FilterAttribute enum に Limit を追加**

`src/features/pod/kube/filter.rs` の `FilterAttribute` enum に `Limit(usize)` バリアントを追加する。

```rust
pub enum FilterAttribute<'a> {
    Pod(Cow<'a, str>),
    ExcludePod(Cow<'a, str>),
    Container(Cow<'a, str>),
    ExcludeContainer(Cow<'a, str>),
    Resource(SpecifiedResource<'a>),
    LabelSelector(Cow<'a, str>),
    FieldSelector(Cow<'a, str>),
    IncludeLog(Cow<'a, str>),
    ExcludeLog(Cow<'a, str>),
    Jq(Cow<'a, str>),
    JMESPath(Cow<'a, str>),
    Limit(usize),  // 追加
}
```

**Step 2: Filter 構造体に limit フィールドを追加**

```rust
pub struct Filter {
    pub pod: Option<Regex>,
    pub exclude_pod: Option<Vec<Regex>>,
    pub container: Option<Regex>,
    pub exclude_container: Option<Vec<Regex>>,
    pub field_selector: Option<String>,
    pub label_selector: Option<LabelSelector>,
    pub include_log: Option<Vec<Regex>>,
    pub exclude_log: Option<Vec<Regex>>,
    pub json_filter: Option<JsonFilter>,
    pub limit: Option<usize>,  // 追加
}
```

**Step 3: Filter::parse() で Limit を処理**

`Filter::parse()` メソッドの match ブロックに `FilterAttribute::Limit` のアームを追加する。

```rust
FilterAttribute::Limit(n) => {
    filter.limit = Some(n);
}
```

**Step 4: ビルド確認**

Run: `cargo check 2>&1 | head -30`
Expected: parser.rs 側の変更がまだなのでパーサーテストでの未使用警告があるかもしれないが、コンパイルは通る

**Step 5: コミット**

```bash
git add src/features/pod/kube/filter.rs
git commit -m "feat: add Limit variant to FilterAttribute and Filter"
```

---

### Task 2: パーサーに limit キーワードを追加

**Files:**
- Modify: `src/features/pod/kube/filter/parser.rs`

**Step 1: limit パーサーのテストを書く**

`parser.rs` の `#[cfg(test)] mod tests` 内に以下を追加:

```rust
#[rstest]
#[case("limit:5000", 5000)]
#[case("lim:5000", 5000)]
#[case("limit:1", 1)]
#[case("limit:100000", 100000)]
fn limit(#[case] query: &str, #[case] expected: usize) {
    let (remaining, actual) = super::limit::<Error<_>>(query).unwrap();

    assert_eq!(actual, FilterAttribute::Limit(expected));
    assert_eq!(remaining, "");
}
```

`attribute` テストの `#[rstest]` にも追加:

```rust
#[case("limit:5000", FilterAttribute::Limit(5000))]
#[case("lim:1000", FilterAttribute::Limit(1000))]
```

`parse_attributes` テストのクエリ配列と期待値にも `"limit:5000"` と `FilterAttribute::Limit(5000)` を追加する。

**Step 2: テストが失敗することを確認**

Run: `cargo test --lib features::pod::kube::filter::parser::tests 2>&1 | tail -20`
Expected: FAIL (limit 関数が存在しない)

**Step 3: limit パーサー関数を実装**

`parser.rs` に `limit` パーサー関数と `positive_integer` ヘルパーを追加する:

```rust
use nom::character::complete::digit1;

fn positive_integer<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, usize, E> {
    let (remaining, digits) = digit1(s)?;
    let n = digits.parse::<usize>().map_err(|_| {
        nom::Err::Error(E::from_error_kind(s, nom::error::ErrorKind::Digit))
    })?;
    Ok((remaining, n))
}

fn limit<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, FilterAttribute<'a>, E> {
    let (remaining, (_, value)) = separated_pair(
        alt((tag("limit"), tag("lim"))),
        char(':'),
        positive_integer,
    )
    .parse(s)?;
    Ok((remaining, FilterAttribute::Limit(value)))
}
```

`attribute` 関数の `alt` に `limit` を追加する（`jq` の前に配置。`l:` で始まるキーワードとの曖昧性を避けるため、`include_log` より前に `limit` を配置し、`limit` / `lim` は完全一致で先にマッチさせる）:

```rust
fn attribute<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    s: &'a str,
) -> IResult<&'a str, FilterAttribute<'a>, E> {
    let (remaining, value) = alt((
        specified_pod,
        specified_daemonset,
        specified_deployment,
        specified_job,
        specified_replicaset,
        specified_service,
        specified_statefulset,
        field_selector,
        label_selector,
        limit,          // include_log より前に配置
        pod,
        exclude_pod,
        container,
        exclude_container,
        include_log,
        exclude_log,
        jmespath,
        jq,
    ))
    .parse(s)?;

    Ok((remaining, value))
}
```

注意: `limit` は `l:` ではなく `limit:` / `lim:` なので `include_log`（`l:`）とは衝突しない。ただし `lim` と `l` の曖昧性を防ぐため、`limit` を先に配置して `limit:` / `lim:` が先にマッチするようにする。nom の `tag("lim")` は `lim:` にマッチするが `l:` にはマッチしないため安全。

**Step 4: テストを実行**

Run: `cargo test --lib features::pod::kube::filter::parser::tests 2>&1 | tail -20`
Expected: PASS

**Step 5: コミット**

```bash
git add src/features/pod/kube/filter/parser.rs
git commit -m "feat: add limit/lim keyword to log query parser"
```

---

### Task 3: Filter::parse() の limit 統合テスト

**Files:**
- Modify: `src/features/pod/kube/filter.rs` (テストセクション)

**Step 1: Filter::parse() で limit が正しく処理されるテストを書く**

`filter.rs` のテストモジュール（末尾付近）に以下を追加:

```rust
#[test]
fn parse_with_limit() {
    let filter = Filter::parse("limit:5000").unwrap();
    assert_eq!(filter.limit, Some(5000));
}

#[test]
fn parse_with_limit_and_other_filters() {
    let filter = Filter::parse("pod:api limit:5000 log:error").unwrap();
    assert_eq!(filter.limit, Some(5000));
    assert!(filter.pod.is_some());
    assert!(filter.include_log.is_some());
}

#[test]
fn parse_without_limit() {
    let filter = Filter::parse("pod:api").unwrap();
    assert_eq!(filter.limit, None);
}
```

**Step 2: テストを実行**

Run: `cargo test --lib features::pod::kube::filter::tests 2>&1 | tail -20`
Expected: PASS

**Step 3: コミット**

```bash
git add src/features/pod/kube/filter.rs
git commit -m "test: add Filter::parse() limit integration tests"
```

---

### Task 4: TextItem を VecDeque に変更

**Files:**
- Modify: `src/ui/widget/text/item.rs`

**Step 1: 既存テストが通ることを確認**

Run: `cargo test --lib ui::widget::text::item::tests 2>&1 | tail -20`
Expected: PASS

**Step 2: Vec を VecDeque に変更**

`item.rs` で以下を変更:

1. `use std::collections::VecDeque;` を追加

2. `TextItem` 構造体:
```rust
pub struct TextItem {
    lines: VecDeque<Line>,
    wrapped_lines: VecDeque<WrappedLine>,
    highlights: Option<Highlights>,
    wrap_width: Option<usize>,
    max_chars: usize,
    highlight_style: SearchHighlightStyle,
}
```

3. `new()` メソッド内で `Vec` → `VecDeque` への変換を追加:
```rust
let wrapped_lines: VecDeque<_> = wrapped_lines.into_iter().flatten().collect();
// lines も VecDeque に:
let lines: VecDeque<_> = lines.into();
```

4. `new_or_extend()` の戻り値は `Vec` のままにし、呼び出し側で `.into()` で変換する。

5. `push()` の `self.lines.push(line)` → `self.lines.push_back(line)`、
   `self.wrapped_lines.extend(wrapped_lines)` はそのまま（VecDeque も extend を持つ）。

6. `extend()` の `self.lines.extend(lines)` はそのまま動作する。

7. `highlight()` 等で `self.lines` にインデックスアクセスしている箇所は VecDeque でも `[]` でアクセス可能なのでそのまま動作する。

8. `self.wrapped_lines[line.wrapped_lines.clone()]` のスライスアクセスは VecDeque ではそのまま動作しないため、`make_contiguous()` を使うか、Range アクセスを `.iter().skip().take()` に変更する必要がある。

  VecDeque のスライスアクセスについて: VecDeque は `Index<usize>` を実装しているが `Index<Range<usize>>` は実装していない。`as_slices()` または `make_contiguous()` を使って連続メモリにする必要がある。

  **方針**: `wrapped_lines` へのスライスアクセスが多く、WrappedLine は行追加時に末尾に追加、削除時に先頭から削除するだけなので、`VecDeque::make_contiguous()` を必要な箇所で呼ぶか、あるいは `wrapped_lines` は `Vec` のままにして先頭削除時は `drain(..n)` を使う方式も検討する。

  **推奨**: `wrapped_lines` は `Vec` のまま維持する。先頭削除は `drain(..n)` で行う。これにより既存のスライスアクセスコードへの影響を最小化する。`lines` のみ `VecDeque` にする。

**修正方針（最終）:**
- `lines: Vec<Line>` → `VecDeque<Line>` に変更
- `wrapped_lines: Vec<WrappedLine>` は `Vec` のまま維持（スライスアクセスの互換性のため）
- `lines` の先頭削除は `pop_front()`、`wrapped_lines` の先頭削除は `drain(..n)`

**Step 3: コンパイルエラーを修正して既存テストを通す**

Run: `cargo test --lib ui::widget::text::item::tests 2>&1 | tail -30`
Expected: PASS

**Step 4: text.rs 側のコンパイルも確認**

Run: `cargo check 2>&1 | head -30`
Expected: コンパイル成功

**Step 5: コミット**

```bash
git add src/ui/widget/text/item.rs
git commit -m "refactor: change TextItem.lines from Vec to VecDeque"
```

---

### Task 5: TextItem に max_lines フィールドとトリム機能を追加

**Files:**
- Modify: `src/ui/widget/text/item.rs`

**Step 1: max_lines のテストを書く**

```rust
mod max_lines {
    use pretty_assertions::assert_eq;
    use super::*;

    #[test]
    fn push_within_limit() {
        let mut item = TextItem::new(vec![], None, SearchHighlightStyle::default());
        item.set_max_lines(Some(3));

        item.push(LiteralItem::new("line1", None));
        item.push(LiteralItem::new("line2", None));
        item.push(LiteralItem::new("line3", None));

        assert_eq!(item.lines.len(), 3);
    }

    #[test]
    fn push_exceeds_limit() {
        let mut item = TextItem::new(vec![], None, SearchHighlightStyle::default());
        item.set_max_lines(Some(2));

        item.push(LiteralItem::new("line1", None));
        item.push(LiteralItem::new("line2", None));
        item.push(LiteralItem::new("line3", None));

        assert_eq!(item.lines.len(), 2);
        // 最も古い "line1" が削除され、"line2" と "line3" が残る
        assert_eq!(item.lines[0].literal_item.item, "line2".to_string());
        assert_eq!(item.lines[1].literal_item.item, "line3".to_string());
    }

    #[test]
    fn no_limit_by_default() {
        let mut item = TextItem::new(vec![], None, SearchHighlightStyle::default());

        for i in 0..100 {
            item.push(LiteralItem::new(format!("line{}", i), None));
        }

        assert_eq!(item.lines.len(), 100);
    }

    #[test]
    fn wrapped_lines_trimmed_correctly() {
        let mut item = TextItem::new(vec![], Some(5), SearchHighlightStyle::default());
        item.set_max_lines(Some(2));

        // 10文字 → wrap_width=5 で 2 wrapped_lines
        item.push(LiteralItem::new("0123456789", None));
        item.push(LiteralItem::new("abcdefghij", None));
        item.push(LiteralItem::new("ABCDEFGHIJ", None));

        assert_eq!(item.lines.len(), 2);
        // "0123456789" の 2 wrapped_lines が削除されている
        assert_eq!(item.wrapped_lines.len(), 4);
    }
}
```

**Step 2: テストが失敗することを確認**

Run: `cargo test --lib ui::widget::text::item::tests::max_lines 2>&1 | tail -20`
Expected: FAIL (`set_max_lines` が存在しない)

**Step 3: max_lines フィールドと set_max_lines/trim_to_limit を実装**

`TextItem` に以下を追加:

```rust
pub struct TextItem {
    lines: VecDeque<Line>,
    wrapped_lines: Vec<WrappedLine>,
    highlights: Option<Highlights>,
    wrap_width: Option<usize>,
    max_chars: usize,
    highlight_style: SearchHighlightStyle,
    max_lines: Option<usize>,  // 追加
}
```

`Default` impl が derive されているので、`max_lines` は `None` がデフォルトになる。

`new()` で `max_lines: None` を初期化に追加。

```rust
pub fn set_max_lines(&mut self, max_lines: Option<usize>) {
    self.max_lines = max_lines;
}

/// max_lines を超過している場合、先頭の行を削除する
/// 削除された wrapped_lines の数を返す
fn trim_to_limit(&mut self) -> usize {
    let Some(max_lines) = self.max_lines else {
        return 0;
    };

    let mut total_wrapped_removed = 0;

    while self.lines.len() > max_lines {
        if let Some(removed_line) = self.lines.pop_front() {
            let wrapped_count = removed_line.wrapped_lines.end - removed_line.wrapped_lines.start;
            self.wrapped_lines.drain(..wrapped_count);
            total_wrapped_removed += wrapped_count;

            // highlights から削除された行のエントリを除去
            if let Some(highlights) = &mut self.highlights {
                highlights.item.retain(|hl| hl.line_index != removed_line.line_index);
            }
        }
    }

    if total_wrapped_removed > 0 {
        // 残りの lines の line_index と wrapped_lines レンジを再計算
        for (i, line) in self.lines.iter_mut().enumerate() {
            line.line_index = i;
            let wrapped_len = line.wrapped_lines.end - line.wrapped_lines.start;
            let new_start = if i == 0 {
                0
            } else {
                self.lines[i - 1].wrapped_lines.end  // ← NG: borrow checker
            };
            // ...
        }
    }

    total_wrapped_removed
}
```

注意: line_index と wrapped_lines の Range は再計算が必要。実装では lines を順にイテレートし、累積で wrapped_lines の開始位置を計算する:

```rust
fn trim_to_limit(&mut self) -> usize {
    let Some(max_lines) = self.max_lines else {
        return 0;
    };

    let mut total_wrapped_removed = 0;

    while self.lines.len() > max_lines {
        if let Some(removed_line) = self.lines.pop_front() {
            let wrapped_count = removed_line.wrapped_lines.end - removed_line.wrapped_lines.start;
            self.wrapped_lines.drain(..wrapped_count);
            total_wrapped_removed += wrapped_count;
        }
    }

    if total_wrapped_removed > 0 {
        // line_index, line_number, wrapped_lines Range を再計算
        let mut wrapped_offset = 0;
        for (i, line) in self.lines.iter_mut().enumerate() {
            line.line_index = i;
            let wrapped_len = line.wrapped_lines.end - line.wrapped_lines.start;
            line.line_number = wrapped_offset;
            line.wrapped_lines = wrapped_offset..(wrapped_offset + wrapped_len);
            wrapped_offset += wrapped_len;
        }

        // highlights の line_index も再計算
        if let Some(highlights) = &mut self.highlights {
            // 削除された行の line_index は既に存在しないので、
            // 新しい line_index にマッピング
            highlights.item.retain_mut(|hl| {
                if let Some(line) = self.lines.iter().find(|l| {
                    // literal_item の内容で照合するのは非効率
                    // 代わりに、元の line_index が削除されたかどうかで判断
                    // → highlights をクリアしてハイライトを再計算するのが安全
                    false
                }) {
                    true
                } else {
                    false
                }
            });

            // 安全策: ハイライトをクリアして再計算
            // highlights は検索中にのみ存在するので、影響は小さい
        }
    }

    total_wrapped_removed
}
```

**実装方針の簡略化:** highlights の整合性維持は複雑になるため、`trim_to_limit()` で行が削除された場合は highlights をクリアする。検索中にログが大量に流れる場合は follow モードのためスクロール位置が末尾になり、ユーザーがアクティブに検索している場面では通常ログの流入が少ない。

最終実装:

```rust
fn trim_to_limit(&mut self) -> usize {
    let Some(max_lines) = self.max_lines else {
        return 0;
    };

    let mut total_wrapped_removed = 0;

    while self.lines.len() > max_lines {
        if let Some(removed_line) = self.lines.pop_front() {
            let wrapped_count = removed_line.wrapped_lines.end - removed_line.wrapped_lines.start;
            self.wrapped_lines.drain(..wrapped_count);
            total_wrapped_removed += wrapped_count;
        }
    }

    if total_wrapped_removed > 0 {
        // line_index, line_number, wrapped_lines Range を再計算
        let mut wrapped_offset = 0;
        for (i, line) in self.lines.iter_mut().enumerate() {
            line.line_index = i;
            line.line_number = wrapped_offset;
            let wrapped_len = line.wrapped_lines.end - line.wrapped_lines.start;
            line.wrapped_lines = wrapped_offset..(wrapped_offset + wrapped_len);
            wrapped_offset += wrapped_len;
        }

        // WrappedLine の line_index も再計算
        for wl in self.wrapped_lines.iter_mut() {
            // wrapped_lines の各エントリの line_index を更新
            // WrappedLine.line_index は対応する Line のインデックス
            // drain で先頭を削除した後、残りの WrappedLine の line_index を
            // 対応する Line の新しい line_index に合わせる
        }
        // WrappedLine の line_index は lines の line_index と連動しているので、
        // lines を走査して各 line の wrapped_lines レンジ内の WrappedLine を更新
        for line in self.lines.iter() {
            for wl in &mut self.wrapped_lines[line.wrapped_lines.clone()] {
                wl.line_index = line.line_index;
            }
        }

        // highlights はクリアする（再計算の複雑さを避けるため）
        if self.highlights.is_some() {
            // clear_highlight で元のスタイルを復元してからクリア
            self.clear_highlight();
        }
    }

    total_wrapped_removed
}
```

`push()` と `extend()` の末尾で `self.trim_to_limit()` を呼ぶ:

```rust
pub fn push(&mut self, item: LiteralItem) {
    // ... 既存のコード ...
    self.lines.push_back(line);
    self.wrapped_lines.extend(wrapped_lines);

    // ハイライト処理 (既存)
    if let Some(highlights) = &mut self.highlights {
        // ...
    }

    self.trim_to_limit();
}

pub fn extend(&mut self, item: Vec<LiteralItem>) {
    // ... 既存のコード ...
    self.trim_to_limit();
}
```

**Step 4: テストを実行**

Run: `cargo test --lib ui::widget::text::item::tests 2>&1 | tail -30`
Expected: PASS

**Step 5: コミット**

```bash
git add src/ui/widget/text/item.rs
git commit -m "feat: add max_lines rolling buffer to TextItem"
```

---

### Task 6: Text ウィジェットにスクロール調整を追加

**Files:**
- Modify: `src/ui/widget/text.rs`

**Step 1: trim_to_limit の戻り値を使ってスクロール調整**

`TextItem::trim_to_limit()` は削除された wrapped_lines の数を返す。しかし、スクロール調整は `Text` ウィジェット側で行う必要がある。

方針: `TextItem::push()` / `extend()` 内で `trim_to_limit()` を呼び、削除された wrapped 行数を `TextItem` に保持するフィールド `last_trimmed_wrapped_count: usize` に記録する。`Text::append_widget_item()` 呼出し後にこの値を読み取ってスクロール位置を調整する。

`item.rs` に追加:

```rust
pub struct TextItem {
    // ... 既存フィールド ...
    max_lines: Option<usize>,
    last_trimmed_wrapped_count: usize,
}
```

```rust
pub fn take_trimmed_wrapped_count(&mut self) -> usize {
    std::mem::take(&mut self.last_trimmed_wrapped_count)
}
```

`trim_to_limit()` 内で:
```rust
self.last_trimmed_wrapped_count = total_wrapped_removed;
```

`text.rs` の `append_widget_item()` を修正:

```rust
fn append_widget_item(&mut self, item: Item) {
    let is_bottom = self.is_bottom();

    match item {
        Item::Single(i) => self.item.push(i),
        Item::Array(i) => self.item.extend(i),
        _ => {
            unreachable!()
        }
    }

    // トリムされた行数分スクロール位置を調整
    let trimmed = self.item.take_trimmed_wrapped_count();
    if trimmed > 0 {
        self.scroll.y = self.scroll.y.saturating_sub(trimmed);
    }

    if self.should_follow() && is_bottom {
        self.select_last()
    }
}
```

**Step 2: コンパイル確認**

Run: `cargo check 2>&1 | head -30`
Expected: コンパイル成功

**Step 3: コミット**

```bash
git add src/ui/widget/text/item.rs src/ui/widget/text.rs
git commit -m "feat: adjust scroll position when lines are trimmed"
```

---

### Task 7: Config に logging.max_lines を追加

**Files:**
- Modify: `src/config.rs`

**Step 1: LoggingConfig を追加**

```rust
#[derive(Default, Debug, Deserialize, Serialize)]
pub struct LoggingConfig {
    pub max_lines: Option<usize>,
}

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct Config {
    pub theme: ThemeConfig,
    pub logging: LoggingConfig,
}
```

**Step 2: コンパイル確認**

Run: `cargo check 2>&1 | head -30`
Expected: コンパイル成功

**Step 3: コミット**

```bash
git add src/config.rs
git commit -m "feat: add logging.max_lines to Config"
```

---

### Task 8: max_lines 設定をログウィジェットに伝達

**Files:**
- Modify: `src/features/pod/view/widgets/log.rs` (log_widget 関数)
- Modify: `src/features/pod/view/tab.rs` (PodTab)
- Modify: `src/ui/widget/text.rs` (TextBuilder)
- 必要に応じて他のファイルも修正

**Step 1: TextBuilder に max_lines メソッドを追加**

`text.rs` の `TextBuilder`:

```rust
pub struct TextBuilder {
    // ... 既存フィールド ...
    max_lines: Option<usize>,
}

impl TextBuilder {
    pub fn max_lines(mut self, max_lines: Option<usize>) -> Self {
        self.max_lines = max_lines;
        self
    }

    pub fn build(self) -> Text {
        let mut item = TextItem::new(self.item, None, self.theme.search.clone());
        item.set_max_lines(self.max_lines);
        Text {
            // ...
            item,
            // ...
        }
    }
}
```

**Step 2: log_widget に max_lines 引数を追加**

`log.rs`:
```rust
pub fn log_widget(
    tx: &Sender<Message>,
    clipboard: &Option<Rc<RefCell<Clipboard>>>,
    theme: WidgetThemeConfig,
    max_lines: Option<usize>,
) -> Widget<'static> {
    // ...
    let builder = Text::builder()
        // ... 既存の設定 ...
        .max_lines(max_lines);
    // ...
}
```

**Step 3: PodTab で Config から max_lines を渡す**

Config の `logging.max_lines` を PodTab 経由で `log_widget()` に渡す。
`tab.rs` の `PodTab::new()` 引数に `max_lines: Option<usize>` を追加し、`log_widget(tx, clipboard, theme, max_lines)` を呼ぶ。

上位の呼び出し元 (`app.rs` や `workers/render/` 等) から `Config.logging.max_lines` を渡す。

**Step 4: ログクエリでの limit 値の伝達**

`LogMessage::Request(LogConfig)` で limit が渡される。`LogConfig` に `limit: Option<usize>` フィールドを追加し、`Filter::parse()` で得た `filter.limit` を `LogConfig` に設定する。

render/action.rs で `LogMessage::Response` を処理する前に、limit 値を TextItem に反映する仕組みを検討する。

方針: `LogMessage` に新しいバリアント `SetMaxLines(Option<usize>)` を追加するか、`LogMessage::Request` 処理時に render 側でも max_lines を更新する。

より簡単な方法: `LogConfig` に limit を持たせ、KubeWorker が LogMessage::Request を受けた時に、limit 値を render 側にも通知する。具体的には `LogMessage::Response` の前に一度だけ `LogMessage::SetMaxLines(limit)` を送信する。

これは具体的な実装時に最適な経路を選択する。基本方針は「Config の max_lines をデフォルトとし、ログクエリの limit で上書き」。

**Step 5: コンパイル確認**

Run: `cargo check 2>&1 | head -30`
Expected: コンパイル成功

**Step 6: コミット**

```bash
git add -A
git commit -m "feat: wire max_lines config through to log widget"
```

---

### Task 9: ログクエリの limit 値を TextItem に伝達

**Files:**
- Modify: `src/features/pod/kube/log.rs` (LogConfig, LogWorker)
- Modify: `src/features/pod/message.rs` (LogMessage)
- Modify: `src/workers/render/action.rs`
- Modify: `src/workers/kube/controller.rs`

**Step 1: LogMessage に SetMaxLines バリアントを追加**

`message.rs`:
```rust
pub enum LogMessage {
    Request(LogConfig),
    Response(Result<Vec<String>>),
    ToggleJsonPrettyPrint,
    SetMaxLines(Option<usize>),  // 追加
}
```

**Step 2: LogWorker::spawn() で limit を送信**

`log.rs` の `LogWorker::spawn()` 内（ログストリーミング開始前）で、Filter から取得した limit 値を送信する:

```rust
// Filter::parse() の後
let limit = filter.limit;

// SetMaxLines メッセージを送信
self.tx.send(LogMessage::SetMaxLines(limit).into())
    .expect("Failed to send LogMessage::SetMaxLines");
```

**Step 3: render/action.rs で SetMaxLines を処理**

```rust
Kube::Log(LogMessage::SetMaxLines(max_lines)) => {
    let widget = window.find_widget_mut(POD_LOG_WIDGET_ID);
    widget.set_max_lines(max_lines);
}
```

`Text` ウィジェットに `set_max_lines()` メソッドを追加:
```rust
impl Text {
    pub fn set_max_lines(&mut self, max_lines: Option<usize>) {
        self.item.set_max_lines(max_lines);
    }
}
```

**Step 4: WidgetTrait に set_max_lines を追加（必要に応じて）**

`find_widget_mut` が返す型に応じて、トレイトに `set_max_lines` を追加するか、ダウンキャストで対応する。既存のパターン（`append_widget_item` 等）に合わせる。

**Step 5: コンパイル確認**

Run: `cargo check 2>&1 | head -30`
Expected: コンパイル成功

**Step 6: コミット**

```bash
git add -A
git commit -m "feat: send limit from log query to TextItem via SetMaxLines message"
```

---

### Task 10: ログクエリヘルプに limit を追加

**Files:**
- Modify: `src/features/pod/view/widgets/log_query_help.rs`

**Step 1: ヘルプテキストに limit の説明を追加**

既存のヘルプウィジェットの items リストに `limit:N` / `lim:N` の説明を追加する:

```
"limit:<N>     Log buffer size limit (alias: lim)"
```

**Step 2: コミット**

```bash
git add src/features/pod/view/widgets/log_query_help.rs
git commit -m "docs: add limit keyword to log query help"
```

---

### Task 11: 全体テストと動作確認

**Step 1: 全ユニットテストを実行**

Run: `cargo test --lib 2>&1 | tail -30`
Expected: PASS

**Step 2: コンパイル警告の確認**

Run: `cargo clippy 2>&1 | tail -30`
Expected: 新しい警告なし（既存の警告は除く）

**Step 3: 必要に応じて修正してコミット**

```bash
git add -A
git commit -m "fix: address clippy warnings and test failures"
```
