# Log query: multi-namespace partial-success Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** ログクエリ `LabelSelector::Resource` (例: `deployment/app`) で複数 ns 選択時に一部 ns でリソース不在 (404 等) でも他の ns のログ取得を継続し、不在 ns は yellow `[kubetui]` prefix の通知行でユーザーに伝える。

**Architecture:** `LogMessage` に `Notice { namespace, message }` variant を追加。renderer (`action.rs`) が新 arm で yellow ANSI escape を付けて log view にインライン append。producer (`log.rs` の per-namespace setup loop) は retrieve エラー時に通知を送って `continue`、その ns の PodWatcher 構築をスキップ。

**Tech Stack:** Rust 2021, `tokio` async, `crossbeam` channel, `ratatui`。

**Spec:** `docs/superpowers/specs/2026-06-03-log-query-multi-namespace-notice-design.md`

---

## File Structure

### Modified files (no new files)

- `src/features/pod/message.rs` — `LogMessage::Notice { namespace, message }` variant 追加
- `src/workers/render/action.rs` — `Kube::Log(LogMessage::Notice { .. })` arm 追加
- `src/features/pod/kube/log.rs` — per-namespace setup loop の retrieve エラーハンドリングを `?` → match に変更、Notice 送信 + `continue`

## Task ordering rationale

`action.rs` の match は `_ => unreachable!()` で締めているため、新 variant が来ると runtime panic する。順序:

1. **Task 1: variant 追加** — 警告だけ (unused variant)、runtime 影響なし
2. **Task 2: renderer arm 追加** — variant をハンドル可能に。まだ誰も送らないので動作変化なし
3. **Task 3: sender 追加** — Notice が流れ始める。renderer が安全に処理

各 commit が独立して runtime 安全。

---

## Pre-flight

- [ ] 作業ブランチ `fix/log-query-multi-namespace-skip` に居て HEAD が spec commit (`7a3c5920`)

```bash
git branch --show-current  # → fix/log-query-multi-namespace-skip
git log --oneline -1       # → 7a3c5920 docs: spec for log query partial-success ...
```

---

## Task 1: `LogMessage::Notice` variant 追加

**Files:**
- Modify: `src/features/pod/message.rs`

- [ ] **Step 1: Add `Notice` variant**

Read `src/features/pod/message.rs` to locate the `LogMessage` enum. Add the new variant at the end (after `StreamError`):

Current:
```rust
#[derive(Debug)]
pub enum LogMessage {
    Request(LogConfig),
    Response(Result<Vec<String>>),
    ToggleJsonPrettyPrint,
    SetMaxLines(Option<usize>),
    StreamError(String),
}
```

