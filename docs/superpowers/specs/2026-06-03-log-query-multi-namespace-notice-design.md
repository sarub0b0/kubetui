# Log query: skip & notify on per-namespace retrieve failure

- 日付: 2026-06-03
- ステータス: Proposed
- 対象範囲: Pod log query で `LabelSelector::Resource` (例: `deployment/app`, `daemonsets/app`) 指定時、複数 namespace 選択モードで一部 ns にリソースが存在しない場合の挙動を「skip + ビュー通知」に修正する。新しい `LogMessage::Notice { namespace, message }` variant を追加し、yellow `[kubetui]` prefix で log view にインライン表示する。
- 対象外: ANSI 色の theme 設定可能化 (将来 PR)、`StreamError` の挙動変更、log filter 構文の拡張、その他のログ機能拡張。

## 背景・動機

`src/features/pod/kube/log.rs` の per-namespace setup loop で、`retrieve_label_selector.retrieve().await?` の `?` により retrieve エラーが即座に loop を中断・上位に伝播する。具体的には:

```rust
for namespace in namespaces {
    let label_selector = if let Some(value) = &filter.label_selector {
        let retrieve_label_selector =
            RetrieveLabelSelector::new(&self.client, &namespace, value);

        Some(retrieve_label_selector.retrieve().await?)  // ← 404 で即 return
    } else { None };

    let pod_watcher = PodWatcher::new(...);
    // ...
}
```

複数 ns 選択時、対象リソースが**一部 ns には存在し他には無い**ケース (典型: `deployment/datadog` が `latest` には存在、`infra` には無い等) で:

1. 不在 ns で `api.get(name).await` が 404 を返す
2. `?` で loop を中断、setup 全体が失敗
3. `LogMessage::Response(Err(...))` がビューに伝達
4. ログビューが**エラー表示で固まり**、存在する ns のログも見られない

期待されるのは「ある ns では取れる、無い ns はその旨だけ伝えて取れる ns のログを表示」という partial-success の UX。

## 現状確認

### 既存の仕組み

- `LogMessage` enum (`src/features/pod/message.rs`) には `Response(Result<...>)`, `StreamError(String)`, `SetMaxLines`, `ToggleJsonPrettyPrint`, `Request` の variant が存在
- `StreamError` は ANSI 赤色 `\x1b[31m[kubetui] {msg}\x1b[39m` で `POD_LOG_WIDGET_ID` widget にインライン追記 (`src/workers/render/action.rs:228`)。stream 継続中のエラー用で error state は触らない
- per-namespace setup は `src/features/pod/kube/log.rs::collect` の `for namespace in namespaces` ループ内
- retrieve 失敗時の error は `anyhow::Error` で wrap されて上位に伝播

### この仕組みが満たしていないニーズ

- retrieve 段階のエラー (404/403/network 等) は **setup-time エラー**で、`StreamError` (stream 中エラー) とは性質が違うが、現状はどちらも `Response(Err(...))` で「ビュー全体がエラー」になる
- 複数 ns 選択時の partial-success サポートが無い

## ゴール

1. 複数 ns 選択時、retrieve 失敗を skip + 通知のみ。他 ns の PodWatcher セットアップは継続する。
2. ユーザーに「どの ns で何が失敗したか」を log view 内で visible にする (ログ表示行と同列にインライン表示)。
3. 一部 ns で取れる場合は「取れた ns のログ」+「取れなかった ns の通知行」が共存する。
4. 全 ns で retrieve 失敗の場合、通知行が並ぶことで「対象がどこにも存在しない」が自然に伝わる (専用エラー UI 不要)。
5. retrieve 失敗を error 種別で分岐しない (404/403/network 等すべて skip + 通知)。エラー文字列に種別情報は含まれるのでユーザーは判別可能。
6. 既存の `StreamError` (stream 継続中エラー、severity 高/赤) の挙動・表示は維持する。

## 非ゴール

- ANSI 色の theme 設定可能化 (本 spec では yellow をハードコード)。
- `StreamError` の挙動変更や色変更。
- 単一 ns 選択時の挙動変更 (単一 ns 内で全 ns 通知のみ → ログ空、は仕様として許容)。
- log filter 構文の拡張、jq/jmespath との統合、新 highlight 等。
- 通知の重複排除や同種エラーのグルーピング (現状 1 ns あたり 1 行で素朴に出す)。

## 設計

### 1. `LogMessage::Notice` variant 追加

`src/features/pod/message.rs`:

```rust
#[derive(Debug)]
pub enum LogMessage {
    Request(LogConfig),
    Response(Result<Vec<String>>),
    ToggleJsonPrettyPrint,
    SetMaxLines(Option<usize>),
    StreamError(String),
    /// Non-fatal informational notice tied to a namespace. Used to surface
    /// per-namespace setup-time failures (e.g. resource not found) without
    /// failing the whole log query when multiple namespaces are selected.
    /// Rendered as a yellow inline line in the log view with `[kubetui]`
    /// prefix.
    Notice {
        namespace: String,
        message: String,
    },
}
```

#### 設計判断

- **`StreamError` とは別 variant にする**: stream 中異常 (赤、severity 高) と retrieve 時 skip 通知 (黄、severity 低) は user に違うシグナルを与える必要があり、型レベルで区別する。
- **構造化フィールド `{ namespace, message }`**: namespace を別フィールドにすることで renderer 側で format 自由 (将来 `[<ns>]` 強調や色変えが容易)。`message` は producer 側で format した自由文字列。
- **`kind` enum は持たせない**: YAGNI。現状 1 用途 (retrieve 失敗) のみ。将来ケースが増えたら enum 化を検討。

### 2. Producer 側 (log.rs)

`src/features/pod/kube/log.rs` の per-namespace setup loop:

```rust
for namespace in namespaces {
    let label_selector = if let Some(value) = &filter.label_selector {
        let retrieve_label_selector =
            RetrieveLabelSelector::new(&self.client, &namespace, value);

        match retrieve_label_selector.retrieve().await {
            Ok(sel) => Some(sel),
            Err(e) => {
                let notice = LogMessage::Notice {
                    namespace: namespace.clone(),
                    message: format!(
                        "failed to retrieve label selector for {}: {}",
                        value, e
                    ),
                };
                if let Err(send_err) = self.tx.send(notice.into()) {
                    logger!(error, "Failed to send LogMessage::Notice: {}", send_err);
                    return;
                }
                continue;
            }
        }
    } else {
        None
    };

    let pod_watcher = PodWatcher::new(/* ... 既存 ... */);
    // ... 既存の watcher 設定 ...
    pod_watchers.push(pod_watcher);
}
```

#### 設計判断

- **エラー種別で分岐しない**: 404/403/network/parse すべて同じ skip + 通知扱い。`anyhow::Error` の Display は root cause を含むのでユーザーは内容を識別可能。`Result<Option<String>>` 化や `is_not_found` 判定の特別ルートは避ける。
- **`tx.send` 失敗時は `logger!(error, ...) + return`**: 既存の setup-time send pattern (`LogMessage::SetMaxLines` 送信箇所、`src/features/pod/kube/log.rs`) と一致。tx 切断時は後続の watcher も死ぬので早期 return が妥当。
- **メッセージ format `"failed to retrieve label selector for {value}: {error}"`**: value (`deployment/app` 等) と error 両方を含めることで「どのクエリの何が失敗したか」を一目で伝える。`LabelSelector::Resource` は既に `Display` 実装あり (`src/features/pod/kube/filter.rs:447,450` 参照)。
- **`continue` で次の ns へ**: その ns の PodWatcher 構築はスキップ。watcher 配列の重複/穴開きは発生しない (元々 push されないだけ)。

### 3. Renderer 側 (action.rs)

`src/workers/render/action.rs` の `Kube::Log(...)` match に新 arm 追加 (既存 `StreamError` arm の直後に配置、関連性で近接させる):

```rust
Kube::Log(LogMessage::StreamError(msg)) => {
    // 既存: \x1b[31m (red)
    let widget = window.find_widget_mut(POD_LOG_WIDGET_ID);
    let item = LiteralItem {
        metadata: None,
        item: format!("\x1b[31m[kubetui] {}\x1b[39m", msg),
    };
    widget.append_widget_item(Item::Array(vec![item]));
}

Kube::Log(LogMessage::Notice { namespace, message }) => {
    let widget = window.find_widget_mut(POD_LOG_WIDGET_ID);
    let item = LiteralItem {
        metadata: None,
        item: format!("\x1b[33m[kubetui] {}: {}\x1b[39m", namespace, message),
    };
    widget.append_widget_item(Item::Array(vec![item]));
}
```

#### 設計判断