Change to:
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
    Notice { namespace: String, message: String },
}
```

- [ ] **Step 2: Verify build**

```bash
cargo build 2>&1 | rg "error|warning: " | rg -v "kubeconfig|Nested" | head -10
```

Expected: 0 errors. One new warning expected: `variant 'Notice' is never constructed` (resolved by Task 3).

Note: the `action.rs` match uses `_ => unreachable!()` as a catch-all, so the build does NOT fail on non-exhaustive matching. The runtime panic on receiving an actual `Notice` is prevented by ordering Task 2 (renderer) before Task 3 (sender).

- [ ] **Step 3: Test**

```bash
cargo test --all 2>&1 | rg "test result:" | tail -3
```

Expected: all tests pass (no new tests; existing must continue to pass).

- [ ] **Step 4: Commit**

```bash
git add src/features/pod/message.rs
git commit -m "feat(log-msg): add LogMessage::Notice variant for per-namespace notices"
```

---

## Task 2: Renderer arm in action.rs

**Files:**
- Modify: `src/workers/render/action.rs:228-237` (existing `StreamError` arm area)

- [ ] **Step 1: Add `Notice` arm next to `StreamError`**

Read the file and locate:

```rust
Kube::Log(LogMessage::StreamError(msg)) => {
    // ストリーム継続中のエラー: ログにインライン追記（エラー状態はクリアしない）
    let widget = window.find_widget_mut(POD_LOG_WIDGET_ID);
    let item = LiteralItem {
        metadata: None,
        item: format!("\x1b[31m[kubetui] {}\x1b[39m", msg),
    };
    widget.append_widget_item(Item::Array(vec![item]));
}
```

Add the new arm immediately after this block (before the next `Kube::Log(...)` arm or any other match arm):

```rust
Kube::Log(LogMessage::Notice { namespace, message }) => {
    // セットアップ時の非致命エラー: ログにインライン追記（widget の error 状態は触らない）
    let widget = window.find_widget_mut(POD_LOG_WIDGET_ID);
    let item = LiteralItem {
        metadata: None,
        item: format!("\x1b[33m[kubetui] {}: {}\x1b[39m", namespace, message),
    };
    widget.append_widget_item(Item::Array(vec![item]));
}
```

- [ ] **Step 2: Build**

```bash
cargo build 2>&1 | rg "error|warning: " | rg -v "kubeconfig|Nested|variant.*Notice" | head -10
```

Expected: 0 errors. The `variant 'Notice' is never constructed` warning from Task 1 should still appear (resolved by Task 3).

- [ ] **Step 3: Test + fmt**

```bash
cargo test --all 2>&1 | rg "test result:" | tail -3
cargo +nightly fmt
cargo +nightly fmt --check 2>&1 | head -3
```

Expected: all tests pass, fmt clean.

- [ ] **Step 4: Commit**

```bash
git add src/workers/render/action.rs
git commit -m "feat(log-view): render LogMessage::Notice as yellow inline notice"
```

---

## Task 3: Producer side — log.rs sender

**Files:**
- Modify: `src/features/pod/kube/log.rs:75-117` (the `for namespace in namespaces` loop)

- [ ] **Step 1: Replace the `?` with match + Notice send + continue**

Read the file. Locate the per-namespace setup loop in `LogWorker::collect`. Current shape:

```rust
for namespace in namespaces {
    // retrieve label selector
    let label_selector = if let Some(value) = &filter.label_selector {
        let retrieve_label_selector =
            RetrieveLabelSelector::new(&self.client, &namespace, value);

        Some(retrieve_label_selector.retrieve().await?)
    } else {
        None
    };

    let pod_watcher = PodWatcher::new(
        self.tx.clone(),
        // ... rest unchanged
```

Replace the `if let Some(value)` block with:

```rust
for namespace in namespaces {
    // retrieve label selector
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
                    return Ok(LogHandle::new(Vec::new()));
                }
                continue;
            }
        }
    } else {
        None
    };

    let pod_watcher = PodWatcher::new(
        self.tx.clone(),
        // ... rest unchanged
```

The rest of the loop body (PodWatcher construction, `pod_watchers.push(pod_watcher)`) and the loop's `}` stay exactly as before. The subsequent collector spawn + `Ok(LogHandle::new(handles))` at the function's end are also unchanged.

Note: `LogHandle::new(Vec::new())` is valid — verified that `LogHandle::new` accepts `Vec<JoinHandle<()>>` and an empty Vec is acceptable. The early return on `tx.send` failure matches the existing setup-time pattern (`LogMessage::SetMaxLines` send site in the same file).

- [ ] **Step 2: Build**

```bash
cargo build 2>&1 | rg "error|warning: " | rg -v "kubeconfig|Nested" | head -10
```

Expected: 0 errors, no new warnings (the `Notice` "never constructed" warning from Tasks 1-2 should now be gone — it's constructed here).

- [ ] **Step 3: Test + fmt**

```bash
cargo test --all 2>&1 | rg "test result:" | tail -3
cargo +nightly fmt
cargo +nightly fmt --check 2>&1 | head -3
```

Expected: all tests pass, fmt clean.

- [ ] **Step 4: Commit**

```bash
git add src/features/pod/kube/log.rs
git commit -m "fix(log-query): skip + notify on per-namespace retrieve failure

When LabelSelector::Resource is used and multiple namespaces are selected,
a missing resource in any namespace previously caused the whole log query
to fail via the ? propagation. Now the retrieve error is captured as a
LogMessage::Notice (per-namespace, non-fatal) and the loop continues with
the remaining namespaces."
```

---

## Task 4: Final verification + PR

- [ ] **Step 1: Run all gates**

```bash
cargo build 2>&1 | rg "error|warning: " | rg -v "kubeconfig|Nested" | head -5
cargo test --all 2>&1 | tail -3
cargo clippy --all-targets 2>&1 | rg "^warning" | head -10
cargo +nightly fmt --check 2>&1 | head -3
```

Expected:
- build: clean (only pre-existing `try_from_kubeconfig` warning)
- test: all green (no new tests added; existing count preserved)
- clippy: no new warning categories
- fmt: clean

- [ ] **Step 2: Push and create PR**

```bash
git push -u origin fix/log-query-multi-namespace-skip
gh pr create --title "fix(log-query): skip + notify on per-namespace retrieve failure" --body "$(cat <<'EOF'
## Summary

Pod log query で `LabelSelector::Resource` (例: `deployment/app`, `daemonsets/app`) 指定時、複数 namespace 選択モードで一部 ns に対象リソースが存在しない (404 等) と `?` 演算子経由でループが中断、全 ns のセットアップが失敗、ログビューがエラー表示になる問題を修正。

修正後は不在 ns を skip + 通知し、存在する ns のログ取得を継続する。通知は yellow `[kubetui]` prefix で log view にインライン表示。

## What changed

- `LogMessage::Notice { namespace, message }` variant を新規追加
- `src/workers/render/action.rs` で Notice を `\x1b[33m[kubetui] <ns>: <message>\x1b[39m` 形式で log view に append
- `src/features/pod/kube/log.rs` の per-namespace setup loop で retrieve エラーを skip + Notice 送信

## Why a new variant

`StreamError` は stream 中の異常 (赤、severity 高) 用。retrieve 失敗 (黄、severity 低) は意味的に別で、ユーザーに違うシグナルを与えたい。型レベルで区別。

## Test plan

- [x] `cargo build`: clean
- [x] `cargo test --all`: all pass
- [x] `cargo clippy --all-targets`: no new warning categories
- [x] `cargo +nightly fmt --check`: clean
- [ ] Manual GKE smoke:
  - [ ] 複数 ns + partial existence (`deployment/<name>` を 3 ns で実行、1 ns のみ存在) → 残り 2 ns の通知 + 存在 ns のログが共存
  - [ ] 全 ns 404 (タイポクエリ) → 通知のみが並ぶ
  - [ ] 単一 ns 404 → 通知 1 行のみ、ログ空 (許容)
  - [ ] 正常ケース (全 ns 存在) → 通知無し、ログのみ (回帰なし)

## Related

- Spec: `docs/superpowers/specs/2026-06-03-log-query-multi-namespace-notice-design.md`
- Plan: `docs/superpowers/plans/2026-06-03-log-query-multi-namespace-notice.md`
- 将来検討 (別 PR): ANSI 色の theme 設定可能化、StreamError との対称扱い
EOF
)"
```

- [ ] **Step 3: Manual GKE verification**

実機で以下を確認:

1. **複数 ns + partial existence**: `deployment/<name>` を 3 ns で実行、1 ns のみ存在 → 残り 2 ns の通知 + 存在 ns のログ
2. **全 ns 404**: タイポクエリで通知のみ並ぶ
3. **単一 ns 404**: 通知 1 行、ログ空
4. **正常回帰**: 全 ns に存在するクエリ → 通知無し、ログのみ

- [ ] **Step 4: Update PR test plan after manual smoke passes**

`gh pr edit <pr> --body ...` で manual smoke のチェック boxes を埋める。

---

## Notes

- 単体テストは追加しない (`kube::Api::get` の mock が型パラメータ + HTTP で複雑、manual smoke で代替) — spec §5 通り。
- Task 順序は: variant → renderer arm → sender。`action.rs` の `_ => unreachable!()` catch-all 由来の runtime 安全性を維持するため。
- `LogHandle::new` は `Vec<JoinHandle<()>>` を受け付け、empty Vec も OK (型確認済み)。