- **`\x1b[33m` = ANSI yellow + `\x1b[39m` = reset to default**: 既存 `StreamError` の赤と同じ ANSI escape スタイル踏襲。
- **`[kubetui] <namespace>: <message>`**: namespace を冒頭に独立で出すことで「どの ns の通知か」を一目で識別。
- **`set_widget_error` を呼ばない**: Notice は実行継続中の事象なので widget 全体のエラー状態は触らない。`append_widget_item` で行追加のみ。`StreamError` と同方針。
- **`Item::Array(vec![item])`**: 既存 StreamError と同じ append 機構を再利用。新 widget API 不要。

### 4. 視覚的な表示順序

Notice は send されたタイミングで widget に append される。実際の出力例 (3 ns 選択時、`deployment/datadog` が latest のみ存在):

```
[kubetui] infra: failed to retrieve label selector for deployment/datadog: NotFound (...)
[kubetui] kube-system: failed to retrieve label selector for deployment/datadog: NotFound (...)
2026-06-03T10:00:00.000Z  datadog-5j97l/agent  some log line
2026-06-03T10:00:01.000Z  datadog-cluster-agent-...  another log line
```

通知行は setup-time に出るので、ログ stream が始まる前にまとめて並ぶ。視覚的には「通知ヘッダ → 実ログ」の順序になり自然に読める。

### 5. テスト方針

#### Unit テスト

- `LogMessage::Notice` の derive(Debug) は自動生成、明示テスト不要。
- producer 側の `retrieve` 失敗 → Notice 送信ロジックは `kube::Api::get` の mock が複雑 (型パラメータ + HTTP) なため unit テスト見送り。manual smoke で代替。
- 既存 LogMessage match site (`src/workers/render/action.rs`) が新 variant の追加で網羅性違反になるかは compile 時に検出される (`_ =>` パターンの有無で `match` の網羅性が変わるため要確認)。

#### 実機検証 (GKE smoke)

以下のケースを実機で確認:

1. **複数 ns、partial existence**: 例 `deployment/datadog` を 3 ns 選択して実行、1 ns にのみ存在 → 残り 2 ns の通知行 + 存在 ns のログが共存
2. **全 ns で 404**: タイポしたクエリで全 ns 通知のみ表示 → ユーザーが「どこにも無い」と気付ける
3. **単一 ns で 404**: 通知 1 行のみ表示、ログ空 → 許容仕様
4. **正常ケース回帰**: 全 ns に存在するクエリ → 通知無し、ログのみ (既存挙動維持)
5. **StreamError との共存**: Notice 表示後に container が起動失敗 → 黄通知 (setup) と赤エラー (stream) が並んで識別可

## リスク / 後方互換

- **既存 LogMessage consumer**: `match` の `_ =>` パターンの有無で挙動が変わる。`src/workers/render/action.rs` で網羅性を確認し、必要なら明示 arm 追加。
- **挙動互換**: 単一 ns + 404 のケースで「現状エラー UI 表示 → Notice 表示 + ログ空」に変化する。ユーザーから見ると「red error から yellow notice に変わった」だけで意味的には同じ (リソースが無い旨の伝達)。許容。
- **チャネル輻輳**: 大量 ns 選択で全部 404 だと通知が ns 数だけ流れる。N=100 でも 100 行 = 軽量。問題なし。
- **format 文字列のローカライズ**: 現状 ANSI escape + 英語固定。i18n は本 spec 外 (kubetui 全体に i18n 機構が無い)。

## 将来検討

- **ANSI 色の theme 設定可能化**: 現状 `StreamError` の赤 / `Notice` の黄は両方ハードコード。`theme.pod.log.{stream_error,notice}.fg_color` 等の schema で設定可にすると、ユーザーが好みの色で表示できる。両 variant 同時の対象として独立 spec で扱う方が一貫性ある (片方だけ themed は対称性を崩す)。`action.rs` は現状 theme 未参照のため、配線追加が要る。Config #1002 で `WidgetThemeConfig` を tab 経由で配線したパターンが参考になる。
- **`LogMessage::Notice` の `kind` 拡張**: 用途が増えたら `kind: LogNoticeKind` enum を導入。例: `RetrievalFailed { resource, error }`, `WatcherReconnected { reason }`, `StreamPaused`。今は 1 用途のみで YAGNI。
- **通知のグルーピング/重複排除**: 同種通知が大量出ても素朴に並べる。問題化したら time-windowed dedup 等を検討。
- **filter autosuggest との交差**: ログクエリ入力に autosuggest を入れる将来構想 (memory `filter-autosuggest-future.md`) では Pod ↔ resource/name 補完が議論対象になる。本 Notice はその文脈にも関連 (誤入力時の feedback として)。
